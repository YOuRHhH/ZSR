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
/// Verify window identity, original dimensions and actual changing pixels through libobs.
#[test]
#[ignore = "opens and records an animated test window; needs OBS runtime and FFmpeg only for decoding"]
fn obs_records_selected_window_pixels() {
    let title = format!("ZSR OBS fixture {}", std::process::id());
    let script = format!(
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
