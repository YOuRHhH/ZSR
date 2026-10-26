use zsr::{
    recording::{Capture, Request},
    resolution::Resolution,
    settings::Settings,
};

#[test]
#[ignore = "requires FFmpeg and ffprobe; encodes generated portrait frames only"]
fn presets_encode_exact_dimensions_without_stretching_or_cropping() {
    let directory = tempfile::tempdir().unwrap();
    for resolution in Resolution::ALL {
        let path = directory.path().join(format!("{resolution:?}.mp4"));
        let request = Request {
            settings: Settings {
                resolution,
                ..Settings::default()
            },
            capture: Capture::TestPattern,
        };
        let mut args = request.arguments(&path);
        args.retain(|arg| arg != "-re");
        let input = args.iter().position(|arg| arg == "-i").unwrap() + 1;
        args[input] = "color=c=red:size=320x640:rate=30".into();
        args.splice(
            args.len() - 1..args.len() - 1,
            ["-frames:v".into(), "1".into()],
        );
        let encoded = zsr::platform::command("ffmpeg")
            .args(args)
            .output()
            .unwrap();
        assert!(
            encoded.status.success(),
            "{resolution:?}: {}",
            String::from_utf8_lossy(&encoded.stderr)
        );
        let probe = zsr::platform::command("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=width,height,sample_aspect_ratio",
                "-of",
                "json",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(probe.status.success());
        let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        let expected = resolution.dimensions().unwrap_or((320, 640));
        assert_eq!(info["streams"][0]["width"], expected.0);
        assert_eq!(info["streams"][0]["height"], expected.1);
        assert_eq!(info["streams"][0]["sample_aspect_ratio"], "1:1");
        let decoded = zsr::platform::command("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(&path)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale=3:1:flags=neighbor",
                "-pix_fmt",
                "rgb24",
                "-f",
                "rawvideo",
                "-",
            ])
            .output()
            .unwrap();
        assert!(decoded.status.success());
        assert_eq!(decoded.stdout.len(), 9);
        assert!(
            decoded.stdout[3] > 220 && decoded.stdout[4] < 20 && decoded.stdout[5] < 20,
            "center must retain the red source"
        );
        if resolution != Resolution::Original {
            assert!(
                decoded.stdout[..3]
                    .iter()
                    .chain(&decoded.stdout[6..])
                    .all(|p| *p < 20),
                "portrait source must have black side bars: {:?}",
                decoded.stdout
            );
        }
    }
}
