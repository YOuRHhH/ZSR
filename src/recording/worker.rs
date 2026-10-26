use super::{Event, Request};
use crate::platform;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::Stdio,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Failure = (String, Option<PathBuf>);
/// Run capture independently of UI polling and retain scheduling policy until FFmpeg exits.
pub(super) fn record(
    request: Request,
    stop: Receiver<()>,
    events: &Sender<Event>,
) -> Result<(), Failure> {
    let screen_input = super::desktop::input(&request).map_err(|error| (error, None))?;
    if let super::Capture::Window(window) = &request.capture {
        crate::windows::validate_target(window, &request.settings.ffmpeg)
            .map_err(|error| (error, None))?;
    }
    fs::create_dir_all(&request.settings.destination).map_err(|e| (e.to_string(), None))?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = format!(
        "ZSR_{}_{}",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"),
        unique
    );
    let partial = request
        .settings
        .destination
        .join(format!("{stem}.partial.mp4"));
    let final_path = request.settings.destination.join(format!("{stem}.mp4"));
    // Reserve the intermediate path exclusively before allowing FFmpeg to write it.
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(|e| (e.to_string(), None))?;
    let log_path = request.settings.destination.join(format!("{stem}.log"));
    let mut log = super::diagnostics::Diagnostics::create(&log_path).map_err(|e| {
        let _ = fs::remove_file(&partial);
        (e.to_string(), None)
    })?;
    let backend = screen_input
        .as_ref()
        .map(|i| i.graph.split('=').next().unwrap_or("unknown"))
        .unwrap_or(match request.capture {
            super::Capture::TestPattern => "testsrc2",
            super::Capture::Window(_) if cfg!(windows) => "gfxcapture",
            _ if cfg!(windows) => "gdigrab",
            _ if cfg!(target_os = "macos") => "avfoundation",
            _ => "x11grab",
        });
    log.write("capture", serde_json::json!({"backend": backend, "fps": request.settings.fps, "resolution": request.settings.resolution, "microphone": request.settings.microphone, "graph": screen_input.as_ref().map(|i| &i.graph), "device": screen_input.as_ref().and_then(|i| i.device.as_ref())}));
    let encoder = super::encoder::select(&request, &mut log);
    let power = super::power::RecordingPower::new(&mut log);
    let mut command = platform::command(&request.settings.ffmpeg);
    if let Some(device) = screen_input
        .as_ref()
        .and_then(|input| input.device.as_deref())
    {
        command.args([
            "-init_hw_device",
            device,
            "-filter_hw_device",
            "zsr_capture",
        ]);
    }
    let mut child = match command
        .args(request.arguments_with_encoder(
            &partial,
            screen_input.as_ref().map(|input| input.graph.as_str()),
            encoder,
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            let _ = fs::remove_file(&partial);
            log.write("spawn_failed", e.to_string());
            return Err((format!("{e}\nDiagnostics: {}", log_path.display()), None));
        }
    };
    log.write("encoder_started", serde_json::json!({"pid": child.id(), "options": encoder.arguments(request.settings.crf, request.settings.fps)}));
    power.protect_child(&child, &mut log);
    let mut stdin = child.stdin.take();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (progress_tx, progress_rx) = mpsc::channel();
    let progress_reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = progress_tx.send(line);
        }
    });
    let error_reader = thread::spawn(move || {
        let mut lines = std::collections::VecDeque::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if lines.len() >= 40 {
                lines.pop_front();
            }
            lines.push_back(line.chars().take(2000).collect::<String>());
        }
        lines.into_iter().collect::<Vec<_>>().join("\n")
    });
    let mut started = false;
    let mut frames = 0;
    let mut stopping = None;
    let launched = Instant::now();
    let mut forced_error = None;
    let status = loop {
        while let Ok(line) = progress_rx.try_recv() {
            log.progress(&line);
            if let Some(value) = line.strip_prefix("frame=") {
                frames = value.trim().parse().unwrap_or(frames);
                if frames > 0 && !started {
                    started = true;
                    let _ = events.send(Event::Started);
                }
            }
            if let Some(value) = line.strip_prefix("out_time_us=") {
                let seconds = value.trim().parse::<f64>().unwrap_or(0.0) / 1_000_000.0;
                let _ = events.send(Event::Progress { frames, seconds });
            }
        }
        if stopping.is_none()
            && (stop.try_recv().is_ok()
                || (!started && launched.elapsed() > Duration::from_secs(30)))
        {
            if !started && launched.elapsed() > Duration::from_secs(30) {
                forced_error = Some("Capture did not produce frames within 30 seconds".to_string());
            }
            if let Some(mut input) = stdin.take() {
                let _ = input.write_all(b"q\n");
                let _ = input.flush();
            }
            stopping = Some(Instant::now());
        }
        if stopping.is_some_and(|time| time.elapsed() > Duration::from_secs(15)) {
            forced_error = Some(
                "FFmpeg did not stop within 15 seconds; incomplete output was retained".into(),
            );
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                forced_error = Some(e.to_string());
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = progress_reader.join();
    // Collect any progress written immediately before process exit.
    for line in progress_rx.try_iter() {
        log.progress(&line);
        if let Some(value) = line.strip_prefix("frame=") {
            frames = value.trim().parse().unwrap_or(frames);
        }
    }
    let diagnostic = error_reader.join().unwrap_or_default();
    log.write("exit", serde_json::json!({"status": status.map(|s| s.to_string()), "error": forced_error, "stderr_tail": diagnostic}));
    if forced_error.is_some() || !status.is_some_and(|s| s.success()) || frames == 0 {
        let detail =
            forced_error.unwrap_or_else(|| "FFmpeg exited without a completed recording".into());
        let exists = fs::metadata(&partial).is_ok_and(|m| m.len() > 0);
        if !exists {
            let _ = fs::remove_file(&partial);
        }
        return Err((
            format!(
                "{detail}\n{diagnostic}\nDiagnostics: {}",
                log_path.display()
            ),
            exists.then_some(partial),
        ));
    }
    // Hard-link promotion is atomic and never overwrites an existing final file.
    // On filesystems without hard links, copy into an exclusively created file.
    if let Err(error) = fs::hard_link(&partial, &final_path) {
        if final_path.exists() {
            return Err((error.to_string(), Some(partial)));
        }
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&final_path)
            .map_err(|e| (e.to_string(), Some(partial.clone())))?;
        let promote = (|| -> std::io::Result<()> {
            let mut source = fs::File::open(&partial)?;
            std::io::copy(&mut source, &mut target)?;
            target.sync_all()
        })();
        drop(target);
        if let Err(e) = promote {
            // This path was created exclusively by us; retain only the original
            // intermediate if copying failed, never a misleading final file.
            let _ = fs::remove_file(&final_path);
            return Err((e.to_string(), Some(partial)));
        }
    }
    fs::remove_file(&partial).map_err(|e| (e.to_string(), Some(partial.clone())))?;
    let _ = events.send(Event::Saved(final_path));
    Ok(())
}
