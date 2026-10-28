use super::super::{App, CaptureMode, State};
use eframe::egui::{self, Color32, RichText, Stroke};
use zsr::i18n::tr;

/// Only the backend's Recording state receives a red live indicator.
fn status_style(state: &State) -> (&'static str, &'static str, Color32, Color32) {
    match state {
        State::Idle => (
            "ready",
            "record_ready_hint",
            Color32::from_rgb(110, 220, 255),
            Color32::from_rgb(8, 20, 30),
        ),
        State::Preparing => (
            "preparing",
            "record_preparing_hint",
            Color32::from_rgb(255, 205, 95),
            Color32::from_rgb(35, 26, 8),
        ),
        State::Recording => (
            "recording",
            "record_live_hint",
            Color32::from_rgb(255, 105, 105),
            Color32::from_rgb(38, 10, 15),
        ),
        State::Stopping => (
            "stopping",
            "record_saving_hint",
            Color32::from_rgb(255, 205, 95),
            Color32::from_rgb(35, 26, 8),
        ),
        State::Saved => (
            "saved",
            "record_saved_hint",
            Color32::from_rgb(110, 235, 170),
            Color32::from_rgb(8, 28, 20),
        ),
        State::Failed => (
            "failed",
            "record_failed_hint",
            Color32::from_rgb(255, 145, 125),
            Color32::from_rgb(38, 16, 10),
        ),
    }
}

impl App {
    /// Keep backend-confirmed recording status, target and primary action easy to identify.
    pub(in crate::app) fn recording_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let (status, hint, accent, background) = status_style(&self.state);
        egui::Frame::group(ui.style())
            .fill(background)
            .stroke(Stroke::new(1.5_f32, accent))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    if matches!(self.state, State::Preparing | State::Stopping) {
                        ui.add(egui::Spinner::new().color(accent));
                    } else {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 6.0, accent);
                    }
                    ui.label(
                        RichText::new(tr(lang, status))
                            .strong()
                            .size(22.0)
                            .color(accent),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{:02}:{:02}:{:02}",
                            self.seconds as u64 / 3600,
                            self.seconds as u64 / 60 % 60,
                            self.seconds as u64 % 60
                        ))
                        .monospace()
                        .strong()
                        .size(32.0),
                    );
                    ui.small(format!("{}  {}", tr(lang, "frames"), self.frames));
                });
                ui.label(tr(lang, hint));
                ui.label(format!(
                    "{}: {}",
                    tr(lang, "capture_target"),
                    self.capture_summary()
                ));
                ui.add_space(4.0);
                if self.active() {
                    if ui
                        .add_enabled(
                            self.state != State::Stopping,
                            egui::Button::new(
                                RichText::new(format!("■  {}", tr(lang, "stop")))
                                    .strong()
                                    .size(18.0),
                            )
                            .min_size(egui::vec2(190.0, 44.0))
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
                            egui::Button::new(
                                RichText::new(format!("●  {}", tr(lang, "record")))
                                    .strong()
                                    .size(18.0),
                            )
                            .min_size(egui::vec2(190.0, 44.0))
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

    /// Repeat the selected target near Start/Stop even when its controls are scrolled away.
    fn capture_summary(&self) -> String {
        let lang = self.settings.language;
        let display = self
            .capabilities
            .as_ref()
            .and_then(|c| c.sources.get(self.display))
            .map(|source| source.label.as_str())
            .unwrap_or(tr(lang, "no_display"));
        match self.capture_mode {
            CaptureMode::Display => format!("{} · {display}", tr(lang, "display")),
            CaptureMode::Region => format!(
                "{} · {display} · {} × {} px (X {}, Y {})",
                tr(lang, "region"),
                self.region.width,
                self.region.height,
                self.region.x,
                self.region.y
            ),
            CaptureMode::Window => format!(
                "{} · {}",
                tr(lang, "window"),
                self.windows
                    .selected(self.selected_window, &self.settings.obs_runtime)
                    .map(|window| window.label())
                    .unwrap_or_else(|| tr(lang, "window_choose").into())
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_status_is_reserved_for_backend_confirmed_recording() {
        assert_eq!(status_style(&State::Preparing).1, "record_preparing_hint");
        assert_eq!(status_style(&State::Recording).1, "record_live_hint");
        assert_eq!(status_style(&State::Stopping).1, "record_saving_hint");
        assert_ne!(
            status_style(&State::Preparing).2,
            status_style(&State::Recording).2
        );
        assert_ne!(
            status_style(&State::Stopping).2,
            status_style(&State::Recording).2
        );
    }
}
