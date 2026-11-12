use super::{Event, Request};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub struct Session {
    pub events: Receiver<Event>,
    stop: Sender<()>,
    ink: std::sync::Arc<std::sync::Mutex<Option<crate::brush::Ink>>>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Session {
    pub fn start(request: Request) -> Result<Self, String> {
        request.validate()?;
        let (events_tx, events) = mpsc::channel();
        let (stop, stop_rx) = mpsc::channel();
        // A single latest snapshot bounds memory when input outpaces the helper pipe.
        let ink = std::sync::Arc::new(std::sync::Mutex::new(None));
        let worker_ink = std::sync::Arc::clone(&ink);
        let worker = thread::Builder::new()
            .name("zsr-recorder".into())
            .spawn(move || {
                if let Err((detail, partial)) =
                    super::obs_worker::record(request, stop_rx, &events_tx, worker_ink)
                {
                    let _ = events_tx.send(Event::Failed { detail, partial });
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            events,
            stop,
            ink,
            worker: Some(worker),
        })
    }
    pub fn stop(&self) {
        let _ = self.stop.send(());
    }
    /// Queue validated ink without blocking the UI on helper I/O.
    pub fn set_ink(&self, ink: crate::brush::Ink) {
        if ink.valid() {
            *self.ink.lock().unwrap() = Some(ink);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        // The worker owns and reaps the OBS helper even if the UI closes unexpectedly.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
