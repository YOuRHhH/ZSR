mod inventory;
pub use inventory::WindowInventory;
#[cfg(windows)]
mod win32;
#[cfg(any(target_os = "linux", test))]
mod x11;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppWindow {
    pub id: u64,
    pub pid: u32,
    pub title: String,
    pub app: String,
}
impl AppWindow {
    pub fn identity(&self) -> (u64, u32) {
        (self.id, self.pid)
    }
    pub fn label(&self) -> String {
        format!(
            "{} — {} · {} · 0x{:x}",
            self.title, self.app, self.pid, self.id
        )
    }
}
pub fn supported() -> bool {
    cfg!(windows)
        || (cfg!(target_os = "linux")
            && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
            && std::env::var_os("WAYLAND_DISPLAY").is_none())
}
pub fn discover() -> Result<Vec<AppWindow>, String> {
    if !supported() {
        return Err("Application window capture is unavailable on this desktop backend.".into());
    }
    #[cfg(windows)]
    {
        win32::discover()
    }
    #[cfg(target_os = "linux")]
    {
        x11::discover()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("Application window capture requires a native capture backend on this platform.".into())
    }
}
pub fn validate_target(target: &AppWindow, runtime: &str) -> Result<(), String> {
    check_backend(runtime)?;
    if !discover()?
        .iter()
        .any(|w| w.identity() == target.identity())
    {
        return Err("The selected window is closed, minimized, or unavailable. Refresh and select it again.".into());
    }
    Ok(())
}
/// Window enumeration needs the configured OBS runtime, not an FFmpeg filter probe.
pub(super) fn check_backend(runtime: &str) -> Result<(), String> {
    crate::obs_backend::resolve(runtime).map(|_| ())
}

#[cfg(test)]
fn graphics_capture_available(help: &str) -> bool {
    help.contains("Filter gfxcapture")
        && [
            "hwnd",
            "capture_cursor",
            "capture_border",
            "max_framerate",
            "output_fmt",
        ]
        .iter()
        .all(|option| {
            help.lines()
                .any(|line| line.split_whitespace().next() == Some(option))
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_missing_or_incomplete_graphics_capture_support() {
        assert!(!super::graphics_capture_available(
            "Unknown filter 'gfxcapture'."
        ));
        assert!(!super::graphics_capture_available(
            "Filter gfxcapture\n window_title <string>"
        ));
        assert!(super::graphics_capture_available(
            "Filter gfxcapture\n hwnd <uint64>\n capture_cursor <boolean>\n capture_border <boolean>\n max_framerate <video_rate>\n output_fmt <int>"
        ));
    }
}
