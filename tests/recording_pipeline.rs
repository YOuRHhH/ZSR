//! Worker lifecycle tests; media generation is opt-in and never captures the desktop.
use std::{path::PathBuf, time::Duration};
use zsr::{
    recording::{Capture, Event, Request, Session},
    settings::Settings,
};

/// Capture must keep advancing even when a minimized UI does not consume progress events.
#[test]
#[ignore = "requires FFmpeg and ffprobe; records only a generated test pattern"]
fn capture_continues_without_ui_polling() {
    let folder = tempfile::tempdir().unwrap();
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().to_owned(),
            fps: 30,
            ..Settings::default()
        },
        capture: Capture::TestPattern,
    })
    .unwrap();
    loop {
        match session
            .events
            .recv_timeout(Duration::from_secs(40))
            .unwrap()
        {
            Event::Started => break,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    }
    // No UI consumption or repaint is required for either capture or stop handling.
    std::thread::sleep(Duration::from_secs(4));
    session.stop();
    let saved = loop {
        match session
            .events
            .recv_timeout(Duration::from_secs(40))
            .unwrap()
        {
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    };
    drop(session);
    let output = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(saved)
        .args(["-map", "0:v:0", "-f", "framemd5", "-"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let hashes: std::collections::HashSet<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.rsplit(',').next())
        .map(|hash| hash.trim().to_owned())
        .collect();
    assert!(
        hashes.len() >= 100,
        "Expected advancing frames while UI was idle; got {}",
        hashes.len()
    );
}

#[test]
fn missing_encoder_reports_failure_without_leaving_empty_files() {
    let directory = tempfile::tempdir().unwrap();
    let settings = Settings {
        destination: directory.path().join("output"),
        ffmpeg: directory
            .path()
            .join("missing-ffmpeg")
            .to_string_lossy()
            .into_owned(),
        ..Settings::default()
    };
    let session = Session::start(Request {
        settings,
        capture: Capture::TestPattern,
    })
    .unwrap();
    match session.events.recv_timeout(Duration::from_secs(5)).unwrap() {
        Event::Failed { partial, .. } => assert!(partial.is_none()),
        event => panic!("expected failure, got {event:?}"),
    }
    drop(session);
    assert_eq!(
        std::fs::read_dir(directory.path().join("output"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
#[ignore = "requires FFmpeg and ffprobe with libx264; run cargo test --test recording_pipeline -- --ignored"]
fn synthetic_video_is_finalized_and_decodable() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("zsr-test-{}-{unique}", std::process::id()));
    let settings = Settings {
        destination: dir.clone(),
        ..Settings::default()
    };
    let session = Session::start(Request {
        settings,
        capture: Capture::TestPattern,
    })
    .unwrap();
    let mut stopped = false;
    let saved: PathBuf = loop {
        match session
            .events
            .recv_timeout(Duration::from_secs(40))
            .expect("recorder response")
        {
            Event::Progress { seconds, .. } if seconds >= 2.0 && !stopped => {
                session.stop();
                stopped = true;
            }
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    };
    drop(session);
    assert!(saved.exists());
    let probe = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,width,height",
            "-of",
            "json",
        ])
        .arg(&saved)
        .output()
        .unwrap();
    assert!(probe.status.success());
    let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(info["streams"][0]["codec_name"], "h264");
    assert_eq!(info["streams"][0]["width"], 640);
    let decode = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&saved)
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    assert!(
        decode.status.success(),
        "{}",
        String::from_utf8_lossy(&decode.stderr)
    );
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "no partial files left after save"
    );
    std::fs::remove_file(saved).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
