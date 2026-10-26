use super::AudioDevice;

fn quoted(text: &str) -> Option<&str> {
    let first = text.find('"')?;
    let last = text.rfind('"')?;
    (last > first).then(|| &text[first + 1..last])
}

pub(super) fn parse(text: &str) -> Vec<AudioDevice> {
    let mut devices: Vec<AudioDevice> = Vec::new();
    let mut pending = None;
    let mut audio_section = false;
    for line in text.lines() {
        if line.contains("DirectShow audio devices") {
            audio_section = true;
            pending = None;
            continue;
        }
        if line.contains("DirectShow video devices") {
            audio_section = false;
            pending = None;
            continue;
        }
        if line.contains("Alternative name") {
            if let Some(index) = pending.take()
                && let Some(input) = quoted(line)
            {
                let device: &mut AudioDevice = &mut devices[index];
                device.id = input.into();
                device.input = input.into();
            }
        } else if line.contains("(video)") {
            pending = None;
        } else if (line.contains("(audio)") || audio_section)
            && let Some(name) = quoted(line)
        {
            devices.push(AudioDevice {
                id: name.into(),
                name: name.into(),
                input: name.into(),
            });
            pending = Some(devices.len() - 1);
        }
    }
    devices
}
