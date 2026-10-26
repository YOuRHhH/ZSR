use super::AppWindow;
use std::{
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
type Discovery = (String, Result<Vec<AppWindow>, String>);
#[derive(Default)]
pub struct WindowInventory {
    pub windows: Vec<AppWindow>,
    pub error: Option<String>,
    rx: Option<Receiver<Discovery>>,
    checked: String,
    refreshed: Option<Instant>,
}

impl WindowInventory {
    pub fn loading(&self) -> bool {
        self.rx.is_some()
    }
    pub fn selected(&self, id: Option<(u64, u32)>, ffmpeg: &str) -> Option<&AppWindow> {
        if self.checked != ffmpeg || self.error.is_some() {
            return None;
        }
        self.windows.iter().find(|w| Some(w.identity()) == id)
    }
    pub fn refresh_due(&mut self, ffmpeg: &str) {
        if self.checked != ffmpeg
            || self
                .refreshed
                .is_none_or(|t| t.elapsed() > Duration::from_secs(5))
        {
            self.refresh(ffmpeg);
        }
    }
    pub fn refresh(&mut self, ffmpeg: &str) {
        if self.loading() || !super::supported() {
            return;
        }
        let ffmpeg = ffmpeg.to_owned();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let result = super::check_backend(&ffmpeg).and_then(|_| super::discover());
            let _ = tx.send((ffmpeg, result));
        });
    }
    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                (String::new(), Err("Window discovery failed.".into()))
            }
        };
        self.rx = None;
        self.refreshed = Some(Instant::now());
        self.checked = result.0;
        match result.1 {
            Ok(mut windows) => {
                windows.sort_by_key(|w| w.title.to_lowercase());
                self.windows = windows;
                self.error = None;
            }
            Err(error) => {
                self.windows.clear();
                self.error = Some(error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_or_reused_window_is_not_silently_replaced() {
        let target = AppWindow {
            id: 10,
            pid: 1,
            title: "Editor".into(),
            app: "edit".into(),
        };
        let mut inventory = WindowInventory {
            checked: "ffmpeg".into(),
            windows: vec![target.clone()],
            ..Default::default()
        };
        assert!(
            inventory
                .selected(Some(target.identity()), "ffmpeg")
                .is_some()
        );
        inventory.windows[0].title = "Renamed document".into();
        assert!(
            inventory
                .selected(Some(target.identity()), "ffmpeg")
                .is_some()
        );
        inventory.windows[0].pid = 2;
        assert!(
            inventory
                .selected(Some(target.identity()), "ffmpeg")
                .is_none()
        );
        assert!(inventory.selected(Some((10, 2)), "other-ffmpeg").is_none());
        inventory.windows.clear();
        assert!(
            inventory
                .selected(Some(target.identity()), "ffmpeg")
                .is_none()
        );
    }
}
