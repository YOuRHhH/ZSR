use eframe::egui::{self, Color32};
use std::{path::PathBuf, sync::LazyLock};

// Static borrowed bytes let egui and its font parser share one allocation.
static CJK_FONT: LazyLock<Option<Vec<u8>>> = LazyLock::new(load_cjk_font);

/// Apply the shared desktop theme and install a system CJK fallback.
pub(super) fn configure(ctx: &egui::Context) -> bool {
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.visuals.selection.bg_fill = Color32::from_rgb(52, 105, 175);
    ctx.set_style(style);
    install_cjk_font(ctx)
}

/// Register borrowed font data, avoiding egui's copy of owned font buffers.
fn install_cjk_font(ctx: &egui::Context) -> bool {
    let Some(bytes) = CJK_FONT.as_deref() else {
        return false;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), egui::FontData::from_static(bytes).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    ctx.set_fonts(fonts);
    true
}

/// Load the first available system CJK font once for all application viewports.
fn load_cjk_font() -> Option<Vec<u8>> {
    let mut candidates = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        candidates.push(PathBuf::from(windir).join("Fonts/msyh.ttc"));
    }
    candidates.extend(
        [
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}
