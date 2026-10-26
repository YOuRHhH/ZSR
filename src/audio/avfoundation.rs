use super::AudioDevice;

pub(super) fn parse(text: &str) -> Vec<AudioDevice> {
    let mut audio_section = false;
    let mut devices: Vec<AudioDevice> = Vec::new();
    let mut counts = std::collections::HashMap::new();
    for line in text.lines() {
        if line.contains("AVFoundation audio devices:") {
            audio_section = true;
            continue;
        }
        if line.contains("AVFoundation video devices:") {
            audio_section = false;
            continue;
        }
        if !audio_section {
            continue;
        }
        let Some((_, content)) = line.split_once("] ") else {
            continue;
        };
        let Some((index, name)) = content
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.split_once("] "))
        else {
            continue;
        };
        if index.parse::<u32>().is_err() || name.trim().is_empty() {
            continue;
        }
        let name = name.trim();
        let count = counts.entry(name.to_owned()).or_insert(0);
        devices.push(AudioDevice {
            id: format!("avfoundation:{name}:{count}"),
            name: name.into(),
            input: index.into(),
        });
        *count += 1;
    }
    devices
}
