//! Bounded, memory-only preview frames from an isolated OBS helper.
use crate::recording::Request;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::Stdio,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

pub const MAX_WIDTH: u32 = 640;
pub const MAX_HEIGHT: u32 = 360;

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
impl Frame {
    /// Validate dimensions before allocating or reading helper-controlled pixel data.
    pub fn read(reader: &mut impl Read) -> Result<Self, String> {
        let mut header = [0; 12];
        reader.read_exact(&mut header).map_err(|e| e.to_string())?;
        // Startup errors still use the helper's JSON event protocol; keep the actionable detail.
        if header[0] == b'{' {
            let mut message = header.to_vec();
            BufReader::new(reader.take(16 * 1024))
                .read_until(b'\n', &mut message)
                .map_err(|e| e.to_string())?;
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&message)
                && let Some(detail) = value["detail"].as_str()
            {
                return Err(detail.into());
            }
            return Err("Invalid preview error response".into());
        }
        let width = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let height = u32::from_le_bytes(header[8..12].try_into().unwrap());
        if &header[..4] != b"ZSRP"
            || !(2..=MAX_WIDTH).contains(&width)
            || !(2..=MAX_HEIGHT).contains(&height)
        {
            return Err("Invalid preview stream; rebuild and stage the OBS helper.".into());
        }
        let mut rgba = vec![0; width as usize * height as usize * 4];
        reader.read_exact(&mut rgba).map_err(|e| e.to_string())?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// Write one RGBA frame; preview mode creates no recording or media file.
    pub fn write(&self, writer: &mut impl Write) -> std::io::Result<()> {
        writer.write_all(b"ZSRP")?;
        writer.write_all(&self.width.to_le_bytes())?;
        writer.write_all(&self.height.to_le_bytes())?;
        writer.write_all(&self.rgba)?;
        writer.flush()
    }
}

type Latest = Arc<Mutex<Option<Result<Frame, String>>>>;
pub struct Session {
    latest: Latest,
    stop: mpsc::Sender<()>,
}
impl Session {
    /// Start capture on a worker; retain only the newest frame so UI stalls cannot queue video.
    pub fn start(request: Request) -> Result<Self, String> {
        request.validate()?;
        let latest = Arc::new(Mutex::new(None));
        let output = latest.clone();
        let (stop, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("zsr-preview".into())
            .spawn(move || {
                if let Err(error) = run(request, receiver, &output) {
                    *output.lock().unwrap() = Some(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { latest, stop })
    }

    /// Take the newest frame or terminal error without waiting for capture.
    pub fn take_frame(&self) -> Option<Result<Frame, String>> {
        self.latest.lock().unwrap().take()
    }
}
impl Drop for Session {
    /// Cleanup belongs to the supervisor, so stopping preview never blocks the UI.
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

/// Reap the child on cancellation, EOF, timeout or startup failure.
fn run(request: Request, stop: mpsc::Receiver<()>, latest: &Latest) -> Result<(), String> {
    if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
        return Ok(());
    }
    let root = crate::obs_backend::resolve(&request.settings.obs_runtime)?;
    let mut child = crate::obs_backend::command(&root)?
        .arg("preview")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut input = child.stdin.take().unwrap();
    let message = serde_json::json!({"request":request}).to_string() + "\n";
    if let Err(error) = input.write_all(message.as_bytes()) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error.to_string());
    }
    let mut stdout = child.stdout.take().unwrap();
    let (done_tx, done) = mpsc::channel();
    let last_frame = Arc::new(Mutex::new(Instant::now()));
    let last_read = last_frame.clone();
    let frames = latest.clone();
    let reader = std::thread::spawn(move || {
        loop {
            match Frame::read(&mut stdout) {
                Ok(frame) => {
                    *last_read.lock().unwrap() = Instant::now();
                    *frames.lock().unwrap() = Some(Ok(frame));
                }
                Err(error) => {
                    let _ = done_tx.send(error);
                    break;
                }
            }
        }
    });
    let result = loop {
        if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
            break Ok(());
        }
        if let Ok(error) = done.try_recv() {
            break Err(error);
        }
        if last_frame.lock().unwrap().elapsed() > Duration::from_secs(30) {
            break Err("Preview timed out; check the selected source and OBS runtime.".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // EOF requests graceful teardown; a hung capture backend is terminated after a bounded wait.
    drop(input);
    let deadline = Instant::now() + Duration::from_millis(700);
    while matches!(child.try_wait(), Ok(None)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unbounded_and_truncated_frames() {
        let mut bytes = b"ZSRP".to_vec();
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.extend(2u32.to_le_bytes());
        assert!(Frame::read(&mut bytes.as_slice()).is_err());
        let frame = Frame {
            width: 2,
            height: 2,
            rgba: vec![123; 16],
        };
        let mut bytes = Vec::new();
        frame.write(&mut bytes).unwrap();
        assert_eq!(Frame::read(&mut bytes.as_slice()).unwrap().rgba, frame.rgba);
        bytes.pop();
        assert!(Frame::read(&mut bytes.as_slice()).is_err());
    }

    #[test]
    fn preserves_native_startup_errors() {
        let bytes = b"{\"event\":\"error\",\"detail\":\"Selected window is closed\"}\n";
        assert_eq!(
            Frame::read(&mut bytes.as_slice()).err().unwrap(),
            "Selected window is closed"
        );
    }
}
