use super::super::{App, CaptureMode};
use eframe::egui::{self, RichText};
use zsr::i18n::tr;
impl App {
    /// Present large, clearly selected capture choices before source/audio controls.
    pub(in crate::app) fn capture_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.heading(tr(lang, "capture"));
        ui.horizontal_wrapped(|ui| {
            for (mode, label) in [
                (CaptureMode::Display, "display"),
                (CaptureMode::Region, "region"),
                (CaptureMode::Window, "window"),
            ] {
                let selected = self.capture_mode == mode;
                if ui
                    .add(
                        egui::Button::new(RichText::new(tr(lang, label)).strong())
                            .selected(selected)
                            .min_size(egui::vec2(112.0, 40.0)),
                    )
                    .clicked()
                {
                    self.capture_mode = mode;
                    if self.capture_mode == CaptureMode::Region {
                        self.open_region_picker();
                    }
                }
            }
        });
        ui.small(tr(
            lang,
            if self.active() {
                "capture_locked_hint"
            } else {
                "capture_choose_hint"
            },
        ));
        if self.capture_mode == CaptureMode::Window {
            self.window_controls(ui);
        } else {
            ui.label(tr(lang, "source"));
            let selected = self
                .capabilities
                .as_ref()
                .and_then(|c| c.sources.get(self.display))
                .map(|s| s.label.as_str())
                .unwrap_or(tr(lang, "no_display"));
            egui::ComboBox::from_id_salt("display")
                .selected_text(selected)
                .width(320.0)
                .show_ui(ui, |ui| {
                    if let Some(cap) = &self.capabilities {
                        for (index, source) in cap.sources.iter().enumerate() {
                            ui.selectable_value(&mut self.display, index, &source.label);
                        }
                    }
                });
            if ui
                .add_enabled(!self.checking, egui::Button::new(tr(lang, "refresh")))
                .clicked()
            {
                self.probe();
            }
            if self.capture_mode == CaptureMode::Region {
                if ui.button(tr(lang, "region_select")).clicked() {
                    self.open_region_picker();
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut self.region.x).range(0..=32768));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut self.region.y).range(0..=32768));
                    ui.label(tr(lang, "width"));
                    ui.add(egui::DragValue::new(&mut self.region.width).range(2..=32768));
                    ui.label(tr(lang, "height"));
                    ui.add(egui::DragValue::new(&mut self.region.height).range(2..=32768));
                });
                ui.small(tr(lang, "region_hint"));
            }
        }
        ui.separator();
        self.audio_controls(ui);
    }
}
