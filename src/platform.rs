use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Display {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub id: u32,
}
unsafe extern "C" {
    fn zsr_displays(out: *mut Display, capacity: i32) -> i32;
}

#[derive(Clone, Debug)]
pub struct Source {
    pub display: Display,
    pub label: String,
    pub av_index: Option<u32>,
}

#[derive(Debug)]
pub struct Capabilities {
    pub sources: Vec<Source>,
    pub error: Option<String>,
    pub wayland: bool,
}

pub fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW; FFmpeg uses piped stdin.
    }
    cmd
}

pub fn probe(ffmpeg: &str) -> Capabilities {
    let wayland = cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
            || std::env::var_os("WAYLAND_DISPLAY").is_some());
    let mut result = Capabilities {
        sources: Vec::new(),
        error: None,
        wayland,
    };
    if wayland {
        return result;
    }
    let detected = (|| -> Result<Vec<Source>, String> {
        let mut buffer = [Display::default(); 32];
        // SAFETY: repr(C) matches display.h; valid writable fixed-size buffer;
        // the C adapter retains no pointer and writes no more than capacity.
        let count = unsafe { zsr_displays(buffer.as_mut_ptr(), buffer.len() as i32) };
        if count <= 0 {
            return Err("No native displays found. Check your desktop session.".into());
        }
        let encoders = output(
            command(ffmpeg).args(["-hide_banner", "-encoders"]),
            Duration::from_secs(8),
        )?;
        if !encoders.contains("libx264") {
            return Err("FFmpeg must include the libx264 encoder.".into());
        }
        let devices = output(
            command(ffmpeg).args(["-hide_banner", "-devices"]),
            Duration::from_secs(8),
        )?;
        let required = if cfg!(windows) {
            "gdigrab"
        } else if cfg!(target_os = "macos") {
            "avfoundation"
        } else {
            "x11grab"
        };
        if !devices.contains(required) {
            return Err(format!(
                "FFmpeg does not include {required} capture support."
            ));
        }
        let mut sources: Vec<Source> = buffer[..count as usize]
            .iter()
            .enumerate()
            .map(|(i, display)| Source {
                display: *display,
                label: format!(
                    "{} · {} × {} · ({}, {})",
                    i + 1,
                    display.width,
                    display.height,
                    display.x,
                    display.y
                ),
                av_index: None,
            })
            .collect();
        if cfg!(target_os = "macos") {
            // Device enumeration intentionally exits nonzero on FFmpeg; inspect its diagnostic text.
            let list = output(
                command(ffmpeg).args([
                    "-hide_banner",
                    "-f",
                    "avfoundation",
                    "-list_devices",
                    "true",
                    "-i",
                    "",
                ]),
                Duration::from_secs(8),
            )?;
            let indices = mac_screen_indices(&list);
            sources = indices
                .iter()
                .enumerate()
                .filter_map(|(i, index)| {
                    sources.get(i).cloned().map(|mut s| {
                        s.av_index = Some(*index);
                        s
                    })
                })
                .collect();
            if sources.is_empty() {
                return Err("FFmpeg did not report an AVFoundation screen. Check Screen Recording permission.".into());
            }
        }
        Ok(sources)
    })();
    match detected {
        Ok(sources) => result.sources = sources,
        Err(error) => result.error = Some(error),
    }
    result
}

fn mac_screen_indices(text: &str) -> Vec<u32> {
    text.lines()
        .filter_map(|line| {
            let before = line
                .split("Capture screen")
                .next()
                .filter(|_| line.contains("Capture screen"))?;
            before.rsplit('[').next()?.split(']').next()?.parse().ok()
        })
        .collect()
}

/// Bounded setup probe. Both pipes are drained concurrently to avoid deadlock.
pub fn output(cmd: &mut Command, timeout: Duration) -> Result<String, String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    fn drain(mut pipe: impl Read) -> Vec<u8> {
        let mut bytes = Vec::new();
        let _ = (&mut pipe).take(2 * 1024 * 1024).read_to_end(&mut bytes);
        bytes
    }
    let out = thread::spawn(move || drain(stdout));
    let err = thread::spawn(move || drain(stderr));
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("FFmpeg setup check timed out or could not complete".into());
            }
        }
    }
    let mut bytes = out.join().unwrap_or_default();
    bytes.extend(err.join().unwrap_or_default());
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mac_indices_are_not_camera_indices() {
        assert_eq!(
            mac_screen_indices(
                "[AVFoundation @ abc] [0] Camera\n[AVFoundation @ abc] [2] Capture screen 0\n[AVFoundation @ abc] [3] Capture screen 1"
            ),
            vec![2, 3]
        );
    }
}
