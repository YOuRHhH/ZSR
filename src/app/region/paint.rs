use super::{Outcome, Picker};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind};
use zsr::{i18n::tr, platform::Display, settings::Language};

impl Picker {
    pub(super) fn paint(&self, ctx: &egui::Context, bounds: Rect) {
        let painter = ctx.layer_painter(egui::LayerId::background());
        // Windows uses this renderer only for the optional smoke preview.
        if !cfg!(windows) {
            painter.rect_filled(bounds, 0.0, Color32::from_black_alpha(1));
        }
        // Keep desktop content clearly visible outside the selection as well.
        let tint = Color32::from_black_alpha(32);
        if let Some(rect) = self.selection {
            if !cfg!(windows) {
                for area in [
                    Rect::from_min_max(bounds.min, Pos2::new(bounds.right(), rect.top())),
                    Rect::from_min_max(Pos2::new(bounds.left(), rect.bottom()), bounds.max),
                    Rect::from_min_max(Pos2::new(bounds.left(), rect.top()), rect.left_bottom()),
                    Rect::from_min_max(rect.right_top(), Pos2::new(bounds.right(), rect.bottom())),
                ] {
                    painter.rect_filled(area, 0.0, tint);
                }
            }
            let color = Color32::from_rgb(90, 185, 255);
            painter.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, color), StrokeKind::Inside);
            for corner in [
                rect.left_top(),
                rect.right_top(),
                rect.left_bottom(),
                rect.right_bottom(),
            ] {
                painter.rect_filled(
                    Rect::from_center_size(corner, egui::vec2(8.0, 8.0)),
                    1.0,
                    Color32::WHITE,
                );
            }
        } else if !cfg!(windows) {
            painter.rect_filled(bounds, 0.0, tint);
        }
    }

    pub(super) fn toolbar(
        &mut self,
        ctx: &egui::Context,
        display: Display,
        language: Language,
    ) -> Option<Outcome> {
        let region = self.region(display);
        let mut outcome = None;
        egui::Area::new(egui::Id::new("region_toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos([16.0, 16.0])
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(tr(language, "region_instructions"));
                        ui.horizontal(|ui| {
                            if let Some(r) = region {
                                ui.label(format!("{} × {} px", r.width, r.height));
                            }
                            if ui
                                .add_enabled(
                                    region.is_some() && self.drag.is_none(),
                                    egui::Button::new(tr(language, "region_confirm")),
                                )
                                .clicked()
                            {
                                outcome = region.map(Outcome::Confirm);
                            }
                            if ui.button(tr(language, "region_cancel")).clicked() {
                                outcome = Some(Outcome::Cancel);
                            }
                        });
                    });
            });
        if self.drag.is_none() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            outcome = region.map(Outcome::Confirm);
        }
        outcome
    }
}
