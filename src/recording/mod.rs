use crate::{platform::Source, settings::Settings};
use std::path::PathBuf;
mod command;
mod desktop;
mod power;
mod session;
mod validation;
mod worker;
pub use session::Session;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub enum Capture {
    Window(crate::windows::AppWindow),
    Screen {
        source: Source,
        region: Option<Region>,
    },
    TestPattern,
}

#[derive(Clone, Debug)]
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
