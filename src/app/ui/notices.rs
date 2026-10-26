use super::super::{App, State};
use eframe::egui::{self, Color32};
use zsr::{i18n::tr, settings::Language};
impl App {
    /// Keep shell-launch progress and failures visible without affecting recording state.
    pub(super) fn file_open_status(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        if self.file_opener.busy() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(lang, "opening_file"));
            });
        }
        if let Some(error) = &self.file_opener.error {
            ui.colored_label(
                Color32::LIGHT_RED,
                format!("{}: {error}", tr(lang, "open_failed")),
            );
            if ui.button(tr(lang, "retry")).clicked() {
                self.file_opener.error = None;
            }
        }
    }

    /// Show setup, recording, and background file-launch status.
    pub(in crate::app) fn notices(&mut self, ui: &mut egui::Ui) {
        self.file_open_status(ui);
        let lang = self.settings.language;
        if self.checking {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(lang, "initializing"));
            });
        }
        if let Some(cap) = &self.capabilities {
            if cap.wayland {
                ui.colored_label(Color32::YELLOW, tr(lang, "wayland"));
            }
            if let Some(error) = &cap.error {
                ui.colored_label(Color32::YELLOW, tr(lang, "capability_error"));
                ui.label(error);
            }
        }
        if self.font_missing && lang == Language::Chinese {
            ui.colored_label(Color32::YELLOW, tr(lang, "font_missing"));
        }
        if let Some(error) = &self.settings_error {
            ui.label(format!("{}: {error}", tr(lang, "settings_error")));
        }
        if !self.detail.is_empty() {
            ui.collapsing(tr(lang, "details"), |ui| {
                ui.small(tr(lang, "diagnostics"));
                ui.label(&self.detail);
            });
            if !self.active() && ui.button(tr(lang, "retry")).clicked() {
                self.detail.clear();
                if self.state == State::Failed {
                    self.state = State::Idle;
                }
            }
        }
    }
}
