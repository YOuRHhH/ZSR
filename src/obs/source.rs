use super::{
    api::{Crop, Ptr, Vec2},
    runtime::{Runtime, c},
};
use std::{
    ptr::null_mut,
    time::{Duration, Instant},
};
use zsr::recording::{Capture, Request};

/// Create a capture source using OBS device identity, preserving the user's selected target.
pub fn video(runtime: &Runtime, request: &Request) -> Result<(Ptr, Option<Crop>), String> {
    let (id, settings, crop) = match &request.capture {
        Capture::Screen { source, region } => {
            #[cfg(windows)]
            let settings = {
                unsafe extern "C" {
                    fn zsr_obs_monitor_id(
                        display: *const zsr::platform::Display,
                        out: *mut std::ffi::c_char,
                        capacity: i32,
                    ) -> i32;
                }
                let mut buffer = [0i8; 512];
                if unsafe { zsr_obs_monitor_id(&source.display, buffer.as_mut_ptr(), 512) } == 0 {
                    return Err("Selected monitor is no longer available".into());
                }
                serde_json::json!({"monitor_id":unsafe {std::ffi::CStr::from_ptr(buffer.as_ptr())}.to_string_lossy(),"method":0,"capture_cursor":request.settings.cursor})
            };
            #[cfg(not(windows))]
            let settings = if cfg!(target_os = "macos") {
                serde_json::json!({"display":source.display.id,"show_cursor":request.settings.cursor})
            } else {
                serde_json::json!({"screen":source.display.id,"show_cursor":request.settings.cursor})
            };
            let id = if cfg!(windows) {
                "monitor_capture"
            } else if cfg!(target_os = "macos") {
                "display_capture"
            } else {
                "xshm_input"
            };
            (
                id,
                settings,
                region.map(|r| Crop {
                    left: r.x,
                    top: r.y,
                    right: source.display.width - r.x - r.width,
                    bottom: source.display.height - r.y - r.height,
                }),
            )
        }
        Capture::Window(window) => {
            if cfg!(windows) {
                zsr::windows::discover()?
                    .into_iter()
                    .find(|w| w.identity() == window.identity())
                    .ok_or("Selected window is unavailable")?;
                let candidates = runtime.choices("window_capture", "window");
                let escaped = |s: &str| s.replace('#', "#22").replace(':', "#3A");
                let matches: Vec<_> = candidates
                    .iter()
                    .filter(|v| {
                        v["id"].as_str().is_some_and(|id| {
                            id.starts_with(&(escaped(&window.title) + ":"))
                                && id.ends_with(&format!(":{}", escaped(&window.app)))
                        })
                    })
                    .collect();
                if matches.len() != 1 {
                    return Err("OBS could not uniquely identify this window; choose a window with a distinct title".into());
                }
                (
                    "window_capture",
                    serde_json::json!({"window":matches[0]["id"],"method":2,"priority":0,"cursor":request.settings.cursor,"client_area":true}),
                    None,
                )
            } else {
                (
                    "xcomposite_input",
                    serde_json::json!({"capture_window":window.id,"show_cursor":request.settings.cursor}),
                    None,
                )
            }
        }
        Capture::TestPattern => {
            return Err("OBS test pattern is not a desktop capture source".into());
        }
    };
    let data = runtime.data(&settings);
    let source = unsafe {
        (runtime.api.obs_source_create)(c(id).as_ptr(), c("ZSR capture").as_ptr(), data, null_mut())
    };
    unsafe {
        (runtime.api.obs_data_release)(data);
    }
    if source.is_null() {
        return Err(format!("OBS source unavailable: {id}"));
    }
    Ok((source, crop))
}

/// Wait for native capture startup and fit the cropped image inside the canvas.
pub fn layout(
    runtime: &Runtime,
    source: Ptr,
    item: Ptr,
    crop: Option<Crop>,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let start = Instant::now();
    while unsafe { (runtime.api.obs_source_get_width)(source) } == 0
        || unsafe { (runtime.api.obs_source_get_height)(source) } == 0
    {
        if start.elapsed() > Duration::from_secs(10) {
            return Err(
                "OBS capture produced no image; check permissions and selected window".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    unsafe {
        if let Some(crop) = crop {
            (runtime.api.obs_sceneitem_set_crop)(item, &crop);
        }
        (runtime.api.obs_sceneitem_set_bounds_type)(item, 2);
        (runtime.api.obs_sceneitem_set_bounds_alignment)(item, 0);
        (runtime.api.obs_sceneitem_set_bounds)(
            item,
            &Vec2 {
                x: width as f32,
                y: height as f32,
            },
        );
    }
    Ok(())
}
