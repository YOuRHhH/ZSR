use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

/// Discover native displays only after the libobs runtime has passed initialization.
pub fn probe(runtime: &str) -> Capabilities {
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
        crate::obs_backend::probe(runtime)?;
        let mut buffer = [Display::default(); 32];
        // SAFETY: Native adapter writes at most the declared array capacity.
        let count = unsafe { zsr_displays(buffer.as_mut_ptr(), 32) };
        if count <= 0 {
            return Err("No native displays found".into());
        }
        Ok(buffer[..count as usize]
            .iter()
            .enumerate()
            .map(|(i, d)| Source {
                display: *d,
                label: format!(
                    "{} · {} × {} · ({}, {})",
                    i + 1,
                    d.width,
                    d.height,
                    d.x,
                    d.y
                ),
                av_index: Some(d.id),
            })
            .collect())
    })();
    match detected {
        Ok(sources) => result.sources = sources,
        Err(error) => result.error = Some(error),
    }
    result
}

#[cfg(test)]
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
