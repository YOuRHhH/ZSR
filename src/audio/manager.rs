use super::{AudioDevice, discover, reconcile, selected};
use std::{
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

type Discovery = (Result<Vec<AudioDevice>, String>, String);

/// Owns asynchronous device discovery; neither UI nor capture code runs probes.
pub struct AudioDevices {
    devices: Vec<AudioDevice>,
    error: Option<String>,
    checking: bool,
    executable: String,
    pending: Option<Receiver<Discovery>>,
    last_check: Instant,
}

impl Default for AudioDevices {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            error: None,
            checking: false,
            executable: String::new(),
            pending: None,
            last_check: Instant::now(),
        }
    }
}

impl AudioDevices {
    pub fn devices(&self) -> &[AudioDevice] {
        &self.devices
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn is_refreshing(&self) -> bool {
        self.checking
    }

    pub fn refresh(&mut self, executable: &str) {
        if self.checking {
            return;
        }
        self.checking = true;
        self.last_check = Instant::now();
        let executable = executable.to_owned();
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        std::thread::spawn(move || {
            let result = discover(&executable);
            let _ = tx.send((result, executable));
        });
    }

    pub fn refresh_if_due(&mut self, executable: &str) {
        if self.last_check.elapsed() >= Duration::from_secs(15) {
            self.refresh(executable);
        }
    }

    pub fn poll(&mut self, executable: &str, saved: &mut String) {
        let Some(rx) = &self.pending else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => (
                Err("Audio discovery worker ended unexpectedly".into()),
                executable.into(),
            ),
        };
        self.pending = None;
        self.checking = false;
        self.executable = result.1;
        if self.executable != executable {
            // A new executable was selected while an old probe was in flight.
            self.devices.clear();
            self.refresh(executable);
            return;
        }
        match result.0 {
            Ok(devices) => {
                reconcile(&devices, saved);
                self.devices = devices;
                self.error = None;
            }
            Err(error) => {
                self.devices.clear();
                self.error = Some(error);
            }
        }
    }

    pub fn ready(&self, executable: &str, saved: &str) -> bool {
        !self.checking
            && self.error.is_none()
            && self.executable == executable
            && selected(&self.devices, saved).is_some()
    }
}
