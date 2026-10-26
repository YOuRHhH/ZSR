use super::{Capture, Region, Request};
use std::path::Path;
impl Request {
    /// Build argv directly; values never pass through a shell.
    pub fn arguments(&self, output: &Path) -> Vec<String> {
        self.arguments_with_screen_input(output, None)
    }

    /// Use a validated compositor input when available while retaining compatibility inputs.
    pub(super) fn arguments_with_screen_input(
        &self,
        output: &Path,
        screen_input: Option<&str>,
    ) -> Vec<String> {
        let s = &self.settings;
        let mut args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "warning".into(),
            "-y".into(),
        ];
        let mut add = |items: &[&str]| {
            args.extend(items.iter().map(|s| s.to_string()));
        };
        let fps = s.fps.to_string();
        match &self.capture {
            Capture::Window(window) => {
                let cursor = if s.cursor { "1" } else { "0" };
                if cfg!(windows) {
                    add(&[
                        "-f",
                        "lavfi",
                        "-i",
                        &format!(
                            "gfxcapture=hwnd={}:capture_cursor={cursor}:capture_border=0:max_framerate={fps}:output_fmt=bgra,hwdownload,format=bgra",
                            window.id
                        ),
                    ]);
                } else {
                    add(&[
                        "-f",
                        "x11grab",
                        "-framerate",
                        &fps,
                        "-draw_mouse",
                        cursor,
                        "-window_id",
                        &window.id.to_string(),
                        "-i",
                        &std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".into()),
                    ]);
                }
            }
            Capture::TestPattern => {
                add(&["-re", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30"])
            }
            Capture::Screen { source, region } => {
                let d = source.display;
                let r = region.unwrap_or(Region {
                    x: 0,
                    y: 0,
                    width: d.width,
                    height: d.height,
                });
                let x = (i64::from(d.x) + i64::from(r.x)).to_string();
                let y = (i64::from(d.y) + i64::from(r.y)).to_string();
                let size = format!("{}x{}", r.width, r.height);
                let cursor = if s.cursor { "1" } else { "0" };
                if cfg!(windows)
                    && let Some(input) = screen_input
                {
                    add(&["-f", "lavfi", "-i", input]);
                } else if cfg!(windows) {
                    add(&[
                        "-f",
                        "gdigrab",
                        "-framerate",
                        &fps,
                        "-draw_mouse",
                        cursor,
                        "-offset_x",
                        &x,
                        "-offset_y",
                        &y,
                        "-video_size",
                        &size,
                        "-i",
                        "desktop",
                    ]);
                } else if cfg!(target_os = "macos") {
                    let input = format!(
                        "{}:{}",
                        source.av_index.unwrap_or(0),
                        if s.microphone {
                            s.audio_device.as_str()
                        } else {
                            "none"
                        }
                    );
                    add(&[
                        "-f",
                        "avfoundation",
                        "-capture_cursor",
                        cursor,
                        "-framerate",
                        &fps,
                        "-i",
                        &input,
                    ]);
                } else {
                    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0.0".into());
                    let input = format!("{display}+{x},{y}");
                    add(&[
                        "-f",
                        "x11grab",
                        "-framerate",
                        &fps,
                        "-draw_mouse",
                        cursor,
                        "-video_size",
                        &size,
                        "-i",
                        &input,
                    ]);
                }
            }
        }
        if s.microphone
            && !cfg!(target_os = "macos")
            && !matches!(self.capture, Capture::TestPattern)
        {
            if cfg!(windows) {
                add(&[
                    "-thread_queue_size",
                    "512",
                    "-f",
                    "dshow",
                    "-i",
                    &format!("audio={}", s.audio_device),
                ]);
            } else {
                add(&[
                    "-thread_queue_size",
                    "512",
                    "-f",
                    "pulse",
                    "-i",
                    &s.audio_device,
                ]);
            }
        }
        add(&["-map", "0:v:0"]);
        if s.microphone && !matches!(self.capture, Capture::TestPattern) {
            add(&[
                "-map",
                if cfg!(target_os = "macos") {
                    "0:a:0"
                } else {
                    "1:a:0"
                },
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-af",
                "aresample=async=1",
            ]);
        } else {
            add(&["-an"]);
        }
        add(&[
            "-vf",
            &self.video_filter(),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            &s.crf.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-r",
            &fps,
            "-g",
            &(s.fps * 2).to_string(),
            "-movflags",
            "+frag_keyframe+empty_moov+default_base_moof",
            "-progress",
            "pipe:1",
            "-stats_period",
            "0.5",
            "-f",
            "mp4",
        ]);
        args.push(output.to_string_lossy().into_owned());
        args
    }

    fn video_filter(&self) -> String {
        let scale = self.settings.resolution.video_filter();
        if cfg!(target_os = "macos")
            && let Capture::Screen {
                region: Some(r), ..
            } = &self.capture
        {
            // AVFoundation captures the display; crop in physical source pixels.
            return format!(
                "crop={}:{}:{}:{}:exact=1,{scale}",
                r.width, r.height, r.x, r.y
            );
        }
        scale
    }
}
