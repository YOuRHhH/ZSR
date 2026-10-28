//! Startup-only logo animation driven by the existing asynchronous device checks.
use super::App;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Startup {
    first_frame: Option<Instant>,
    #[cfg(windows)]
    native_window: Option<isize>,
}

impl Startup {
    /// Round the native splash outline without making the main renderer transparent.
    pub(super) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let startup = Self::default();
        #[cfg(windows)]
        let mut startup = startup;
        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = cc.window_handle()
                && let RawWindowHandle::Win32(window) = handle.as_raw()
            {
                startup.native_window = Some(window.hwnd.get());
                let scale = cc.egui_ctx.pixels_per_point();
                startup.set_frame(Vec2::splat(200.0) * scale, (16.0 * scale).round() as i32);
            }
        }
        #[cfg(not(windows))]
        let _ = cc;
        startup
    }

    /// Apply or clear the clip only while eframe still owns the live native window.
    #[cfg(windows)]
    fn set_frame(&self, size: Vec2, radius: i32) {
        unsafe extern "C" {
            fn zsr_set_splash_frame(
                window: *mut std::ffi::c_void,
                width: i32,
                height: i32,
                radius: i32,
            ) -> i32;
        }
        if let Some(window) = self.native_window {
            // The creation/update callbacks run on the HWND's owning UI thread.
            unsafe {
                zsr_set_splash_frame(
                    window as *mut std::ffi::c_void,
                    size.x.round() as i32,
                    size.y.round() as i32,
                    radius,
                )
            };
        }
    }
}

struct Phase {
    corners: f32,
    angle: f32,
    finished: bool,
}

impl Phase {
    /// Finish the short reveal once discovery settles, including unsuccessful probes.
    fn at(seconds: f32, loading: bool) -> Self {
        let fade = ((seconds - 0.18) / 0.6).clamp(0.0, 1.0);
        // Smoothstep avoids abrupt opacity changes; rotation starts at zero velocity.
        let spinning = (seconds - 2.0).max(0.0);
        Self {
            corners: fade * fade * (3.0 - 2.0 * fade),
            angle: (spinning - (1.0 - (-spinning).exp())) * 1.6,
            finished: !loading && seconds >= 0.9,
        }
    }
}

impl App {
    /// Show the animated splash until discovery finishes, then open the controls directly.
    pub(super) fn startup_screen(&mut self, ctx: &egui::Context) -> bool {
        let Some(startup) = &mut self.startup else {
            return false;
        };
        // Start on the first visible frame so constructor work cannot skip the reveal.
        let elapsed = startup
            .first_frame
            .get_or_insert_with(Instant::now)
            .elapsed();
        let phase = Phase::at(
            elapsed.as_secs_f32(),
            self.checking || self.audio.is_refreshing(),
        );
        if phase.finished {
            // Switch directly to the main window: no geometry interpolation or fades.
            #[cfg(windows)]
            startup.set_frame(Vec2::ZERO, 0);
            self.startup = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
                640.0, 480.0,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!self.custom_title_bar));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1040.0, 760.0)));
            return false;
        }
        // Outline the rounded splash in white, leaving room for rotating corners.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                // Draw inside the visible clipping region, not the larger fixed renderer.
                let bounds =
                    egui::Rect::from_center_size(ui.max_rect().center(), Vec2::splat(200.0));
                ui.painter().rect_stroke(
                    bounds.shrink(0.5),
                    16.0,
                    Stroke::new(1.0_f32, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
                paint_logo(ui.painter(), bounds.center(), 132.0, &phase);
            });
        ctx.request_repaint_after(Duration::from_millis(16));
        true
    }
}

/// Reproduce logo/svg/zsr-mark-on-dark.svg without raster scaling artifacts.
fn paint_logo(painter: &egui::Painter, center: Pos2, size: f32, phase: &Phase) {
    let scale = size / 120.0;
    painter.circle_filled(center, 22.0 * scale, Color32::from_rgb(255, 77, 61));
    let color = Color32::from_white_alpha((phase.corners * 255.0).round() as u8);
    let stroke = Stroke::new(10.0 * scale, color);
    for corner in 0..4 {
        let rotation = egui::emath::Rot2::from_angle(
            phase.angle + corner as f32 * std::f32::consts::FRAC_PI_2,
        );
        // Each bracket approaches from its own corner while it fades into place.
        let offset = Vec2::splat(-8.0 * (1.0 - phase.corners));
        let point = |x: f32, y: f32| {
            center + rotation * ((egui::vec2(x - 60.0, y - 60.0) + offset) * scale)
        };
        let mut points = vec![point(14.0, 40.0), point(14.0, 24.0)];
        for step in 1..=12 {
            let angle = std::f32::consts::PI + step as f32 / 12.0 * std::f32::consts::FRAC_PI_2;
            points.push(point(24.0 + angle.cos() * 10.0, 24.0 + angle.sin() * 10.0));
        }
        points.push(point(40.0, 14.0));
        // Explicit round caps match the SVG's stroke-linecap="round".
        painter.circle_filled(points[0], stroke.width / 2.0, color);
        painter.circle_filled(
            *points.last().expect("bracket endpoint"),
            stroke.width / 2.0,
            color,
        );
        painter.add(egui::Shape::line(points, stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::Phase;

    #[cfg(windows)]
    #[test]
    fn native_splash_clip_preserves_window_geometry_and_is_removed_for_main_ui() {
        use std::ffi::c_void;
        #[repr(C)]
        #[derive(Default, Debug, PartialEq, Eq)]
        struct Rect {
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
        }
        unsafe extern "system" {
            fn CreateWindowExW(
                ex: u32,
                class: *const u16,
                title: *const u16,
                style: u32,
                x: i32,
                y: i32,
                width: i32,
                height: i32,
                parent: *mut c_void,
                menu: *mut c_void,
                instance: *mut c_void,
                parameter: *mut c_void,
            ) -> *mut c_void;
            fn DestroyWindow(window: *mut c_void) -> i32;
            fn GetWindowRect(window: *mut c_void, rect: *mut Rect) -> i32;
            fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> *mut c_void;
            fn GetWindowRgn(window: *mut c_void, region: *mut c_void) -> i32;
            fn GetRgnBox(region: *mut c_void, rect: *mut Rect) -> i32;
            fn DeleteObject(object: *mut c_void) -> i32;
        }
        unsafe extern "C" {
            fn zsr_set_splash_frame(
                window: *mut c_void,
                width: i32,
                height: i32,
                radius: i32,
            ) -> i32;
        }
        struct Window(*mut c_void);
        impl Drop for Window {
            fn drop(&mut self) {
                // Release the hidden test window even if an assertion fails.
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // A hidden standard window exercises real clipping without touching the user's app.
        unsafe {
            let null = std::ptr::null_mut();
            let window = Window(CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0x80000000,
                100,
                100,
                1040,
                760,
                null,
                null,
                null,
                null,
            ));
            assert!(!window.0.is_null());
            let mut original = Rect::default();
            assert_ne!(GetWindowRect(window.0, &mut original), 0);
            {
                let size = eframe::egui::Vec2::splat(200.0);
                assert_ne!(
                    zsr_set_splash_frame(window.0, size.x as i32, size.y as i32, 16),
                    0
                );
                let mut actual = Rect::default();
                assert_ne!(GetWindowRect(window.0, &mut actual), 0);
                assert_eq!(actual, original);
                let region = CreateRectRgn(0, 0, 0, 0);
                assert!(!region.is_null());
                let mut clip = Rect::default();
                let kind = GetWindowRgn(window.0, region);
                GetRgnBox(region, &mut clip);
                DeleteObject(region);
                assert_ne!(kind, 0);
                assert!((clip.right - clip.left - size.x as i32).abs() <= 1);
                assert!((clip.bottom - clip.top - size.y as i32).abs() <= 1);
                assert!((clip.left + clip.right - 1040).abs() <= 1);
                assert!((clip.top + clip.bottom - 760).abs() <= 1);
            }
            assert_ne!(zsr_set_splash_frame(window.0, 0, 0, 0), 0);
            let region = CreateRectRgn(0, 0, 0, 0);
            assert!(!region.is_null());
            let kind = GetWindowRgn(window.0, region);
            DeleteObject(region);
            assert_eq!(kind, 0, "the main window must no longer be clipped");
        }
    }

    #[test]
    fn reveal_precedes_rotation_and_waits_for_loading() {
        let first = Phase::at(0.0, false);
        assert_eq!(first.corners, 0.0);
        assert!(!first.finished);
        let revealed = Phase::at(0.9, true);
        assert_eq!(revealed.corners, 1.0);
        assert_eq!(revealed.angle, 0.0);
        assert!(!revealed.finished);
        assert_eq!(Phase::at(2.0, true).angle, 0.0);
        assert!(Phase::at(2.1, true).angle > 0.0);
        assert!(!Phase::at(20.0, true).finished);
        assert!(Phase::at(0.9, false).finished);
        assert!(Phase::at(3.0, false).finished);
    }
}
