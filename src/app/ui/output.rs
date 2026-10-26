use super::super::App;
use super::super::Background;
use eframe::egui::{self};
use zsr::i18n::tr;
impl App {
    /// Render resolution, frame rate, and the current save destination.
    pub(in crate::app) fn output_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.heading(tr(lang, "output"));
        ui.horizontal(|ui| {
            ui.label(tr(lang, "resolution"));
            egui::ComboBox::from_id_salt("output_resolution")
                .selected_text(tr(lang, self.settings.resolution.label_key()))
                .show_ui(ui, |ui| {
                    for resolution in zsr::resolution::Resolution::ALL {
                        ui.selectable_value(
                            &mut self.settings.resolution,
                            resolution,
                            tr(lang, resolution.label_key()),
                        );
                    }
                });
        });
        ui.small(tr(lang, "resolution_hint"));
        ui.horizontal(|ui| {
            ui.label(tr(lang, "fps"));
            egui::ComboBox::from_id_salt("fps")
                .selected_text(format!("{} FPS", self.settings.fps))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.fps, 30, "30 FPS");
                    ui.selectable_value(&mut self.settings.fps, 60, "60 FPS");
                });
        });
        ui.add_space(12.0);
        ui.label(tr(lang, "folder"));
        ui.label(self.settings.destination.to_string_lossy().as_ref());
        if ui.button(tr(lang, "more_settings")).clicked() {
            self.settings_open = true;
        }
    }

    /// Edit compression and destination preferences in the settings window.
    pub(super) fn output_preferences(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.horizontal(|ui| {
            ui.label(tr(lang, "quality"));
            let label = match self.settings.crf {
                18 => "high",
                28 => "compact",
                _ => "standard",
            };
            egui::ComboBox::from_id_salt("quality")
                .selected_text(tr(lang, label))
                .show_ui(ui, |ui| {
                    for (crf, key) in [(18, "high"), (23, "standard"), (28, "compact")] {
                        ui.selectable_value(&mut self.settings.crf, crf, tr(lang, key));
                    }
                });
        });
        if cfg!(windows) {
            ui.checkbox(
                &mut self.settings.hardware_encoding,
                tr(lang, "hardware_encoding"),
            );
            ui.small(tr(lang, "hardware_encoding_hint"));
        }
        ui.label(tr(lang, "folder"));
        let mut path = self.settings.destination.to_string_lossy().into_owned();
        if ui.text_edit_singleline(&mut path).changed() {
            self.settings.destination = path.into();
        }
        if ui.button(tr(lang, "browse")).clicked() {
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    let _ = tx.send(Background::Folder(path));
                }
            });
        }
    }

    /// Configure and recheck the recording backend from advanced settings.
    pub(super) fn backend_preferences(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.label(tr(lang, "obs_runtime"));
        ui.text_edit_singleline(&mut self.settings.obs_runtime);
        ui.small(tr(lang, "obs_runtime_hint"));
        if ui
            .add_enabled(!self.checking, egui::Button::new(tr(lang, "check")))
            .clicked()
        {
            self.probe();
        }
    }
}
