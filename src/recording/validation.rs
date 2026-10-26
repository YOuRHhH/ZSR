use super::{Capture, Request};
impl Request {
    pub fn validate(&self) -> Result<(), String> {
        self.settings.validate()?;
        if let Capture::Window(window) = &self.capture {
            if !crate::windows::supported() {
                return Err(
                    "Application window capture is unavailable on this desktop backend.".into(),
                );
            }
            if window.id == 0 {
                return Err("Select an application window before recording.".into());
            }
        }
        if let Capture::Screen { source, region } = &self.capture {
            let d = source.display;
            if d.width < 2 || d.height < 2 {
                return Err("Invalid display dimensions".into());
            }
            if cfg!(target_os = "macos") && source.av_index.is_none() {
                return Err("AVFoundation screen index is unavailable".into());
            }
            if let Some(r) = region
                && (r.x < 0
                    || r.y < 0
                    || r.width < 2
                    || r.height < 2
                    || i64::from(r.x) + i64::from(r.width) > i64::from(d.width)
                    || i64::from(r.y) + i64::from(r.height) > i64::from(d.height))
            {
                return Err(
                    "Region must fit inside the selected display and be at least 2 × 2 pixels"
                        .into(),
                );
            }
        }
        Ok(())
    }
}
