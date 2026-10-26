use crate::{platform::Source, settings::Settings};
use std::path::Path;

use super::*;
use crate::platform::Display;
fn request(region: Option<Region>) -> Request {
    Request {
        settings: Settings::default(),
        capture: Capture::Screen {
            source: Source {
                display: Display {
                    x: -1920,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    id: 0,
                },
                label: String::new(),
                av_index: Some(2),
            },
            region,
        },
    }
}
#[test]
fn invalid_regions_cannot_escape_display_or_overflow() {
    for region in [
        Region {
            x: -1,
            y: 0,
            width: 100,
            height: 100,
        },
        Region {
            x: i32::MAX,
            y: 0,
            width: i32::MAX,
            height: 100,
        },
        Region {
            x: 0,
            y: 0,
            width: 0,
            height: 100,
        },
    ] {
        assert!(request(Some(region)).validate().is_err());
    }
}
#[test]
fn paths_and_devices_remain_single_arguments() {
    let mut request = request(None);
    request.settings.microphone = true;
    request.settings.audio_device = "Microphone & $(anything)".into();
    let args = request.arguments(Path::new("folder with spaces/录制.mp4"));
    assert_eq!(args.last().unwrap(), "folder with spaces/录制.mp4");
    if cfg!(windows) {
        assert!(args.contains(&"audio=Microphone & $(anything)".into()));
    }
    assert!(args.windows(2).any(|p| p == ["-map", "0:v:0"]));
}
#[test]
fn negative_display_origin_is_preserved() {
    if cfg!(windows) {
        let args = request(None).arguments(Path::new("out.mp4"));
        assert!(args.windows(2).any(|p| p == ["-offset_x", "-1920"]));
    }
}

#[test]
fn window_capture_uses_identity_not_title_and_keeps_microphone() {
    let mut request = request(None);
    request.settings.microphone = true;
    request.settings.audio_device = "Mic 录音".into();
    request.capture = Capture::Window(crate::windows::AppWindow {
        id: 123456,
        pid: 99,
        title: "Same title & $(command)".into(),
        app: "app".into(),
    });
    let args = request.arguments(Path::new("out.mp4"));
    assert!(!args.iter().any(|arg| arg.contains("Same title")));
    assert!(
        !args
            .iter()
            .any(|arg| arg == "desktop" || arg == "-offset_x")
    );
    if cfg!(windows) {
        assert!(args.windows(2).any(|p| p == ["-f", "lavfi"]));
        let graph = args.windows(2).find(|p| p[0] == "-i").unwrap()[1].as_str();
        assert!(graph.starts_with("gfxcapture=hwnd=123456:"));
        assert!(graph.ends_with("hwdownload,format=bgra"));
        assert!(!args.iter().any(|arg| arg == "gdigrab"));
        assert!(args.windows(2).any(|p| p == ["-i", "audio=Mic 录音"]));
        assert!(args.windows(2).any(|p| p == ["-map", "1:a:0"]));
    } else if cfg!(target_os = "linux") {
        assert!(args.windows(2).any(|p| p == ["-window_id", "123456"]));
    }
    if let Capture::Window(w) = &mut request.capture {
        w.id = 0;
    }
    assert!(
        request.validate().is_err(),
        "HWND 0 would accidentally capture the desktop"
    );
}
