use super::{Capture, Request, diagnostics::Diagnostics};
use std::{
    process::Stdio,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encoder {
    Software,
    Amf,
    Nvenc,
    Qsv,
}

impl Encoder {
    /// Return the backend name stored in each recording's diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Software => "libx264",
            Self::Amf => "h264_amf",
            Self::Nvenc => "h264_nvenc",
            Self::Qsv => "h264_qsv",
        }
    }

    /// Map the compression preference to each encoder's own quality controls.
    pub fn arguments(self, quality: u32, fps: u32) -> Vec<String> {
        let q = quality.to_string();
        let options = match self {
            Self::Software => vec![
                "-preset",
                if fps >= 60 { "ultrafast" } else { "veryfast" },
                "-crf",
                &q,
                "-pix_fmt",
                "yuv420p",
            ],
            Self::Amf => vec![
                "-quality", "speed", "-rc", "cqp", "-qp_i", &q, "-qp_p", &q, "-pix_fmt", "nv12",
            ],
            Self::Nvenc => vec![
                "-preset", "p1", "-rc", "vbr", "-cq", &q, "-b:v", "0", "-pix_fmt", "nv12",
            ],
            Self::Qsv => vec![
                "-preset",
                "veryfast",
                "-global_quality",
                &q,
                "-pix_fmt",
                "nv12",
            ],
        };
        [vec!["-c:v", self.name()], options]
            .concat()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

/// Try real encoder initialization on Windows; unsupported drivers fall back before capture starts.
pub(super) fn select(request: &Request, log: &mut Diagnostics) -> Encoder {
    if cfg!(windows)
        && request.settings.hardware_encoding
        && !matches!(request.capture, Capture::TestPattern)
    {
        for encoder in [Encoder::Amf, Encoder::Nvenc, Encoder::Qsv] {
            match probe(request, encoder) {
                Ok(()) => {
                    log.write("encoder_selected", encoder.name());
                    return encoder;
                }
                Err(error) => log.write(
                    "encoder_probe_failed",
                    serde_json::json!({"encoder": encoder.name(), "reason": error}),
                ),
            }
        }
    }
    log.write("encoder_selected", "libx264");
    Encoder::Software
}

/// Encode two synthetic frames at the intended output size with a bounded driver timeout.
fn probe(request: &Request, encoder: Encoder) -> Result<(), String> {
    let source_size = match &request.capture {
        Capture::Screen { source, region } => region
            .map(|r| (r.width as u32, r.height as u32))
            .unwrap_or((source.display.width as u32, source.display.height as u32)),
        _ => (1280, 720),
    };
    let (width, height) = request
        .settings
        .resolution
        .dimensions()
        .unwrap_or(source_size);
    let input = format!(
        "color=size={}x{}:rate={}",
        width / 2 * 2,
        height / 2 * 2,
        request.settings.fps
    );
    let mut errors = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut child = crate::platform::command(&request.settings.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            &input,
            "-frames:v",
            "2",
        ])
        .args(encoder.arguments(request.settings.crf, request.settings.fps))
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(errors.try_clone().map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                use std::io::{Read, Seek};
                let _ = errors.rewind();
                let mut text = String::new();
                let _ = errors.take(4096).read_to_string(&mut text);
                return Err(format!("{status}: {text}"));
            }
            Ok(None) if start.elapsed() < Duration::from_secs(5) => {
                std::thread::sleep(Duration::from_millis(30))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("encoder initialization timed out or failed".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Hardware quality controls must not accidentally inherit software-only CRF flags.
    #[test]
    fn encoder_options_are_separate() {
        for encoder in [Encoder::Amf, Encoder::Nvenc, Encoder::Qsv] {
            let args = encoder.arguments(23, 60);
            assert!(!args.iter().any(|arg| arg == "-crf" || arg == "ultrafast"));
            assert!(args.windows(2).any(|pair| pair == ["-c:v", encoder.name()]));
        }
        assert!(
            Encoder::Software
                .arguments(23, 60)
                .iter()
                .any(|arg| arg == "ultrafast")
        );
        assert!(
            Encoder::Software
                .arguments(23, 30)
                .iter()
                .any(|arg| arg == "veryfast")
        );
    }
}
