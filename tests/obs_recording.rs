//! End-to-end libobs tests; the application never invokes the ffmpeg command-line encoder.
#[cfg(windows)]
#[path = "support/tone.rs"]
mod tone;
use std::time::{Duration, Instant};
use zsr::{
    recording::{Capture, Event, Region, Request, Session},
    settings::Settings,
};
/// Verify screen/region, microphone and hardware/software paths through the actual worker.
#[test]
#[ignore = "records display and default microphone briefly; needs a staged libobs runtime and ffprobe for verification"]
fn obs_screen_region_and_audio_finalize() {
    let runtime = zsr::obs_backend::default_runtime();
    let capabilities = zsr::platform::probe(&runtime);
    assert!(capabilities.error.is_none(), "{:?}", capabilities.error);
    let source = capabilities.sources[0].clone();
    for (desktop_audio, microphone, audio_tracks) in [
        (false, false, zsr::settings::AudioTracks::Mixed),
        (true, true, zsr::settings::AudioTracks::Mixed),
        (true, true, zsr::settings::AudioTracks::Separate),
        (true, false, zsr::settings::AudioTracks::Separate),
    ] {
        #[cfg(windows)]
        let _tone = desktop_audio.then(tone::Tone::start);
        let folder = tempfile::tempdir().unwrap();
        let session = Session::start(Request {
            settings: Settings {
                destination: folder.path().into(),
                obs_runtime: runtime.clone(),
                fps: 60,
                microphone,
                desktop_audio,
                audio_tracks,
                audio_device: "default".into(),
                hardware_encoding: microphone,
                ..Default::default()
            },
            capture: Capture::Screen {
                source: source.clone(),
                region: (!microphone).then_some(Region {
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                }),
            },
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(50);
        let path = loop {
            assert!(Instant::now() < deadline, "OBS timeout");
            match session
                .events
                .recv_timeout(Duration::from_secs(45))
                .unwrap()
            {
                Event::Progress { seconds, .. } if seconds >= 3.0 => session.stop(),
                Event::Saved(path) => break path,
                Event::Failed { detail, .. } => panic!("{detail}"),
                _ => {}
            }
        };
        drop(session);
        assert!(std::fs::metadata(&path).unwrap().len() > 1000);
        let output = zsr::platform::command("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_name,width,height,duration",
                "-of",
                "json",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let streams = value["streams"].as_array().unwrap();
        let expected_audio = if desktop_audio
            && microphone
            && audio_tracks == zsr::settings::AudioTracks::Separate
        {
            2
        } else {
            1
        };
        assert_eq!(
            streams
                .iter()
                .filter(|stream| stream["codec_name"] == "aac")
                .count(),
            expected_audio,
            "{value}"
        );
        let stream = &value["streams"][0];
        assert_eq!(stream["codec_name"], "h264");
        assert_eq!(
            stream["width"],
            if microphone {
                source.display.width
            } else {
                640
            }
        );
        assert_eq!(
            stream["height"],
            if microphone {
                source.display.height
            } else {
                360
            }
        );
        assert!(stream["duration"].as_str().unwrap().parse::<f64>().unwrap() >= 3.0);
        let decoded = zsr::platform::command("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args(["-map", "0", "-f", "null", "-"])
            .output()
            .unwrap();
        assert!(
            decoded.status.success(),
            "{}",
            String::from_utf8_lossy(&decoded.stderr)
        );
        let audio_log = std::fs::read_to_string(path.with_extension("log")).unwrap();
        let meter = audio_log
            .lines()
            .filter_map(|line| line.strip_prefix("ZSR audio: "))
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .rfind(|v| v["source"] == "Desktop audio");
        if desktop_audio {
            let meter = meter.as_ref().expect("desktop diagnostics missing");
            assert_eq!(meter["volume_multiplier"], 1.0);
            assert_eq!(meter["muted"], false);
            assert_eq!(meter["mixer_mask"], 1);
            println!("Desktop source measurement: {meter}");
        }
        #[cfg(windows)]
        if desktop_audio {
            let pcm = zsr::platform::command("ffmpeg")
                .args(["-v", "error", "-i"])
                .arg(&path)
                .args([
                    "-map", "0:a:0", "-ac", "2", "-ar", "48000", "-f", "f32le", "-",
                ])
                .output()
                .unwrap();
            assert!(pcm.status.success());
            let samples: Vec<f32> = pcm
                .stdout
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            let energy =
                samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64;
            if !microphone {
                let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                let encoded_db = 20.0 * f64::from(peak).log10();
                let captured_db = meter.as_ref().unwrap()["source_peak_dbfs"]
                    .as_f64()
                    .unwrap();
                println!(
                    "Desktop peak: OBS={captured_db:.2} dBFS, decoded AAC={encoded_db:.2} dBFS"
                );
                assert!(
                    (encoded_db - captured_db).abs() < 3.0,
                    "Unexpected level change between OBS source and encoded track"
                );
            }
            assert!(
                energy.sqrt() > 0.001,
                "Desktop track is silent despite test playback"
            );
        }
        let log = std::fs::read_to_string(path.with_extension("log")).unwrap();
        assert!(log.contains("Total frames output"));
        assert!(!log.contains("Double destroy"));
        println!(
            "OBS desktop={desktop_audio}, microphone={microphone}, tracks={audio_tracks:?}: {}",
            value["streams"]
        );
    }
}
/// Missing runtime must fail explicitly instead of silently invoking a different capture engine.
#[test]
fn missing_obs_runtime_is_reported() {
    assert!(zsr::obs_backend::resolve("ZSR-missing-obs-runtime").is_err());
}
