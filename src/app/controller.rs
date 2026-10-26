use super::{App, Background, CaptureMode, State};
use std::sync::mpsc;
use zsr::{
    audio,
    i18n::tr,
    platform,
    recording::{Capture, Event, Request, Session},
};

impl App {
    pub(in crate::app) fn probe(&mut self) {
        self.checking = true;
        let tx = self.tx.clone();
        let runtime = self.settings.obs_runtime.clone();
        std::thread::spawn(move || {
            let result = platform::probe(&runtime);
            let _ = tx.send(Background::Probe(result, runtime));
        });
        self.audio.refresh(&self.settings.obs_runtime);
    }

    pub(in crate::app) fn active(&self) -> bool {
        self.session.is_some()
    }

    pub(in crate::app) fn audio_ready(&self) -> bool {
        !self.settings.microphone
            || self
                .audio
                .ready(&self.settings.obs_runtime, &self.settings.audio_device)
    }

    /// Apply background results and refresh the file list after recording finalization.
    pub(in crate::app) fn poll(&mut self) {
        self.audio
            .poll(&self.settings.obs_runtime, &mut self.settings.audio_device);
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Background::Probe(result, runtime) => {
                    self.capabilities = Some(result);
                    self.probed_runtime = runtime;
                    self.checking = false;
                    self.display = 0;
                }
                Background::Folder(path) => self.settings.destination = path,
            }
        }
        let mut finished = false;
        if let Some(session) = &self.session {
            loop {
                let event = match session.events.try_recv() {
                    Ok(event) => event,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !finished {
                            self.state = State::Failed;
                            self.detail = "Recording worker ended unexpectedly".into();
                            finished = true;
                        }
                        break;
                    }
                };
                match event {
                    Event::Started => {
                        if self.state != State::Stopping {
                            self.state = State::Recording;
                        }
                    }
                    Event::Progress { frames, seconds } => {
                        self.frames = frames;
                        self.seconds = seconds.max(0.0);
                    }
                    Event::Saved(path) => {
                        self.recordings.refresh();
                        self.saved = Some(path);
                        self.state = State::Saved;
                        finished = true;
                    }
                    Event::Failed { detail, partial } => {
                        self.detail = detail;
                        if let Some(path) = partial {
                            self.detail
                                .push_str(&format!("\nIncomplete file: {}", path.display()));
                        }
                        self.state = State::Failed;
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.session = None;
        }
    }

    pub(in crate::app) fn start(&mut self) {
        if self.picker.is_some() {
            return;
        }
        if !self.audio_ready() {
            self.detail = tr(self.settings.language, "audio_missing").into();
            self.state = State::Failed;
            return;
        }
        let capture = if self.capture_mode == CaptureMode::Window {
            let Some(window) = self
                .windows
                .selected(self.selected_window, &self.settings.obs_runtime)
            else {
                self.detail = tr(self.settings.language, "window_missing").into();
                self.state = State::Failed;
                return;
            };
            Capture::Window(window.clone())
        } else {
            let Some(source) = self
                .capabilities
                .as_ref()
                .and_then(|c| c.sources.get(self.display))
                .cloned()
            else {
                return;
            };
            Capture::Screen {
                source,
                region: (self.capture_mode == CaptureMode::Region).then_some(self.region),
            }
        };
        let mut settings = self.settings.clone();
        if settings.microphone {
            // Resolve the current recording identifier at start, leaving the
            // persistent identity unchanged (macOS indices may move on refresh).
            settings.audio_device = audio::selected(self.audio.devices(), &settings.audio_device)
                .expect("audio_ready validated the selection")
                .input
                .clone();
        }
        let request = Request { settings, capture };
        match Session::start(request) {
            Ok(session) => {
                self.session = Some(session);
                self.state = State::Preparing;
                self.saved = None;
                self.detail.clear();
                self.frames = 0;
                self.seconds = 0.0;
            }
            Err(error) => {
                self.detail = error;
                self.state = State::Failed;
            }
        }
    }
}
