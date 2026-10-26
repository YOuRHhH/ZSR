use eframe::egui::{self, Color32};
use std::{path::PathBuf, sync::LazyLock};

// Static borrowed bytes let egui and its font parser share one allocation.
static CJK_FONT: LazyLock<Option<Vec<u8>>> = LazyLock::new(load_cjk_font);

/// Style only the main native frame while preserving standard system window controls.
pub(super) fn configure_title_bar(cc: &eframe::CreationContext<'_>) -> bool {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        unsafe extern "C" {
            fn zsr_style_title_bar(window: *mut std::ffi::c_void) -> i32;
        }
        if let Ok(handle) = cc.window_handle()
            && let RawWindowHandle::Win32(window) = handle.as_raw()
        {
            // The creation context owns this live HWND throughout initialization.
            if unsafe { zsr_style_title_bar(window.hwnd.get() as *mut std::ffi::c_void) } == 0 {
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                return true;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = cc;
    false
}

/// Apply a high-contrast desktop theme and install a system CJK fallback.
pub(super) fn configure(ctx: &egui::Context) -> bool {
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.visuals = high_contrast_visuals();
    ctx.set_style(style);
    install_cjk_font(ctx)
}

/// Keep text bright, controls outlined, and hover/selection states distinct on black surfaces.
fn high_contrast_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    let accent = Color32::from_rgb(110, 220, 255);
    visuals.override_text_color = Some(Color32::WHITE);
    visuals.weak_text_color = Some(Color32::from_gray(210));
    visuals.disabled_alpha = 0.65;
    visuals.panel_fill = Color32::BLACK;
    visuals.window_fill = Color32::from_gray(8);
    visuals.extreme_bg_color = Color32::BLACK;
    visuals.faint_bg_color = Color32::from_gray(24);
    visuals.window_stroke = egui::Stroke::new(1.5_f32, Color32::from_gray(190));
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = Color32::from_rgb(0, 65, 115);
    visuals.selection.stroke = egui::Stroke::new(2.0_f32, Color32::WHITE);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = Color32::from_gray(24);
        widget.weak_bg_fill = Color32::from_gray(24);
        widget.bg_stroke = egui::Stroke::new(1.5_f32, Color32::from_gray(170));
        widget.fg_stroke = egui::Stroke::new(2.0_f32, Color32::WHITE);
    }
    for widget in [
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.bg_fill = Color32::from_rgb(15, 45, 65);
        widget.weak_bg_fill = widget.bg_fill;
        widget.bg_stroke = egui::Stroke::new(2.0_f32, accent);
    }
    visuals
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
