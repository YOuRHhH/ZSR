use zsr::platform::Display;

#[cfg(target_os = "macos")]
pub(super) fn adjust_macos(ctx: &eframe::egui::Context, display: Display) {
    use eframe::egui::{Pos2, Vec2, ViewportCommand};
    // CoreGraphics origins use desktop points, while capture sizes are pixels.
    // Use the overlay's own DPI after it moves to its target display.
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(
        display.x as f32 / ctx.zoom_factor(),
        display.y as f32 / ctx.zoom_factor(),
    )));
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
        display.width as f32 / ctx.pixels_per_point(),
        display.height as f32 / ctx.pixels_per_point(),
    )));
}

pub(super) fn position(_title: &str, _display: Display) -> bool {
    true
}
