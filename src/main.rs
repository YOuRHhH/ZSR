#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod app;
mod app_icon;

/// Create an opaque main window; only the independent region overlays need transparency.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        // Present completed frames on the display refresh boundary.
        vsync: true,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_transparent(
                cfg!(feature = "ui-smoke")
                    && std::env::var_os("ZSR_UI_TRANSPARENT_BASELINE").is_some(),
            )
            .with_title("ZSR")
            .with_icon(app_icon::load())
            .with_app_id("zsr")
            .with_inner_size([1040.0, 760.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ZSR",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
