use super::{Capture, Request};
#[cfg(windows)]
mod duplication;

pub(super) struct Input {
    pub graph: String,
    pub device: Option<String>,
}

/// Prefer compositor-driven capture when FFmpeg supports exact monitor and region selection.
pub(super) fn input(request: &Request) -> Result<Option<Input>, String> {
    if !cfg!(windows) || !matches!(request.capture, Capture::Screen { .. }) {
        return Ok(None);
    }
    #[cfg(windows)]
    if let Some(input) = duplication::input(request)? {
        return Ok(Some(input));
    }
    let help = crate::platform::output(
        crate::platform::command(&request.settings.ffmpeg).args([
            "-hide_banner",
            "-h",
            "filter=gfxcapture",
        ]),
        std::time::Duration::from_secs(5),
    )?;
    if !supported(&help) {
        return Ok(None);
    }
    #[cfg(windows)]
    if let Capture::Screen { source, .. } = &request.capture {
        unsafe extern "C" {
            fn zsr_display_monitor(display: *const crate::platform::Display) -> u64;
        }
        // SAFETY: The C adapter reads one repr(C) display and retains no pointer.
        let monitor = unsafe { zsr_display_monitor(&source.display) };
        if monitor == 0 {
            return Err(
                "Display layout changed. Refresh displays and select the recording area again."
                    .into(),
            );
        }
        return Ok(Some(Input {
            graph: graph(request, monitor),
            device: None,
        }));
    }
    Ok(None)
}

/// Require every option we use before choosing WGC; old Lite backends retain GDI support.
fn supported(help: &str) -> bool {
    help.contains("Filter gfxcapture")
        && [
            "hmonitor",
            "capture_cursor",
            "max_framerate",
            "output_fmt",
            "crop_left",
            "crop_top",
            "crop_right",
            "crop_bottom",
        ]
        .iter()
        .all(|option| {
            help.lines()
                .any(|line| line.split_whitespace().next() == Some(option))
        })
}

/// Crop in GPU capture coordinates before downloading pixels for the existing encoder.
#[cfg(any(windows, test))]
fn graph(request: &Request, monitor: u64) -> String {
    let Capture::Screen { source, region } = &request.capture else {
        unreachable!()
    };
    let d = source.display;
    let r = region.unwrap_or(super::Region {
        x: 0,
        y: 0,
        width: d.width,
        height: d.height,
    });
    format!(
        "gfxcapture=hmonitor={monitor}:capture_cursor={}:max_framerate={}:output_fmt=bgra:crop_left={}:crop_top={}:crop_right={}:crop_bottom={},hwdownload,format=bgra",
        u8::from(request.settings.cursor),
        request.settings.fps,
        r.x,
        r.y,
        d.width - r.x - r.width,
        d.height - r.y - r.height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Old filters lacking monitor or cropping options must use compatibility capture.
    #[test]
    fn rejects_incomplete_backend() {
        assert!(!supported("Unknown filter 'gfxcapture'"));
        assert!(!supported("Filter gfxcapture\n hwnd <uint64>"));
        assert!(supported(
            "Filter gfxcapture\n hmonitor x\n capture_cursor x\n max_framerate x\n output_fmt x\n crop_left x\n crop_top x\n crop_right x\n crop_bottom x"
        ));
    }

    /// Negative desktop origins must not leak into monitor-relative region crop values.
    #[test]
    fn crops_region_relative_to_exact_monitor() {
        let request = Request {
            settings: crate::settings::Settings::default(),
            capture: Capture::Screen {
                source: crate::platform::Source {
                    display: crate::platform::Display {
                        x: -1920,
                        y: -1080,
                        width: 1920,
                        height: 1080,
                        id: 3,
                    },
                    label: String::new(),
                    av_index: None,
                },
                region: Some(super::super::Region {
                    x: 100,
                    y: 50,
                    width: 640,
                    height: 360,
                }),
            },
        };
        let input = graph(&request, 123456);
        assert!(input.starts_with("gfxcapture=hmonitor=123456:"));
        assert!(input.contains("crop_left=100:crop_top=50:crop_right=1180:crop_bottom=670"));
        let args =
            request.arguments_with_screen_input(std::path::Path::new("out.mp4"), Some(&input));
        if cfg!(windows) {
            assert!(
                args.windows(2)
                    .any(|pair| pair[0] == "-filter_complex" && pair[1].starts_with(&input))
            );
            assert!(!args.iter().any(|arg| arg == "gdigrab" || arg == "desktop"));
        }
    }
}
