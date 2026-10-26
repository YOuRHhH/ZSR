//! Enumerate inputs using the same backend that will actually record them.
#[cfg(test)]
mod avfoundation;
#[cfg(test)]
mod dshow;
mod manager;
#[cfg(test)]
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

/// Enumerate microphones using the exact OBS plugin that records them.
pub fn discover(runtime: &str) -> Result<Vec<AudioDevice>, String> {
    let result = crate::obs_backend::probe(runtime)?;
    Ok(result["audio"]
        .as_array()
        .ok_or("OBS returned no audio device list")?
        .iter()
        .filter_map(|d| {
            let id = d["id"].as_str()?.to_owned();
            Some(AudioDevice {
                id: id.clone(),
                name: d["name"].as_str()?.to_owned(),
                input: id,
            })
        })
        .collect())
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
