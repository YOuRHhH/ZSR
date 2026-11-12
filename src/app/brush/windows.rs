//! A native layered overlay owns its window and input loop on a dedicated thread.
use super::Canvas;
use eframe::egui;
use std::{
    ffi::c_void,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use zsr::brush::Ink;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Event {
    kind: i32,
    x: i32,
    y: i32,
}
#[repr(C)]
struct Stroke {
    offset: u32,
    count: u32,
    width: f32,
    color: u32,
}
unsafe extern "C" {
    fn zsr_brush_create() -> *mut c_void;
    fn zsr_brush_present(
        window: *mut c_void,
        rect: *const i32,
        enabled: i32,
        strokes: *const Stroke,
        count: u32,
        points: *const f32,
    ) -> i32;
    fn zsr_brush_mode(window: *mut c_void, enabled: i32);
    fn zsr_brush_tools(window: *mut c_void, title: *const u16);
    fn zsr_brush_poll(window: *mut c_void, events: *mut Event) -> i32;
    fn zsr_brush_destroy(window: *mut c_void);
}
struct Window(*mut c_void);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            zsr_brush_destroy(self.0);
        }
    }
}

pub(super) struct Overlay {
    stop: Arc<AtomicBool>,
    errors: mpsc::Receiver<String>,
}
impl Overlay {
    pub(super) fn start(canvas: Arc<Mutex<Canvas>>, ctx: egui::Context) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let (tx, errors) = mpsc::channel();
        std::thread::Builder::new()
            .name("recording-brush".into())
            .spawn(move || {
                if let Err(error) = run(&canvas, &worker_stop, &ctx) {
                    let _ = tx.send(error);
                    ctx.request_repaint();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { stop, errors })
    }
    pub(super) fn error(&self) -> Option<String> {
        self.errors.try_recv().ok()
    }
}
impl Drop for Overlay {
    fn drop(&mut self) {
        // Do not join from the main UI: destroying a focused native window can send
        // messages to that UI. The worker retains and frees all resources after this flag.
        self.stop.store(true, Ordering::Release);
    }
}

fn run(shared: &Mutex<Canvas>, stop: &AtomicBool, ctx: &egui::Context) -> Result<(), String> {
    let window = Window(unsafe { zsr_brush_create() });
    if window.0.is_null() {
        return Err("Could not create the native drawing overlay".into());
    }
    let mut last = None;
    let mut events = [Event::default(); 256];
    while !stop.load(Ordering::Acquire) {
        // Window messages and focus changes run with no Rust canvas/egui locks held.
        let count = unsafe { zsr_brush_poll(window.0, events.as_mut_ptr()) } as usize;
        let (rect, enabled, ink, changed, tools_title) = {
            let mut canvas = shared.lock().unwrap();
            if !canvas.open {
                break;
            }
            let area = canvas
                .native_area
                .expect("open native canvas has capture bounds");
            for event in &events[..count] {
                let point = [
                    (event.x as f32 / area.width as f32).clamp(0.0, 1.0),
                    (event.y as f32 / area.height as f32).clamp(0.0, 1.0),
                ];
                match event.kind {
                    1 if canvas.enabled => canvas.begin_stroke(point, area),
                    2 if canvas.enabled => canvas.extend_stroke(point, area),
                    3 if canvas.enabled => {
                        canvas.extend_stroke(point, area);
                        canvas.drawing = false;
                    }
                    4 => canvas.disable(),
                    5 if canvas.enabled => {
                        canvas.ink.strokes.pop();
                        canvas.drawing = false;
                        canvas.changed();
                    }
                    6 if canvas.enabled => {
                        canvas.ink.strokes.clear();
                        canvas.drawing = false;
                        canvas.changed();
                    }
                    7 => canvas.drawing = false,
                    _ => {}
                }
            }
            canvas.take_restore(); // Native mode changes restore the previous foreground window.
            canvas.focus = false;
            let rect = [area.x, area.y, area.width, area.height];
            let key = (rect, canvas.enabled);
            let repaint = std::mem::take(&mut canvas.repaint) || last != Some(key);
            last = Some(key);
            (
                rect,
                canvas.enabled,
                repaint.then(|| canvas.ink.clone()),
                count > 0,
                canvas.tools_title.clone(),
            )
        };
        if let Some(ink) = ink {
            present(&window, rect, enabled, &ink)?;
        }
        unsafe {
            zsr_brush_mode(window.0, i32::from(enabled));
            if !tools_title.is_empty() {
                zsr_brush_tools(window.0, tools_title.as_ptr());
            }
        }
        if changed {
            ctx.request_repaint();
        }
        // Coalesce mouse moves at display cadence instead of uploading a full DIB per event.
        std::thread::sleep(Duration::from_millis(16));
    }
    Ok(())
}

fn present(window: &Window, rect: [i32; 4], enabled: bool, ink: &Ink) -> Result<(), String> {
    let mut points = Vec::<[f32; 2]>::new();
    let mut strokes = Vec::new();
    for stroke in &ink.strokes {
        // Expand shapes with the same pixel geometry used by X11 and the video compositor.
        for path in stroke.paths(rect[2] as f32, rect[3] as f32) {
            strokes.push(Stroke {
                offset: points.len() as u32,
                count: path.len() as u32,
                width: stroke.width * rect[2].min(rect[3]) as f32,
                color: u32::from_le_bytes([stroke.color[0], stroke.color[1], stroke.color[2], 0]),
            });
            points.extend_from_slice(&path);
        }
    }
    // C borrows these bounded arrays only for this synchronous bitmap update.
    if unsafe {
        zsr_brush_present(
            window.0,
            rect.as_ptr(),
            i32::from(enabled),
            strokes.as_ptr(),
            strokes.len() as u32,
            points.as_ptr().cast(),
        )
    } == 0
    {
        return Err("Could not present the native drawing overlay".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn layered_window_preserves_alpha_and_escape_releases_input() {
        unsafe extern "C" {
            fn zsr_brush_test() -> i32;
        }
        // Keep test window ownership/DPI changes on an isolated thread, as in production.
        let result = std::thread::spawn(|| unsafe { zsr_brush_test() })
            .join()
            .unwrap();
        assert_eq!(result, 0, "native brush regression flags: {result}");
    }
}
