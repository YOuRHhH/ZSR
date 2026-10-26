//! Opt-in validation of the selected Windows display backend and MP4 timing.
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
    let folder = tempfile::tempdir().unwrap();
    let cap = zsr::platform::probe("ffmpeg");
    assert!(cap.error.is_none(), "{:?}", cap.error);
    let source = cap.sources.into_iter().next().expect("display");
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().into(),
            fps: 60,
            microphone: false,
            ..Default::default()
        },
        capture: Capture::Screen {
            source,
            region: Some(Region {
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
            _ => {}
        }
    };
    drop(session);
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
    assert_eq!(data["streams"][0]["width"], 320);
    assert_eq!(data["streams"][0]["height"], 180);
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
