use super::{Outcome, lifecycle};
use crate::app::App;
use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use zsr::{i18n::tr, platform::Display, recording::Region, settings::Language};

#[derive(Default)]
pub(in crate::app) struct PickerWindow {
    rx: Option<Receiver<Result<Outcome, String>>>,
    minimized: bool,
    #[cfg(feature = "ui-smoke")]
    pub(super) state: std::sync::Mutex<super::Picker>,
}

fn pick(display: Display, initial: Region, language: Language) -> Result<Outcome, String> {
    unsafe extern "C" {
        fn zsr_pick_region(
            display: *const Display,
            rect: *mut i32,
            instructions: *const u16,
            confirm: *const u16,
            cancel: *const u16,
        ) -> i32;
    }
    let wide = |key| {
        tr(language, key)
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let instructions = wide("region_instructions");
    let confirm = wide("region_confirm");
    let cancel = wide("region_cancel");
    let mut rect = [initial.x, initial.y, initial.width, initial.height];
    // SAFETY: C owns the native window/event loop on this worker. Borrowed
    // display, four coordinates, and terminated UTF-16 strings live until return.
    let result = unsafe {
        zsr_pick_region(
            &display,
            rect.as_mut_ptr(),
            instructions.as_ptr(),
            confirm.as_ptr(),
            cancel.as_ptr(),
        )
    };
    match result {
        1 => Ok(Outcome::Confirm(Region {
            x: rect[0],
            y: rect[1],
            width: rect[2],
            height: rect[3],
        })),
        2 => Ok(Outcome::Cancel),
        _ => Err(tr(language, "region_failed").into()),
    }
}

impl App {
    pub(in crate::app) fn open_region_picker(&mut self) {
        if !self.active()
            && !self.checking
            && self.picker.is_none()
            && self
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
        // Let the main event loop apply minimize before opening the native picker.
        if !window.minimized {
            lifecycle::minimize_main(ctx);
            window.minimized = true;
            ctx.request_repaint();
            return;
        }
        if window.rx.is_none() {
            let Some(source) = self
                .capabilities
                .as_ref()
                .and_then(|c| c.sources.get(self.display))
            else {
                self.picker = None;
                lifecycle::restore_main(ctx);
                return;
            };
            let (tx, rx) = mpsc::channel();
            window.rx = Some(rx);
            let display = source.display;
            let initial = self.region;
            let language = self.settings.language;
            let wake = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("region-picker".into())
                .spawn(move || {
                    let _ = tx.send(pick(display, initial, language));
                    wake.request_repaint();
                });
            if let Err(error) = spawned {
                self.detail = format!("{}: {error}", tr(language, "region_failed"));
                self.picker = None;
                lifecycle::restore_main(ctx);
            }
            return;
        }
        let outcome = match window.rx.as_ref().expect("started picker").try_recv() {
            Ok(outcome) => outcome,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(tr(self.settings.language, "region_failed").into())
            }
        };
        match outcome {
            Ok(Outcome::Confirm(region)) => {
                self.region = region;
                self.region_marker.confirmed = true;
            }
            Ok(Outcome::Cancel) => {}
            Err(error) => self.detail = error,
        }
        self.picker = None;
        lifecycle::restore_main(ctx);
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_picker_pixels_input_and_frame_cost() {
        unsafe extern "C" {
            fn zsr_picker_verify(micros: *mut f64) -> i32;
        }
        let mut micros = 0.0;
        // SAFETY: the native self-test uses a private offscreen DIB, never desktop capture.
        assert_eq!(unsafe { zsr_picker_verify(&mut micros) }, 0);
        eprintln!(
            "Native 1920x1080 picker draw: {micros:.0} us/frame (100 frames; no compositor timing)"
        );
    }
}
