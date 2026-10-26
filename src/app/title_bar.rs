use super::App;
use eframe::egui::{self, ViewportCommand};

impl App {
    /// Provide a black draggable title bar when Windows rejects explicit native caption colors.
    pub(super) fn title_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("window_title_bar")
            .exact_height(34.0)
            .show(ctx, |ui| {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.horizontal(|ui| {
                    // Allocate dragging only to the title area, never over window control buttons.
                    let width = (ui.available_width() - 180.0).max(30.0);
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(width, 28.0),
                        egui::Sense::click_and_drag(),
                    );
                    ui.painter().text(
                        rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        "ZSR",
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                    if response.double_clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    } else if response.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if ui.button("−").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    if ui.button(if maximized { "❐" } else { "□" }).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                    }
                    if ui.button("×").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
    }
}
