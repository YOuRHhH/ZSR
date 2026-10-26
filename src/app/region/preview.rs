use super::super::App;
use super::PickerWindow;
use eframe::egui;
use zsr::{platform::Display, recording::Region};
impl App {
    /// Render the overlay alone in the primary framebuffer for visual QA;
    /// eframe's immediate secondary viewports do not deliver screenshot events.
    #[cfg(feature = "ui-smoke")]
    pub(in crate::app) fn region_preview(&mut self, ctx: &egui::Context) -> bool {
        if self.smoke.is_none() || std::env::var_os("ZSR_UI_REGION_PICKER").is_none() {
            return false;
        }
        let picker = self.picker.get_or_insert_with(PickerWindow::default);
        let display = Display {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            id: 0,
        };
        let _ = picker.state.lock().expect("picker state").show(
            ctx,
            display,
            Region {
                x: 240,
                y: 180,
                width: 1280,
                height: 720,
            },
            self.settings.language,
        );
        true
    }
}
