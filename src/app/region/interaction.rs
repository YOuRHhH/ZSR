use super::Picker;
use eframe::egui::{self, Pos2, Rect, Vec2};

pub(super) enum Drag {
    Draw(Pos2),
    Move { offset: Vec2, size: Vec2 },
    Resize(Pos2),
}

impl Picker {
    pub(super) fn interact(&mut self, ctx: &egui::Context, bounds: Rect) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(bounds, egui::Sense::drag());
                if response.hovered() {
                    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if response.drag_started()
                    && let Some(origin) = ctx.input(|i| i.pointer.press_origin())
                {
                    self.drag = Some(if let Some(rect) = self.selection {
                        let corners = [
                            rect.left_top(),
                            rect.right_top(),
                            rect.right_bottom(),
                            rect.left_bottom(),
                        ];
                        if let Some(index) = corners
                            .iter()
                            .position(|corner| corner.distance(origin) <= 12.0)
                        {
                            Drag::Resize(corners[(index + 2) % 4])
                        } else if rect.contains(origin) {
                            Drag::Move {
                                offset: origin - rect.min,
                                size: rect.size(),
                            }
                        } else {
                            Drag::Draw(origin)
                        }
                    } else {
                        Drag::Draw(origin)
                    });
                }
                if let Some(position) = response.interact_pointer_pos()
                    && let Some(drag) = &self.drag
                {
                    let position = position.clamp(bounds.min, bounds.max);
                    self.selection = Some(match drag {
                        Drag::Draw(anchor) | Drag::Resize(anchor) => {
                            Rect::from_two_pos(*anchor, position)
                        }
                        Drag::Move { offset, size } => {
                            let min = (position - *offset).clamp(bounds.min, bounds.max - *size);
                            Rect::from_min_size(min, *size)
                        }
                    });
                }
                if response.drag_stopped() {
                    self.drag = None;
                }
            });
    }
}
