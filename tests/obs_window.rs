//! Native window-capture regression using a disposable, animated window owned by this test.
#![cfg(windows)]
use std::{
    process::{Child, Stdio},
    time::{Duration, Instant},
};
use zsr::{
    recording::{Capture, Event, Request, Session},
    settings::Settings,
};
struct Fixture(Child);
impl Drop for Fixture {
    /// Reap only the fixture process created by this test.
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
/// Share an owned animation between recording and memory-only preview regression checks.
fn fixture(name: &str) -> (Fixture, zsr::windows::AppWindow) {
    fixture_with_size(name, 640, 360)
}

/// Vary the native aspect ratio to exercise OBS's output alignment requirements.
fn fixture_with_size(name: &str, width: u32, height: u32) -> (Fixture, zsr::windows::AppWindow) {
    let title = format!("ZSR OBS {name} fixture {}", std::process::id());
    let script = format!(
        r#"
Add-Type -AssemblyName PresentationFramework
$window = New-Object System.Windows.Window
$window.Title = '{title}'
$window.Width = {width}
$window.Height = {height}
$window.WindowStyle = 'None'
$window.ResizeMode = 'NoResize'
$window.WindowStartupLocation = 'Manual'
$window.Left = 0
$window.Top = 0
$window.Topmost = $true
$window.Background = [System.Windows.Media.Brushes]::CornflowerBlue
# Distinct edge strips catch clipping that a uniform animation cannot reveal.
$grid = New-Object System.Windows.Controls.Grid
$left = New-Object System.Windows.Controls.Border
$left.Width = 16
$left.HorizontalAlignment = 'Left'
$left.Background = [System.Windows.Media.Brushes]::Red
$right = New-Object System.Windows.Controls.Border
$right.Width = 16
$right.HorizontalAlignment = 'Right'
$right.Background = [System.Windows.Media.Brushes]::Lime
$grid.Children.Add($left) | Out-Null
$grid.Children.Add($right) | Out-Null
$window.Content = $grid
$timer = New-Object System.Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromMilliseconds(250)
$timer.Add_Tick({{
 if ($window.Background -eq [System.Windows.Media.Brushes]::CornflowerBlue) {{ $window.Background = [System.Windows.Media.Brushes]::Orange }}
 else {{ $window.Background = [System.Windows.Media.Brushes]::CornflowerBlue }}
}})
$timer.Start()
$window.ShowDialog() | Out-Null
"#
    );
    let _fixture = Fixture(
        zsr::platform::command("powershell.exe")
            .args(["-NoProfile", "-STA", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    let target = loop {
        if let Some(w) = zsr::windows::discover()
            .unwrap()
            .into_iter()
            .find(|w| w.title == title)
        {
            break w;
        }
        assert!(start.elapsed() < Duration::from_secs(15));
        std::thread::sleep(Duration::from_millis(100));
    };
    (_fixture, target)
}

/// Verify window identity, original dimensions and actual changing pixels through libobs.
#[test]
#[ignore = "opens and records an animated test window; needs OBS runtime and FFmpeg only for decoding"]
fn obs_records_selected_window_pixels() {
    let (_fixture, target) = fixture("recording");
    let folder = tempfile::tempdir().unwrap();
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().into(),
            fps: 60,
            microphone: false,
            ..Default::default()
        },
        capture: Capture::Window(target),
    })
    .unwrap();
    let path = loop {
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
    let info = zsr::platform::command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "json",
        ])
        .arg(&path)
        .output()
        .unwrap();
    let info: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(info["streams"][0]["width"], 640);
    assert_eq!(info["streams"][0]["height"], 360);
    let decoded = zsr::platform::command("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&path)
        .args([
            "-vf",
            "scale=1:1",
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .unwrap();
    assert!(decoded.status.success());
    let mut blue = 0;
    let mut orange = 0;
    for rgb in decoded.stdout.as_chunks::<3>().0 {
        if u16::from(rgb[2]) > u16::from(rgb[0]) + 30 {
            blue += 1;
        }
        if rgb[0] > 200 && rgb[2] < 80 {
            orange += 1;
        }
    }
    assert!(
        blue > 20 && orange > 20,
        "capture did not contain alternating window pixels: blue={blue}, orange={orange}"
    );
}

/// Exercise IPC/compositing with multiple shapes/colors, then clear back to source pixels.
#[test]
#[ignore = "records an owned window; needs OBS runtime and FFmpeg for decoding"]
fn obs_records_and_clears_window_brush() {
    let (_fixture, target) = fixture("brush");
    let area = zsr::brush::desktop::bounds(&Capture::Window(target.clone())).unwrap();
    assert_eq!((area.width, area.height), (640, 360));
    let folder = tempfile::tempdir().unwrap();
    let session = Session::start(Request {
        settings: Settings {
            destination: folder.path().into(),
            microphone: false,
            desktop_audio: false,
            ..Default::default()
        },
        capture: Capture::Window(target),
    })
    .unwrap();
    let mut cleared = false;
    let path = loop {
        match session
            .events
            .recv_timeout(Duration::from_secs(45))
            .unwrap()
        {
            Event::Started => session.set_ink(zsr::brush::Ink {
                strokes: vec![
                    zsr::brush::Stroke {
                        points: (0..101).map(|i| [0.1 + i as f32 * 0.008, 0.5]).collect(),
                        width: 0.08,
                        tool: zsr::brush::Tool::Freehand,
                        color: zsr::brush::COLOR,
                    },
                    zsr::brush::Stroke {
                        points: vec![[0.25, 0.2], [0.75, 0.8]],
                        width: 0.04,
                        tool: zsr::brush::Tool::Rectangle,
                        color: [30, 220, 90],
                    },
                    zsr::brush::Stroke {
                        points: vec![[0.3, 0.3], [0.7, 0.7]],
                        width: 0.04,
                        tool: zsr::brush::Tool::Ellipse,
                        color: [0, 0, 0],
                    },
                    zsr::brush::Stroke {
                        points: vec![[0.1, 0.9], [0.9, 0.9]],
                        width: 0.04,
                        tool: zsr::brush::Tool::Arrow,
                        color: [40, 200, 255],
                    },
                ],
            }),
            Event::Progress { seconds, .. } if seconds >= 4.0 => session.stop(),
            Event::Progress { seconds, .. } if seconds >= 2.0 && !cleared => {
                session.set_ink(zsr::brush::Ink::default());
                cleared = true;
            }
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    };
    drop(session);
    for (x, y, expected) in [
        (500, 180, zsr::brush::COLOR),
        (160, 180, [30, 220, 90]),
        (320, 108, [0, 0, 0]),
        (320, 324, [40, 200, 255]),
    ] {
        for (time, expect_ink) in [("1.0", true), ("3.0", false)] {
            let pixels = zsr::platform::command("ffmpeg")
                .args(["-v", "error", "-ss", time, "-i"])
                .arg(&path)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    &format!("crop=2:2:{x}:{y},scale=1:1"),
                    "-pix_fmt",
                    "rgb24",
                    "-f",
                    "rawvideo",
                    "-",
                ])
                .output()
                .unwrap();
            assert!(pixels.status.success());
            let rgb = &pixels.stdout;
            assert_eq!(rgb.len(), 3);
            let matches = rgb
                .iter()
                .zip(expected)
                .all(|(&actual, expected)| actual.abs_diff(expected) < 35);
            assert_eq!(
                matches, expect_ink,
                "unexpected brush pixel at {time}s, ({x}, {y}): {rgb:?}, expected {expected:?}"
            );
        }
    }
}

/// Exercise real preview pixels, helper teardown/restart, scaling, and absence of media files.
#[test]
#[ignore = "opens an animated test window; needs a staged OBS runtime"]
fn obs_previews_selected_window_without_recording() {
    let (_fixture, target) = fixture("preview");
    let folder = tempfile::tempdir().unwrap();
    for resolution in [
        zsr::resolution::Resolution::Original,
        zsr::resolution::Resolution::P720,
    ] {
        let session = zsr::preview::Session::start(Request {
            settings: Settings {
                destination: folder.path().into(),
                resolution,
                ..Default::default()
            },
            capture: Capture::Window(target.clone()),
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut blue = false;
        let mut orange = false;
        let mut frames = 0;
        let mut last_pixel = [0; 4];
        while !(blue && orange) {
            assert!(
                Instant::now() < deadline,
                "Preview did not show changing source pixels: {frames} frames, last RGBA={last_pixel:?}, blue={blue}, orange={orange}"
            );
            if let Some(result) = session.take_frame() {
                let frame = result.unwrap();
                assert_eq!((frame.width, frame.height), (640, 360));
                // Inspect the center, away from window borders and the pointer.
                let offset = ((frame.height / 2 * frame.width + frame.width / 2) * 4) as usize;
                let rgb = &frame.rgba[offset..offset + 3];
                frames += 1;
                last_pixel.copy_from_slice(&frame.rgba[offset..offset + 4]);
                blue |= u16::from(rgb[2]) > u16::from(rgb[0]) + 30;
                orange |= rgb[0] > 200 && rgb[2] < 80;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Dropping a preview must not wait for OBS or create a partial MP4/log file.
        let dropped = Instant::now();
        drop(session);
        assert!(dropped.elapsed() < Duration::from_millis(100));
    }
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);
}

/// Verify the selected display and a crop within our fixture through the same preview path.
#[test]
#[ignore = "opens an animated test window and previews the desktop in memory; needs OBS runtime"]
fn obs_previews_display_and_region() {
    let (_fixture, _) = fixture("region preview");
    let capabilities = zsr::platform::probe(&zsr::obs_backend::default_runtime());
    let source = capabilities
        .sources
        .into_iter()
        .find(|s| s.display.x == 0 && s.display.y == 0)
        .expect("primary display at fixture origin");
    let folder = tempfile::tempdir().unwrap();
    for region in [
        None,
        Some(zsr::recording::Region {
            x: 64,
            y: 64,
            width: 200,
            height: 100,
        }),
    ] {
        let session = zsr::preview::Session::start(Request {
            settings: Settings {
                destination: folder.path().into(),
                ..Default::default()
            },
            capture: Capture::Screen {
                source: source.clone(),
                region,
            },
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut blue = false;
        let mut orange = false;
        while !(blue && orange) {
            assert!(
                Instant::now() < deadline,
                "Display/crop preview did not contain fixture pixels"
            );
            if let Some(frame) = session.take_frame() {
                let frame = frame.unwrap();
                let (x, y) = if region.is_some() {
                    assert_eq!((frame.width, frame.height), (200, 100));
                    (100, 50)
                } else {
                    assert!(frame.width <= 640 && frame.height <= 360);
                    (
                        100 * frame.width / source.display.width as u32,
                        100 * frame.height / source.display.height as u32,
                    )
                };
                let offset = ((y * frame.width + x) * 4) as usize;
                let rgb = &frame.rgba[offset..offset + 3];
                blue |= u16::from(rgb[2]) > u16::from(rgb[0]) + 30;
                orange |= rgb[0] > 200 && rgb[2] < 80;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);
}

/// A 360x640 window scales to 202x360 before OBS rounds its output width down to 200.
#[test]
#[ignore = "opens a portrait test window; needs a staged OBS runtime"]
fn obs_previews_portrait_window() {
    let (_fixture, target) = fixture_with_size("portrait preview", 360, 640);
    let session = zsr::preview::Session::start(Request {
        settings: Settings::default(),
        capture: Capture::Window(target),
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        assert!(
            Instant::now() < deadline,
            "Portrait preview never delivered a frame"
        );
        if let Some(result) = session.take_frame() {
            let frame = result.unwrap();
            assert_eq!((frame.width, frame.height), (200, 360));
            assert_eq!(frame.rgba.len(), 200 * 360 * 4);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Changing the canvas aspect ratio must preserve both edges, not just deliver a frame.
#[test]
#[ignore = "opens test windows with colored edge markers; needs a staged OBS runtime"]
fn obs_window_preview_preserves_both_edges() {
    for (width, height) in [(360, 640), (1000, 700), (1000, 360)] {
        let (_fixture, target) = fixture_with_size("preview edges", width, height);
        let session = zsr::preview::Session::start(Request {
            settings: Settings::default(),
            capture: Capture::Window(target),
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut edges = ([0; 3], [0; 3]);
        loop {
            assert!(
                Instant::now() < deadline,
                "Preview clipped an edge for {width}x{height}: left/right RGB={edges:?}"
            );
            if let Some(result) = session.take_frame() {
                let frame = result.unwrap();
                let row = (frame.height / 2 * frame.width * 4) as usize;
                let right = row + (frame.width as usize - 3) * 4;
                edges.0.copy_from_slice(&frame.rgba[row + 8..row + 11]);
                edges.1.copy_from_slice(&frame.rgba[right..right + 3]);
                if edges.0[0] > 220
                    && edges.0[1] < 30
                    && edges.0[2] < 30
                    && edges.1[0] < 30
                    && edges.1[1] > 220
                    && edges.1[2] < 30
                {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
