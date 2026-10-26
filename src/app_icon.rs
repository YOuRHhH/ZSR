pub fn load() -> eframe::egui::IconData {
    #[cfg(windows)]
    let bytes = include_bytes!("../logo/windows/zsr-256.png").as_slice();
    #[cfg(target_os = "macos")]
    let bytes = include_bytes!("../logo/macos/zsr.iconset/icon_128x128@2x.png").as_slice();
    #[cfg(not(any(windows, target_os = "macos")))]
    let bytes = include_bytes!("../logo/linux/hicolor/256x256/apps/zsr.png").as_slice();
    eframe::icon_data::from_png_bytes(bytes)
        .expect("bundled ZSR application icon must be valid PNG")
}
