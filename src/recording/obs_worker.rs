use super::{Event, Request};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::Stdio,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
/// Supervise the isolated OBS engine and preserve incomplete files on any engine failure.
pub(super) fn record(
    request: Request,
    stop: Receiver<()>,
    events: &Sender<Event>,
) -> Result<(), (String, Option<PathBuf>)> {
    let root = crate::obs_backend::resolve(&request.settings.obs_runtime).map_err(|e| (e, None))?;
    fs::create_dir_all(&request.settings.destination).map_err(|e| (e.to_string(), None))?;
    let folder = request
        .settings
        .destination
        .canonicalize()
        .map_err(|e| (e.to_string(), None))?;
    let stem = format!(
        "ZSR_{}_{}",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let partial = folder.join(format!("{stem}.partial.mp4"));
    let final_path = folder.join(format!("{stem}.mp4"));
    let log_path = folder.join(format!("{stem}.log"));
    let mut log = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log_path)
        .map_err(|e| (e.to_string(), None))?;
    let mut child = crate::obs_backend::command(&root)
        .map_err(|e| (e, None))?
        .arg("record")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| (e.to_string(), None))?;
    let mut input = child.stdin.take().unwrap();
    let init=serde_json::json!({"request":request,"output":partial.to_string_lossy().trim_start_matches(r"\\?\")}).to_string();
    if writeln!(input, "{init}").is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(("OBS helper input failed".into(), None));
    }
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    let errors = thread::spawn(move || {
        let mut tail = std::collections::VecDeque::new();
        let mut bytes = 0;
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if bytes < 4 * 1024 * 1024 {
                let _ = writeln!(log, "{line}");
                bytes += line.len() + 1;
            }
            if tail.len() == 25 {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        tail.into_iter().collect::<Vec<_>>().join("\n")
    });
    let launched = Instant::now();
    let mut started = false;
    let mut complete = false;
    let mut stopping = None;
    let mut failure = None;
    let status = loop {
        while let Ok(line) = rx.try_recv() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                match v["event"].as_str() {
                    Some("started") => {
                        started = true;
                        let _ = events.send(Event::Started);
                    }
                    Some("progress") => {
                        let _ = events.send(Event::Progress {
                            frames: v["frames"].as_u64().unwrap_or(0),
                            seconds: v["seconds"].as_f64().unwrap_or(0.0),
                        });
                    }
                    Some("complete") => complete = true,
                    Some("error") => {
                        failure = Some(v["detail"].as_str().unwrap_or("OBS error").to_string())
                    }
                    _ => {}
                }
            }
        }
        if stopping.is_none()
            && (stop.try_recv().is_ok()
                || (!started && launched.elapsed() > Duration::from_secs(40)))
        {
            let _ = writeln!(input, "stop");
            let _ = input.flush();
            stopping = Some(Instant::now());
        }
        if stopping.is_some_and(|t| t.elapsed() > Duration::from_secs(20)) {
            failure = Some("OBS shutdown timed out".into());
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                failure = Some(e.to_string());
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = reader.join();
    for line in rx.try_iter() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if v["event"] == "complete" {
                complete = true;
            }
            if v["event"] == "error" {
                failure = Some(v["detail"].as_str().unwrap_or("OBS error").into());
            }
        }
    }
    let tail = errors.join().unwrap_or_default();
    if failure.is_some() || !complete || !status.is_some_and(|s| s.success()) {
        return Err((
            format!(
                "{}\n{}\nLog: {}",
                failure.unwrap_or_else(|| "OBS recording did not complete".into()),
                tail,
                log_path.display()
            ),
            partial.is_file().then_some(partial),
        ));
    }
    // Exclusive copy avoids overwriting any existing movie; retain the original on failure.
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&final_path)
        .map_err(|e| (e.to_string(), Some(partial.clone())))?;
    let promoted = (|| -> std::io::Result<()> {
        let mut source = fs::File::open(&partial)?;
        std::io::copy(&mut source, &mut target)?;
        target.sync_all()
    })();
    drop(target);
    if let Err(e) = promoted {
        let _ = fs::remove_file(&final_path);
        return Err((e.to_string(), Some(partial)));
    }
    fs::remove_file(&partial).map_err(|e| (e.to_string(), Some(partial)))?;
    let _ = events.send(Event::Saved(final_path));
    Ok(())
}
