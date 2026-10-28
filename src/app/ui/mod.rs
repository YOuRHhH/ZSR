mod audio;
mod capture;
mod notices;
mod output;
mod recording;
mod recordings;
mod settings;
mod windows;
use super::App;
use eframe::egui::{self, Color32, RichText};
use zsr::i18n::tr;
impl App {
    /// Render recording essentials, the local-storage notice and optional preferences.
    pub(super) fn render(&mut self, ctx: &egui::Context) {
        if self.custom_title_bar {
            self.title_bar(ctx);
        }
        let lang = self.settings.language;
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new("ZSR")
                        .size(28.0)
                        .color(Color32::from_rgb(120, 185, 250)),
                );
                ui.label(tr(lang, "tagline"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(lang, "settings")).clicked() {
                        self.settings_open = true;
                    }
                    if ui.button(tr(lang, "recordings")).clicked() {
                        self.recordings.open = true;
                        self.recordings.refresh();
                    }
                });
            });
            ui.add_space(8.0);
        });
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            // Keep the storage notice without a development-stage label in shipped builds.
            ui.horizontal_wrapped(|ui| {
                ui.small(tr(lang, "privacy"));
            });
        });
        // Keep Start/Stop and the recording indicator visible while settings scroll.
        egui::TopBottomPanel::top("recording_status").show(ctx, |ui| {
            ui.add_space(8.0);
            self.recording_controls(ui);
            ui.add_space(8.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(12.0);
                self.notices(ui);
                ui.add_space(14.0);
                ui.add_enabled_ui(!self.active() && self.picker.is_none(), |ui| {
                    if ui.available_width() >= 760.0 {
                        ui.columns(2, |cols| {
                            egui::Frame::group(cols[0].style()).show(&mut cols[0], |ui| {
                                self.capture_controls(ui);
                            });
                            egui::Frame::group(cols[1].style()).show(&mut cols[1], |ui| {
                                self.output_controls(ui);
                            });
                        });
                    } else {
                        self.capture_controls(ui);
                        ui.separator();
                        self.output_controls(ui);
                    }
                });
            });
        });
        self.settings_window(ctx);
        self.recordings_window(ctx);
    }
}
