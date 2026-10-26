//! Opt-in regression for compositor capture running alongside the configured microphone.
#![cfg(feature = "legacy-ffmpeg")]
#![cfg(windows)]
use std::time::{Duration, Instant};
use zsr::{
    recording::{Capture, Event, Request, Session},
    settings::Settings,
};

/// Catch the former audio-enabled pipeline that filled most output frames with CFR duplicates.
#[test]
#[ignore = "records the display and configured microphone for six seconds; removes its temporary output"]
fn microphone_does_not_starve_desktop_capture() {
    for hardware in [true, false] {
        verify_microphone_capture(hardware);
    }
}

/// Check cadence and A/V duration for both automatic hardware and software encoding.
fn verify_microphone_capture(hardware: bool) {
    let folder = tempfile::tempdir().unwrap();
    let saved_settings = Settings::load().unwrap();
    assert!(
        !saved_settings.audio_device.is_empty(),
        "configure a microphone first"
    );
    let cap = zsr::platform::probe(&saved_settings.ffmpeg);
    assert!(cap.error.is_none(), "{:?}", cap.error);
    let source = cap.sources.into_iter().next().expect("display");
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().into(),
            fps: 60,
            microphone: true,
            hardware_encoding: hardware,
            resolution: zsr::resolution::Resolution::Original,
            ..saved_settings.clone()
        },
        capture: Capture::Screen {
            source,
            region: None,
        },
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(45);
    let path = loop {
        assert!(Instant::now() < deadline, "recording timed out");
        match session
            .events
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
        {
            Event::Progress { seconds, .. } if seconds >= 6.0 => session.stop(),
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    };
    drop(session);
    let rows: Vec<serde_json::Value> = std::fs::read_to_string(path.with_extension("log"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let last = &rows.iter().rfind(|r| r["event"] == "progress").unwrap()["detail"];
    let frames = last["frame"].as_u64().unwrap();
    let duplicates = last["dup_frames"].as_u64().unwrap();
    println!("hardware={hardware}, microphone-enabled final progress: {last}");
    assert!(frames >= 300, "too few frames: {frames}");
    assert!(
        duplicates < frames / 3,
        "capture starved: {duplicates} / {frames} duplicated"
    );
    let probe = zsr::platform::command("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,duration",
            "-of",
            "json",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert!(probe.status.success());
    let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    let streams = info["streams"].as_array().unwrap();
    assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
    assert!(streams.iter().any(|s| s["codec_type"] == "video"));
    let durations: Vec<f64> = streams
        .iter()
        .map(|s| s["duration"].as_str().unwrap().parse().unwrap())
        .collect();
    assert!(
        (durations[0] - durations[1]).abs() < 0.3,
        "A/V duration mismatch: {durations:?}"
    );
    let decode = zsr::platform::command(&saved_settings.ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(&path)
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    assert!(
        decode.status.success(),
        "{}",
        String::from_utf8_lossy(&decode.stderr)
    );
}
