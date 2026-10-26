//! Opt-in validation of the selected Windows display backend and MP4 timing.
#![cfg(feature = "legacy-ffmpeg")]
#![cfg(windows)]
use std::time::{Duration, Instant};
use zsr::{
    recording::{Capture, Event, Region, Request, Session},
    settings::Settings,
};

/// Exercise real monitor mapping, region coordinates, capture startup and decodable output.
#[test]
#[ignore = "captures a 320x180 desktop region for two seconds; requires Windows and FFmpeg"]
fn selected_display_region_is_finalized() {
    for hardware in [true, false] {
        verify_capture(hardware, false);
    }
}

/// Verify both automatic hardware selection and explicit software capture, including sidecars.
fn verify_capture(hardware: bool, full_display: bool) {
    let folder = tempfile::tempdir().unwrap();
    let cap = zsr::platform::probe("ffmpeg");
    assert!(cap.error.is_none(), "{:?}", cap.error);
    let source = cap.sources.into_iter().next().expect("display");
    let (width, height) = if full_display {
        (source.display.width / 2 * 2, source.display.height / 2 * 2)
    } else {
        (320, 180)
    };
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().into(),
            fps: 60,
            microphone: false,
            hardware_encoding: hardware,
            ..Default::default()
        },
        capture: Capture::Screen {
            source,
            region: (!full_display).then_some(Region {
                x: 0,
                y: 0,
                width: 320,
                height: 180,
            }),
        },
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let saved = loop {
        assert!(Instant::now() < deadline, "capture timed out");
        match session
            .events
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
        {
            Event::Progress { seconds, .. } if seconds >= 2.0 => session.stop(),
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            Event::Started if full_display => {
                // Simulate suspended UI event consumption while the full-size worker runs.
                std::thread::sleep(Duration::from_secs(5));
                session.stop();
            }
            _ => {}
        }
    };
    drop(session);
    let log = std::fs::read_to_string(saved.with_extension("log")).unwrap();
    let rows: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let backend = rows.iter().find(|r| r["event"] == "capture").unwrap();
    let encoder = rows
        .iter()
        .find(|r| r["event"] == "encoder_selected")
        .unwrap();
    let last = rows.iter().rfind(|r| r["event"] == "progress").unwrap();
    assert!(last["detail"]["frame"].as_u64().unwrap() > 0);
    for key in ["speed", "dup_frames", "drop_frames"] {
        assert!(last["detail"].get(key).is_some(), "missing {key}");
    }
    assert!(rows.iter().any(|r| r["event"] == "child_priority"));
    if !hardware {
        assert_eq!(encoder["detail"], "libx264");
    }
    println!(
        "hardware={hardware}: backend={}, encoder={}, final_progress={}",
        backend["detail"]["backend"], encoder["detail"], last["detail"]
    );
    let output = zsr::platform::command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,duration",
            "-of",
            "json",
        ])
        .arg(&saved)
        .output()
        .unwrap();
    assert!(output.status.success());
    let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(data["streams"][0]["width"], width);
    assert_eq!(data["streams"][0]["height"], height);
    assert!(
        data["streams"][0]["duration"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
            >= 2.0
    );
    let decode = zsr::platform::command("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(saved)
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    assert!(
        decode.status.success(),
        "{}",
        String::from_utf8_lossy(&decode.stderr)
    );
}

/// Exercise full-resolution capture without a UI consuming progress for five seconds.
#[test]
#[ignore = "captures the display with hardware and software encoding; requires Windows and FFmpeg"]
fn display_capture_without_ui_polling() {
    for hardware in [true, false] {
        verify_capture(hardware, true);
    }
}
