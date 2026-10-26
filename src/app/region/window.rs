use super::super::App;
use super::{FRAME_INTERVAL, Outcome, Picker, lifecycle, placement};
use eframe::egui;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};

/// The independent viewport owns drawing/input state. Only completed selections
/// cross back to the app, so dragging never waits for the minimized main window.
pub(in crate::app) struct PickerWindow {
    pub(super) state: Arc<Mutex<Picker>>,
    tx: Sender<Outcome>,
    rx: Receiver<Outcome>,
    minimized: bool,
}

impl Default for PickerWindow {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            state: Arc::new(Mutex::new(Picker::default())),
            tx,
            rx,
            minimized: false,
        }
    }
}

impl App {
    pub(in crate::app) fn open_region_picker(&mut self) {
        if self.active() || self.checking {
            return;
        }
        if self
            .capabilities
            .as_ref()
            .and_then(|c| c.sources.get(self.display))
            .is_some()
        {
            self.picker = Some(PickerWindow::default());
        }
    }

    pub(in crate::app) fn region_picker(&mut self, ctx: &egui::Context) {
        let Some(window) = &mut self.picker else {
            return;
        };
        if let Ok(outcome) = window.rx.try_recv() {
            if let Outcome::Confirm(region) = outcome {
                self.region = region;
                self.region_marker.confirmed = true;
            }
            self.picker = None;
            lifecycle::restore_main(ctx);
            ctx.request_repaint();
            return;
        }
        let Some(source) = self
            .capabilities
            .as_ref()
            .and_then(|c| c.sources.get(self.display))
        else {
            self.picker = None;
            lifecycle::restore_main(ctx);
            return;
        };
        let display = source.display;
        let initial = self.region;
        let language = self.settings.language;
        let title = format!("ZSR_REGION_PICKER_{}", std::process::id());
        let scale = ctx.pixels_per_point();
        let builder = egui::ViewportBuilder::default()
            .with_title(&title)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(!cfg!(windows))
            .with_always_on_top()
            .with_has_shadow(false)
            .with_taskbar(false)
            .with_position([display.x as f32 / scale, display.y as f32 / scale])
            .with_inner_size([display.width as f32 / scale, display.height as f32 / scale]);
        let state = Arc::clone(&window.state);
        let tx = window.tx.clone();
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("region_picker"),
            builder,
            move |ctx, class| {
                let mut picker = state.lock().expect("picker state");
                if picker.finished {
                    return;
                }
                let outcome = if class == egui::ViewportClass::Embedded {
                    Some(Outcome::Cancel)
                } else {
                    #[cfg(target_os = "macos")]
                    placement::adjust_macos(ctx, display);
                    if !picker.placed {
                        picker.placed = placement::position(&title, display);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    picker.show(ctx, display, initial, language)
                };
                if let Some(outcome) = outcome {
                    picker.finished = true;
                    let _ = tx.send(outcome);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    ctx.request_repaint_of(egui::ViewportId::ROOT);
                } else if picker.drag.is_some() {
                    ctx.request_repaint_after(FRAME_INTERVAL);
                }
                // Idle updates are input-driven, avoiding full-screen polling.
            },
        );
        if !window.minimized {
            lifecycle::minimize_main(ctx);
            window.minimized = true;
        }
    }
}
