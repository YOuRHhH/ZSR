#[cfg(not(windows))]
mod window;
#[cfg(not(windows))]
pub(super) use window::PickerWindow;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::PickerWindow;
#[cfg(any(not(windows), test, feature = "ui-smoke"))]
mod interaction;
mod lifecycle;
mod marker;
pub(super) use marker::RegionMarker;
#[cfg(any(not(windows), test, feature = "ui-smoke"))]
mod paint;
#[cfg(not(windows))]
mod placement;
#[cfg(feature = "ui-smoke")]
mod preview;
#[cfg(test)]
mod tests;
#[cfg(any(not(windows), test, feature = "ui-smoke"))]
use eframe::egui::{self, Pos2, Rect, Vec2};
use zsr::recording::Region;
#[cfg(any(not(windows), test, feature = "ui-smoke"))]
use zsr::{platform::Display, settings::Language};

#[cfg(not(windows))]
pub(super) const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

#[derive(Default)]
#[cfg(any(not(windows), test, feature = "ui-smoke"))]
pub(super) struct Picker {
    selection: Option<Rect>,
    drag: Option<interaction::Drag>,
    #[cfg(not(windows))]
    placed: bool,
    #[cfg(not(windows))]
    finished: bool,
    canvas_size: Vec2,
}
pub(super) enum Outcome {
    Confirm(Region),
    Cancel,
}

#[cfg(any(not(windows), test, feature = "ui-smoke"))]
impl Picker {
    fn region(&self, display: Display) -> Option<Region> {
        let rect = self.selection?;
        zsr::region::from_points(
            [rect.min.x, rect.min.y],
            [rect.max.x, rect.max.y],
            [self.canvas_size.x, self.canvas_size.y],
            [display.width, display.height],
        )
    }
    fn show(
        &mut self,
        ctx: &egui::Context,
        display: Display,
        initial: Region,
        language: Language,
    ) -> Option<Outcome> {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.viewport().close_requested()) {
            return Some(Outcome::Cancel);
        }
        let bounds = ctx.content_rect();
        if self.canvas_size != bounds.size() {
            let size = bounds.size();
            if self.canvas_size.x > 0.0
                && let Some(rect) = self.selection
            {
                let ratio = size / self.canvas_size;
                self.selection = Some(Rect::from_min_max(
                    (rect.min.to_vec2() * ratio).to_pos2(),
                    (rect.max.to_vec2() * ratio).to_pos2(),
                ));
            } else {
                let scale = size / Vec2::new(display.width as f32, display.height as f32);
                self.selection = Some(
                    Rect::from_min_size(
                        Pos2::new(initial.x as f32 * scale.x, initial.y as f32 * scale.y),
                        Vec2::new(
                            initial.width as f32 * scale.x,
                            initial.height as f32 * scale.y,
                        ),
                    )
                    .intersect(bounds),
                );
            }
            self.canvas_size = size;
        }
        self.interact(ctx, bounds);
        self.paint(ctx, bounds);
        self.toolbar(ctx, display, language)
    }
}
