use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

/// Choose whether enabled audio sources share a track or remain independently editable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioTracks {
    #[default]
    Mixed,
    Separate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: Language,
    pub destination: PathBuf,
    pub fps: u32,
    pub crf: u32,
    /// Prefer a verified hardware encoder on Windows, falling back to software.
    pub hardware_encoding: bool,
    pub resolution: crate::resolution::Resolution,
    pub cursor: bool,
    pub microphone: bool,
    pub desktop_audio: bool,
    pub audio_tracks: AudioTracks,
    /// Persistent discovered-device identity; resolved to backend input at start.
    pub audio_device: String,
    #[cfg(feature = "legacy-ffmpeg")]
    pub ffmpeg: String,
    /// Directory containing the private libobs runtime and ZSR helper.
    pub obs_runtime: String,
    pub start_shortcut: String,
    pub stop_shortcut: String,
}

impl Default for Settings {
    /// Supply defaults for new installations and fields absent from older settings files.
    fn default() -> Self {
        let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty()))
            .unwrap_or_default();
        Self {
            language: if locale.to_lowercase().starts_with("zh") {
                Language::Chinese
            } else {
                Language::English
            },
            destination: UserDirs::new()
                .and_then(|d| d.video_dir().map(|p| p.join("ZSR")))
                .unwrap_or_else(|| PathBuf::from("recordings")),
            fps: 30,
            crf: 23,
            hardware_encoding: true,
            resolution: Default::default(),
            cursor: true,
            microphone: false,
            desktop_audio: false,
            audio_tracks: AudioTracks::Mixed,
            audio_device: String::new(),
            #[cfg(feature = "legacy-ffmpeg")]
            ffmpeg: "ffmpeg".into(),
            obs_runtime: crate::obs_backend::default_runtime(),
            start_shortcut: "Ctrl+Shift+F9".into(),
            stop_shortcut: "Ctrl+Shift+F10".into(),
        }
    }
}

impl Settings {
    fn path() -> Result<PathBuf, String> {
        ProjectDirs::from("app", "ZSR", "ZSR")
            .map(|d| d.config_dir().join("settings.json"))
            .ok_or_else(|| "Cannot locate application configuration directory".into())
    }
    pub fn load() -> Result<Self, String> {
        let path = Self::path()?;
        match fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        // Replace the settings atomically; an interrupted write preserves the old file.
        let mut temporary =
            tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
        temporary.write_all(&bytes).map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        temporary.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        if ![30, 60].contains(&self.fps) {
            return Err("FPS must be 30 or 60".into());
        }
        if !(18..=30).contains(&self.crf) {
            return Err("Quality must be between CRF 18 and 30".into());
        }
        if self.destination.as_os_str().is_empty() {
            return Err("Output directory is required".into());
        }
        if self.microphone && self.audio_device.trim().is_empty() {
            return Err("An audio device is required".into());
        }
        Ok(())
    }
}
