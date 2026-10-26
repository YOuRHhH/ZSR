use super::super::App;
use eframe::egui;
use zsr::i18n::tr;

impl App {
    /// Show completed recordings in the current save folder with lightweight file actions.
    pub(super) fn recordings_window(&mut self, ctx: &egui::Context) {
        if !self.recordings.open {
            return;
        }
        let lang = self.settings.language;
        let mut open = true;
        egui::Window::new(tr(lang, "recordings"))
            .id(egui::Id::new("recordings_window"))
            .open(&mut open)
            .default_width(580.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.label(self.settings.destination.to_string_lossy().as_ref());
                ui.small(tr(lang, "recordings_hint"));
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.recordings.loading(),
                            egui::Button::new(tr(lang, "recordings_refresh")),
                        )
                        .clicked()
                    {
                        self.recordings.refresh();
                    }
                    if ui
                        .add_enabled(
                            !self.file_opener.busy(),
                            egui::Button::new(tr(lang, "open_folder")),
                        )
                        .clicked()
                    {
                        self.file_opener
                            .open(self.settings.destination.clone(), ui.ctx());
                    }
                    if self.recordings.loading() {
                        ui.spinner();
                    }
                });
                self.file_open_status(ui);
                if let Some(error) = &self.recordings.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                if self.recordings.files.is_empty() && !self.recordings.loading() {
                    ui.label(tr(lang, "recordings_empty"));
                }
                egui::ScrollArea::vertical()
                    .id_salt("recordings_list")
                    .show(ui, |ui| {
                        for file in &self.recordings.files {
                            ui.separator();
                            ui.label(file.path.file_name().unwrap_or_default().to_string_lossy())
                                .on_hover_text(file.path.to_string_lossy());
                            let date: chrono::DateTime<chrono::Local> = file.modified.into();
                            ui.horizontal_wrapped(|ui| {
                                ui.small(format!(
                                    "{} · {:.2} MiB",
                                    date.format("%Y-%m-%d %H:%M"),
                                    file.bytes as f64 / 1_048_576.0
                                ));
                                if ui
                                    .add_enabled(
                                        !self.file_opener.busy(),
                                        egui::Button::new(tr(lang, "open_recording")),
                                    )
                                    .clicked()
                                {
                                    self.file_opener.open(file.path.clone(), ui.ctx());
                                }
                            });
                        }
                    });
            });
        self.recordings.open = open;
    }
}
