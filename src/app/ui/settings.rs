use super::super::App;
use eframe::egui;
use zsr::{i18n::tr, settings::Language};

impl App {
    /// Show less frequent preferences without taking recording controls off the main screen.
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let lang = self.settings.language;
        let mut open = true;
        egui::Window::new(tr(lang, "settings"))
            .id(egui::Id::new("preferences_window"))
            .open(&mut open)
            .collapsible(false)
            .default_pos(ctx.content_rect().center() - egui::vec2(240.0, 300.0))
            .default_width(480.0)
            .default_height(600.0)
            .vscroll(true)
            .show(ctx, |ui| {
                if self.active() {
                    ui.label(tr(lang, "settings_locked"));
                }
                ui.add_enabled_ui(!self.active() && self.picker.is_none(), |ui| {
                    ui.heading(tr(lang, "general"));
                    ui.horizontal(|ui| {
                        ui.label(tr(lang, "language"));
                        egui::ComboBox::from_id_salt("language")
                            .selected_text(match lang {
                                Language::English => "English",
                                Language::Chinese => "简体中文",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.language,
                                    Language::English,
                                    "English",
                                );
                                ui.selectable_value(
                                    &mut self.settings.language,
                                    Language::Chinese,
                                    "简体中文",
                                );
                            });
                    });
                    ui.checkbox(&mut self.settings.cursor, tr(lang, "cursor"));
                    ui.separator();
                    ui.heading(tr(lang, "output"));
                    self.output_preferences(ui);
                    self.shortcut_controls(ui);
                    ui.separator();
                    ui.collapsing(tr(lang, "advanced"), |ui| {
                        self.backend_preferences(ui);
                    });
                    ui.add_space(8.0);
                    ui.small(tr(lang, "settings_saved_hint"));
                });
            });
        self.settings_open = open;
        if !open || self.active() || self.picker.is_some() {
            self.shortcuts.cancel_capture();
        }
    }
}
