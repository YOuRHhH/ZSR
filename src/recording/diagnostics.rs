use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::Instant,
};

/// Per-recording diagnostics are bounded and written by the worker, never by the UI.
pub(super) struct Diagnostics {
    file: File,
    started: Instant,
    bytes: usize,
    progress: serde_json::Map<String, serde_json::Value>,
}

impl Diagnostics {
    /// Create a unique sidecar beside the movie so GUI builds retain useful diagnostics.
    pub fn create(path: &Path) -> std::io::Result<Self> {
        Ok(Self {
            file: OpenOptions::new().write(true).create_new(true).open(path)?,
            started: Instant::now(),
            bytes: 0,
            progress: Default::default(),
        })
    }

    /// Append one JSON record, capping disk usage at 4 MiB per recording.
    pub fn write(&mut self, event: &str, detail: impl serde::Serialize) {
        if self.bytes >= 4 * 1024 * 1024 {
            return;
        }
        let line = serde_json::json!({"wall_seconds": self.started.elapsed().as_secs_f64(), "event": event, "detail": detail}).to_string();
        if self.bytes + line.len() + 1 > 4 * 1024 * 1024 {
            return;
        }
        self.bytes += line.len() + 1;
        let _ = writeln!(self.file, "{line}");
    }

    /// Parse complete FFmpeg progress batches; unavailable values remain explicitly null.
    pub fn progress(&mut self, line: &str) {
        let Some((key, raw)) = line.split_once('=') else {
            return;
        };
        let raw = raw.trim();
        let value = match key {
            "frame" | "dup_frames" | "drop_frames" | "out_time_us" => {
                raw.parse::<u64>().ok().map(serde_json::Value::from)
            }
            "fps" | "speed" => raw
                .trim_end_matches('x')
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(serde_json::Value::from),
            "progress" => {
                let batch = std::mem::take(&mut self.progress);
                self.write("progress", batch);
                return;
            }
            _ => return,
        };
        self.progress
            .insert(key.into(), value.unwrap_or(serde_json::Value::Null));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Preserve missing speed, duplicate counts and the final batch without UI consumption.
    #[test]
    fn progress_batches_are_persisted() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("capture.log");
        let mut log = Diagnostics::create(&path).unwrap();
        for line in [
            "frame=60",
            "speed=N/A",
            "dup_frames=12",
            "drop_frames=3",
            "progress=continue",
            "speed=0.75x",
            "progress=end",
        ] {
            log.progress(line);
        }
        drop(log);
        let rows: Vec<serde_json::Value> = std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(rows[0]["detail"]["speed"].is_null());
        assert_eq!(rows[0]["detail"]["dup_frames"], 12);
        assert_eq!(rows[0]["detail"]["drop_frames"], 3);
        assert_eq!(rows[1]["detail"]["speed"], 0.75);
    }
}
