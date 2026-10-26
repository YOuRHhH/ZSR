#![cfg(feature = "legacy-ffmpeg")]
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
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "creates a temporary test window and records only that window; requires Windows, PowerShell, FFmpeg and ffprobe"]
fn records_selected_window_and_rejects_closed_target() {
    record_fixture(false);
}

#[test]
#[ignore = "creates and records an animated WPF compositor window; requires Windows, PowerShell, FFmpeg gfxcapture and ffprobe"]
fn records_changing_composited_window_pixels() {
    record_fixture(true);
}

fn record_fixture(composited: bool) {
    let title = format!("ZSR window fixture {} {composited}", std::process::id());
    // The helper owns a plain solid-color window; no desktop or other app is selected.
    let script = if composited {
        format!(
            r#"
Add-Type -AssemblyName PresentationFramework
$window = New-Object System.Windows.Window
$window.Title = '{title}'
$window.Width = 640
$window.Height = 360
$window.WindowStyle = 'None'
$window.ResizeMode = 'NoResize'
$window.Background = [System.Windows.Media.Brushes]::CornflowerBlue
$timer = New-Object System.Windows.Threading.DispatcherTimer
$timer.Interval = [TimeSpan]::FromMilliseconds(333)
$timer.Add_Tick({{
    if ($window.Background -eq [System.Windows.Media.Brushes]::CornflowerBlue) {{
        $window.Background = [System.Windows.Media.Brushes]::Orange
    }} else {{ $window.Background = [System.Windows.Media.Brushes]::CornflowerBlue }}
}})
$timer.Start()
$window.ShowDialog() | Out-Null
"#
        )
    } else {
        format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = '{title}'
$form.ClientSize = New-Object System.Drawing.Size(640,360)
$form.BackColor = [System.Drawing.Color]::CornflowerBlue
$form.FormBorderStyle = 'FixedSingle'
[System.Windows.Forms.Application]::Run($form)
"#
        )
    };
    let child = zsr::platform::command("powershell.exe")
        .args(["-NoProfile", "-STA", "-Command", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut fixture = Fixture(child);
    let start = Instant::now();
    let target = loop {
        if let Some(window) = zsr::windows::discover()
            .unwrap()
            .into_iter()
            .find(|w| w.title == title)
        {
            break window;
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "fixture did not open"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let directory = tempfile::tempdir().unwrap();
    let session = Session::start(Request {
        settings: Settings {
            destination: directory.path().into(),
            cursor: false,
            ..Default::default()
        },
        capture: Capture::Window(target.clone()),
    })
    .unwrap();
    let saved = loop {
        match session
            .events
            .recv_timeout(Duration::from_secs(30))
            .unwrap()
        {
            Event::Progress { seconds, .. } if seconds >= 2.0 => session.stop(),
            Event::Saved(path) => break path,
            Event::Failed { detail, .. } => panic!("{detail}"),
            _ => {}
        }
    };
    drop(session);
    let probe = zsr::platform::command("ffprobe")
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
    assert_eq!(info["streams"][0]["height"], 360);
    let decoded = zsr::platform::command("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&saved)
        .args([
            "-frames:v",
            if composited { "10" } else { "1" },
            "-vf",
            if composited {
                "fps=5,scale=1:1"
            } else {
                "scale=1:1"
            },
            "-pix_fmt",
            "rgb24",
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .unwrap();
    assert!(decoded.status.success());
    if composited {
        let near = |pixel: &[u8], color: [i16; 3]| {
            pixel
                .iter()
                .zip(color)
                .all(|(a, b)| (i16::from(*a) - b).abs() < 15)
        };
        assert!(
            decoded.stdout.len() >= 6,
            "expected multiple composited frames"
        );
        assert!(
            decoded
                .stdout
                .as_chunks::<3>()
                .0
                .iter()
                .any(|p| near(p, [100, 149, 237])),
            "missing blue pixels: {:?}",
            decoded.stdout
        );
        assert!(
            decoded
                .stdout
                .as_chunks::<3>()
                .0
                .iter()
                .any(|p| near(p, [255, 165, 0])),
            "missing animated orange pixels: {:?}",
            decoded.stdout
        );
    } else {
        assert_eq!(decoded.stdout.len(), 3);
        // CornflowerBlue (100,149,237), allowing H.264/YUV conversion rounding.
        for (actual, expected) in decoded.stdout.iter().zip([100_i16, 149, 237]) {
            assert!(
                (i16::from(*actual) - expected).abs() < 15,
                "wrong captured pixels: {:?}",
                decoded.stdout
            );
        }
    }
    fixture.0.kill().unwrap();
    fixture.0.wait().unwrap();
    assert!(zsr::windows::validate_target(&target, "ffmpeg").is_err());
}
