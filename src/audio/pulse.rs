use super::AudioDevice;

pub(super) fn parse(text: &str) -> Result<Vec<AudioDevice>, String> {
    let sources: Vec<serde_json::Value> = serde_json::from_str(text)
        .map_err(|e| format!("Cannot read PulseAudio device list: {e}\n{text}"))?;
    Ok(sources
        .into_iter()
        .filter_map(|source| {
            let name = source.get("name")?.as_str()?;
            let monitor = source.get("monitor_of_sink");
            if name.ends_with(".monitor")
                || source["properties"]["device.class"] == "monitor"
                || monitor.is_some_and(|v| {
                    !v.is_null() && v.as_u64() != Some(u32::MAX as u64) && v.as_str() != Some("n/a")
                })
            {
                return None;
            }
            Some(AudioDevice {
                id: name.into(),
                name: source
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(name)
                    .into(),
                input: name.into(),
            })
        })
        .collect())
}
