use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

#[derive(Default)]
pub(super) struct FileOpener {
    pending: Option<Receiver<Result<(), String>>>,
    pub error: Option<String>,
}

impl FileOpener {
    /// Report an outstanding OS launch request so repeated clicks cannot pile up workers.
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Launch Explorer or the default player outside the UI thread and wake the UI on completion.
    pub fn open(&mut self, path: PathBuf, ctx: &eframe::egui::Context) {
        let ctx = ctx.clone();
        self.launch(move || {
            let result =
                open::that_detached(&path).map_err(|error| format!("{}: {error}", path.display()));
            ctx.request_repaint();
            result
        });
    }

    /// Run one fallible launcher operation without waiting for it in the event loop.
    fn launch(&mut self, action: impl FnOnce() -> Result<(), String> + Send + 'static) {
        if self.busy() {
            return;
        }
        self.error = None;
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("zsr-open-file".into())
            .spawn(move || {
                let _ = tx.send(action());
            }) {
            Ok(_) => self.pending = Some(rx),
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Collect launch errors without waiting for Explorer, the player, or a slow shell extension.
    pub fn poll(&mut self) {
        let Some(receiver) = &self.pending else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.error = result.err();
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.error = Some("File launcher stopped unexpectedly".into());
                self.pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blocked shell must not block UI polling, and its eventual error must remain visible.
    #[test]
    fn slow_launcher_is_nonblocking_and_reports_errors() {
        let (release, wait) = mpsc::channel();
        let mut opener = FileOpener::default();
        opener.launch(move || {
            wait.recv().unwrap();
            Err("fixture launch failure".into())
        });
        opener.poll();
        assert!(opener.busy());
        opener.launch(|| panic!("duplicate request must not run"));
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while opener.busy() && std::time::Instant::now() < deadline {
            opener.poll();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!opener.busy());
        assert_eq!(opener.error.as_deref(), Some("fixture launch failure"));
    }
}
