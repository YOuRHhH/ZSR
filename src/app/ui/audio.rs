use super::super::App;
use eframe::egui::{self, Color32};
use zsr::audio;
use zsr::i18n::tr;
impl App {
    /// Select capture sources and MP4 track routing before a recording begins.
    pub(in crate::app) fn audio_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.heading(tr(lang, "audio"));
        ui.checkbox(&mut self.settings.desktop_audio, tr(lang, "desktop_audio"));
        ui.checkbox(&mut self.settings.microphone, tr(lang, "microphone"));
        ui.label(tr(lang, "audio_device"));
        let selected = audio::selected(self.audio.devices(), &self.settings.audio_device)
            .map(|device| device.name.as_str())
            .unwrap_or(tr(
                lang,
                if self.settings.audio_device.is_empty() {
                    "audio_none"
                } else {
                    "audio_missing"
                },
            ));
        ui.add_enabled_ui(
            self.settings.microphone && !self.audio.is_refreshing(),
            |ui| {
                egui::ComboBox::from_id_salt("audio_device")
                    .selected_text(selected)
                    .width(320.0)
                    .show_ui(ui, |ui| {
                        for (index, device) in self.audio.devices().iter().enumerate() {
                            let duplicate = self
                                .audio
                                .devices()
                                .iter()
                                .filter(|other| other.name == device.name)
                                .count()
                                > 1;
                            let label = if duplicate {
                                format!("{} ({})", device.name, index + 1)
                            } else {
                                device.name.clone()
                            };
                            ui.selectable_value(
                                &mut self.settings.audio_device,
                                device.id.clone(),
                                label,
                            );
                        }
                    });
            },
        );
        if ui
            .add_enabled(
                !self.audio.is_refreshing(),
                egui::Button::new(tr(lang, "audio_refresh")),
            )
            .clicked()
        {
            self.audio.refresh(&self.settings.obs_runtime);
        }
        if self.audio.is_refreshing() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(lang, "audio_loading"));
            });
        } else if self.settings.microphone && !self.audio_ready() {
            ui.colored_label(
                Color32::YELLOW,
                tr(
                    lang,
                    if self.audio.error().is_some() {
                        "audio_error"
                    } else {
                        "audio_missing"
                    },
                ),
            );
        }
        if let Some(error) = self.audio.error() {
            ui.collapsing(tr(lang, "details"), |ui| {
                ui.small(tr(lang, "diagnostics"));
                ui.label(error);
            });
        }
        ui.small(tr(lang, "audio_hint"));
        ui.label(tr(lang, "audio_tracks"));
        egui::ComboBox::from_id_salt("audio_tracks")
            .selected_text(tr(
                lang,
                match self.settings.audio_tracks {
                    zsr::settings::AudioTracks::Mixed => "audio_mixed",
                    zsr::settings::AudioTracks::Separate => "audio_separate",
                },
            ))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.settings.audio_tracks,
                    zsr::settings::AudioTracks::Mixed,
                    tr(lang, "audio_mixed"),
                );
                ui.selectable_value(
                    &mut self.settings.audio_tracks,
                    zsr::settings::AudioTracks::Separate,
                    tr(lang, "audio_separate"),
                );
            });
        ui.small(tr(lang, "audio_tracks_hint"));
    }
}
