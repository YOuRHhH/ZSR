#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod app;
mod app_icon;

/// Open a compact splash; startup completion restores the main recording window.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        // Present completed frames on the display refresh boundary.
        vsync: true,
        centered: true,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_transparent(
                cfg!(feature = "ui-smoke")
                    && std::env::var_os("ZSR_UI_TRANSPARENT_BASELINE").is_some(),
            )
            .with_title("ZSR")
            .with_icon(app_icon::load())
            .with_app_id("zsr")
            // Windows reveals a 200 × 200 region of the final-size renderer, avoiding
            // a large splash window before switching directly to the main interface.
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(if cfg!(windows) {
                [1040.0, 760.0]
            } else {
                [200.0, 200.0]
            })
            .with_min_inner_size([200.0, 200.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ZSR",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
