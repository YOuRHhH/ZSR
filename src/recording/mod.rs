use crate::{platform::Source, settings::Settings};
use std::path::PathBuf;
#[cfg(feature = "legacy-ffmpeg")]
mod command;
#[cfg(feature = "legacy-ffmpeg")]
#[allow(dead_code)]
mod desktop;
#[cfg(feature = "legacy-ffmpeg")]
#[allow(dead_code)]
mod diagnostics;
#[cfg(feature = "legacy-ffmpeg")]
#[allow(dead_code)]
mod encoder;
mod obs_worker;
#[cfg(feature = "legacy-ffmpeg")]
#[allow(dead_code)]
mod power;
mod session;
mod validation;
#[cfg(feature = "legacy-ffmpeg")]
#[allow(dead_code)]
mod worker;
pub use session::Session;
#[cfg(all(test, feature = "legacy-ffmpeg"))]
mod tests;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Capture {
    Window(crate::windows::AppWindow),
    Screen {
        source: Source,
        region: Option<Region>,
    },
    TestPattern,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub settings: Settings,
    pub capture: Capture,
}

#[derive(Debug)]
pub enum Event {
    Started,
    Progress {
        frames: u64,
        seconds: f64,
    },
    Saved(PathBuf),
    Failed {
        detail: String,
        partial: Option<PathBuf>,
    },
}
