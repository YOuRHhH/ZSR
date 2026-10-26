use eframe::egui;
use zsr::{platform::Display, recording::Region};

#[derive(Default)]
pub(in crate::app) struct RegionMarker {
    pub(super) confirmed: bool,
    bounds: Option<[i32; 4]>,
    #[cfg(windows)]
    handle: usize,
}
#[cfg(windows)]
unsafe extern "C" {
    fn zsr_marker_create() -> usize;
    fn zsr_marker_position(handle: usize, x: i32, y: i32, width: i32, height: i32) -> i32;
    fn zsr_marker_destroy(handle: usize);
}

impl RegionMarker {
    pub(in crate::app) fn update(
        &mut self,
        ctx: &egui::Context,
        target: Option<(Display, Region)>,
    ) {
        let bounds = target
            .filter(|_| self.confirmed)
            .and_then(|(d, r)| marker_bounds(d, r));
        #[cfg(windows)]
        {
            let _ = ctx;
            if self.bounds == bounds {
                return;
            }
            if let Some([x, y, width, height]) = bounds {
                // SAFETY: the native marker is created, positioned, and destroyed
                // only on the main UI thread. No native pointer escapes this owner.
                unsafe {
                    if self.handle == 0 {
                        self.handle = zsr_marker_create();
                    }
                    if self.handle == 0
                        || zsr_marker_position(self.handle, x, y, width, height) == 0
                    {
                        return;
                    }
                }
            } else {
                self.close();
            }
        }
        #[cfg(not(windows))]
        if let Some([x, y, width, height]) = bounds {
            let scale = ctx.pixels_per_point();
            #[cfg(not(target_os = "macos"))]
            let position = [(x - 2) as f32 / scale, (y - 2) as f32 / scale];
            #[cfg(target_os = "macos")]
            let position = {
                let _ = (x, y);
                let (display, region) = target.expect("validated marker target");
                [
                    display.x as f32 / ctx.zoom_factor() + (region.x - 2) as f32 / scale,
                    display.y as f32 / ctx.zoom_factor() + (region.y - 2) as f32 / scale,
                ]
            };
            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("confirmed_region"),
                egui::ViewportBuilder::default()
                    .with_title("ZSR recording area")
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_always_on_top()
                    .with_active(false)
                    .with_mouse_passthrough(true)
                    .with_taskbar(false)
                    .with_has_shadow(false)
                    .with_position(position)
                    .with_inner_size([(width + 4) as f32 / scale, (height + 4) as f32 / scale]),
                |ctx, _| {
                    ctx.layer_painter(egui::LayerId::background()).rect_stroke(
                        ctx.content_rect(),
                        0.0,
                        egui::Stroke::new(
                            2.0 / ctx.pixels_per_point(),
                            egui::Color32::from_rgb(90, 185, 255),
                        ),
                        egui::StrokeKind::Inside,
                    );
                },
            );
        }
        self.bounds = bounds;
    }
    #[cfg(windows)]
    fn close(&mut self) {
        if self.handle != 0 {
            // SAFETY: unique native ownership; destroyed once on its creating thread.
            unsafe {
                zsr_marker_destroy(self.handle);
            }
            self.handle = 0;
        }
    }
}
#[cfg(windows)]
impl Drop for RegionMarker {
    fn drop(&mut self) {
        self.close();
    }
}

fn marker_bounds(display: Display, region: Region) -> Option<[i32; 4]> {
    if region.x < 0
        || region.y < 0
        || region.width < 2
        || region.height < 2
        || i64::from(region.x) + i64::from(region.width) > i64::from(display.width)
        || i64::from(region.y) + i64::from(region.height) > i64::from(display.height)
    {
        return None;
    }
    let x = display.x.checked_add(region.x)?;
    let y = display.y.checked_add(region.y)?;
    x.checked_sub(2)?;
    y.checked_sub(2)?;
    x.checked_add(region.width)?.checked_add(2)?;
    y.checked_add(region.height)?.checked_add(2)?;
    Some([x, y, region.width, region.height])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn native_marker_is_click_through_outside_capture_and_destroyed() {
        unsafe extern "C" {
            fn zsr_marker_verify() -> i32;
        }
        // SAFETY: self-contained native test creates and destroys its own windows.
        assert_eq!(unsafe { zsr_marker_verify() }, 0);
    }
    #[test]
    fn marker_matches_capture_on_negative_origin_and_rejects_invalid_bounds() {
        let display = Display {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
            id: 0,
        };
        let region = Region {
            x: 100,
            y: 200,
            width: 1280,
            height: 720,
        };
        assert_eq!(
            marker_bounds(display, region),
            Some([-1820, 200, 1280, 720])
        );
        assert!(
            marker_bounds(
                display,
                Region {
                    width: 1920,
                    ..region
                }
            )
            .is_none()
        );
    }
}
