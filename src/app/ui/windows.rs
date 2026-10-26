use crate::app::App;
use eframe::egui;
use zsr::i18n::tr;
impl App {
    pub(super) fn window_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        if !zsr::windows::supported() {
            ui.label(tr(lang, "window_unsupported"));
            return;
        }
        ui.label(tr(lang, "window_source"));
        let selected = self
            .windows
            .selected(self.selected_window, &self.settings.ffmpeg)
            .map(|w| w.label())
            .unwrap_or_else(|| tr(lang, "window_choose").into());
        egui::ComboBox::from_id_salt("application_window")
            .selected_text(selected)
            .width(320.0)
            .show_ui(ui, |ui| {
                for window in &self.windows.windows {
                    ui.selectable_value(
                        &mut self.selected_window,
                        Some(window.identity()),
                        window.label(),
                    );
                }
            });
        if ui
            .add_enabled(
                !self.windows.loading(),
                egui::Button::new(tr(lang, "refresh")),
            )
            .clicked()
        {
            self.windows.refresh(&self.settings.ffmpeg);
        }
        if self.windows.loading() {
            ui.spinner();
        }
        if let Some(error) = &self.windows.error {
            ui.label(error);
        } else if self.windows.windows.is_empty() && !self.windows.loading() {
            ui.label(tr(lang, "window_empty"));
        } else if self.selected_window.is_some()
            && self
                .windows
                .selected(self.selected_window, &self.settings.ffmpeg)
                .is_none()
        {
            ui.label(tr(lang, "window_missing"));
        }
        ui.small(tr(lang, "window_hint"));
    }
}
