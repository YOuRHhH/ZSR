//! Enumerate inputs using the same backend that will actually record them.
use crate::platform::{command, output};
use std::time::Duration;
mod avfoundation;
mod dshow;
mod manager;
mod pulse;
pub use manager::AudioDevices;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDevice {
    /// Persisted identity, separate from the transient AVFoundation device index.
    pub id: String,
    pub name: String,
    pub input: String,
}

pub fn discover(ffmpeg: &str) -> Result<Vec<AudioDevice>, String> {
    let timeout = Duration::from_secs(8);
    if cfg!(windows) {
        let text = output(
            command(ffmpeg).args([
                "-hide_banner",
                "-list_devices",
                "true",
                "-f",
                "dshow",
                "-i",
                "dummy",
            ]),
            timeout,
        )?;
        // FFmpeg intentionally exits with failure after listing devices.
        if text.contains("Unknown input format") || text.contains("Unrecognized option") {
            return Err(text);
        }
        if !text.contains("(audio)") && !text.contains("(video)") && !text.contains("DirectShow") {
            return Err(text);
        }
        Ok(dshow::parse(&text))
    } else if cfg!(target_os = "macos") {
        let text = output(
            command(ffmpeg).args([
                "-hide_banner",
                "-f",
                "avfoundation",
                "-list_devices",
                "true",
                "-i",
                "",
            ]),
            timeout,
        )?;
        if !text.contains("AVFoundation audio devices:") {
            return Err(text);
        }
        Ok(avfoundation::parse(&text))
    } else if cfg!(target_os = "linux") {
        // pactl also works with PipeWire's PulseAudio compatibility server.
        let text = output(
            command("pactl").args(["--format=json", "list", "sources"]),
            timeout,
        )
        .map_err(|e| format!("Cannot enumerate PulseAudio inputs with pactl: {e}"))?;
        pulse::parse(&text)
    } else {
        Err("Audio device discovery is unavailable on this platform".into())
    }
}

/// Match persisted identities; legacy manually entered names/indices remain usable
/// only when exactly one current device matches. Never replace a missing selection.
pub fn selected<'a>(devices: &'a [AudioDevice], saved: &str) -> Option<&'a AudioDevice> {
    if let Some(device) = devices.iter().find(|device| device.id == saved) {
        return Some(device);
    }
    let mut matches = devices
        .iter()
        .filter(|device| device.name == saved || device.input == saved);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub fn reconcile(devices: &[AudioDevice], saved: &mut String) {
    if saved.is_empty() {
        if let Some(device) = devices.first() {
            *saved = device.id.clone();
        }
    } else if let Some(device) = selected(devices, saved) {
        *saved = device.id.clone();
    }
}
