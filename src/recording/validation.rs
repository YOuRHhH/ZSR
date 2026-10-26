use super::{Capture, Request};
impl Request {
    /// Reject invalid recording settings and geometry before starting the native engine.
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
            // OBS identifies displays natively; the legacy FFmpeg AVFoundation index is irrelevant.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// OBS screen requests need a native display identity, not an FFmpeg device-list index.
    #[test]
    fn native_display_does_not_require_avfoundation_index() {
        let request = Request {
            settings: crate::settings::Settings {
                microphone: false,
                ..Default::default()
            },
            capture: Capture::Screen {
                source: crate::platform::Source {
                    display: crate::platform::Display {
                        width: 1920,
                        height: 1080,
                        id: 1,
                        ..Default::default()
                    },
                    label: "Native display".into(),
                    av_index: None,
                },
                region: None,
            },
        };
        assert!(request.validate().is_ok());
    }
}
