use super::{Event, Request};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

pub struct Session {
    pub events: Receiver<Event>,
    stop: Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Session {
    pub fn start(request: Request) -> Result<Self, String> {
        request.validate()?;
        let (events_tx, events) = mpsc::channel();
        let (stop, stop_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("zsr-recorder".into())
            .spawn(move || {
                if let Err((detail, partial)) =
                    super::obs_worker::record(request, stop_rx, &events_tx)
                {
                    let _ = events_tx.send(Event::Failed { detail, partial });
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            events,
            stop,
            worker: Some(worker),
        })
    }
    pub fn stop(&self) {
        let _ = self.stop.send(());
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
