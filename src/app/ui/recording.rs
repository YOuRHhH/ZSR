use super::super::{App, CaptureMode, State};
use eframe::egui::{self, Color32};
use zsr::i18n::tr;
impl App {
    /// Render recording actions and dispatch file launches without blocking the UI.
    pub(in crate::app) fn recording_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let status = match self.state {
            State::Idle => "ready",
            State::Preparing => "preparing",
            State::Recording => "recording",
            State::Stopping => "stopping",
            State::Saved => "saved",
            State::Failed => "failed",
        };
        egui::Frame::group(ui.style())
            .inner_margin(20.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.heading(tr(lang, status));
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{}  {:02}:{:02}:{:02}",
                        tr(lang, "elapsed"),
                        self.seconds as u64 / 3600,
                        self.seconds as u64 / 60 % 60,
                        self.seconds as u64 % 60
                    ));
                    ui.label(format!("{}  {}", tr(lang, "frames"), self.frames));
                });
                if self.active() {
                    if ui
                        .add_enabled(
                            self.state != State::Stopping,
                            egui::Button::new(tr(lang, "stop"))
                                .fill(Color32::from_rgb(115, 20, 30)),
                        )
                        .clicked()
                        && let Some(session) = &self.session
                    {
                        session.stop();
                        self.state = State::Stopping;
                    }
                } else {
                    let ready = self.picker.is_none()
                        && (self.capture_mode != CaptureMode::Window
                            || self
                                .windows
                                .selected(self.selected_window, &self.settings.obs_runtime)
                                .is_some())
                        && self.audio_ready()
                        && !self.checking
                        && self.probed_runtime == self.settings.obs_runtime
                        && self.capabilities.as_ref().is_some_and(|c| {
                            !c.sources.is_empty() && c.error.is_none() && !c.wayland
                        });
                    if ui
                        .add_enabled(
                            ready,
                            egui::Button::new(tr(lang, "record"))
                                .fill(Color32::from_rgb(0, 65, 115)),
                        )
                        .clicked()
                    {
                        self.start();
                    }
                }
                if let Some(path) = self.saved.clone() {
                    ui.label(format!("{}: {}", tr(lang, "saved_path"), path.display()));
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.file_opener.busy(),
                                egui::Button::new(tr(lang, "open_recording")),
                            )
                            .clicked()
                        {
                            self.file_opener.open(path.clone(), ui.ctx());
                        }
                        if ui
                            .add_enabled(
                                !self.file_opener.busy(),
                                egui::Button::new(tr(lang, "open_folder")),
                            )
                            .clicked()
                            && let Some(parent) = path.parent()
                        {
                            self.file_opener.open(parent.to_owned(), ui.ctx());
                        }
                    });
                }
            });
    }
}
