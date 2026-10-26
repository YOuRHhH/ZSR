use eframe::egui;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// Opt-in UI timing harness; never modifies preferences or starts a recording.
pub(in crate::app) struct Performance {
    path: PathBuf,
    start: Instant,
    last: Instant,
    intervals: Vec<f64>,
    cpu: Vec<f32>,
    done: bool,
}

impl Performance {
    /// Enable a bounded timing run only when an explicit output path is supplied.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("ZSR_UI_PERFORMANCE").map(|path| Self {
            path: path.into(),
            start: Instant::now(),
            last: Instant::now(),
            intervals: Vec::new(),
            cpu: Vec::new(),
            done: false,
        })
    }

    /// Sample repaint intervals after warm-up, then save results and close this test instance.
    pub fn sample(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        if self.done {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.start);
        if elapsed > Duration::from_secs(2) && self.intervals.len() < 1024 {
            self.intervals
                .push(now.duration_since(self.last).as_secs_f64() * 1000.0);
            if let Some(cpu) = frame.info().cpu_usage {
                self.cpu.push(cpu * 1000.0);
            }
        }
        self.last = now;
        ctx.request_repaint_after(Duration::from_millis(16));
        if elapsed > Duration::from_secs(6) {
            self.done = true;
            std::fs::write(&self.path, serde_json::to_vec_pretty(&serde_json::json!({
                "frame_intervals_ms": self.intervals, "cpu_frame_ms": self.cpu,
                "transparent_baseline": std::env::var_os("ZSR_UI_TRANSPARENT_BASELINE").is_some(),
            })).unwrap()).expect("write UI performance report");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
