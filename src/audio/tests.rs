use super::{
    avfoundation::parse as parse_avfoundation, dshow::parse as parse_dshow,
    pulse::parse as parse_pulse,
};

use super::*;
#[test]
fn dshow_keeps_unicode_names_and_unique_alternative_ids() {
    let devices = parse_dshow(concat!(
        "[in#0 @ abc] \"Camera\" (video)\n[in#0 @ abc] Alternative name \"camera-id\"\n",
        "[in#0 @ abc] \"麦克风\" (audio)\n[in#0 @ abc] Alternative name \"mic-1\"\n",
        "[in#0 @ abc] \"麦克风\" (audio)\n[in#0 @ abc] Alternative name \"mic-2\"\n"
    ));
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].name, "麦克风");
    assert_eq!(devices[1].input, "mic-2");
    assert!(selected(&devices, "麦克风").is_none());
    assert_eq!(selected(&devices, "mic-1"), Some(&devices[0]));
}
#[test]
fn older_dshow_sections_are_supported() {
    let devices = parse_dshow(
        "[dshow @ abc] DirectShow video devices\n[dshow @ abc] \"Camera\"\n[dshow @ abc] DirectShow audio devices\n[dshow @ abc] \"Mic\"\n[dshow @ abc] Alternative name \"id\"",
    );
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].input, "id");
}
#[test]
fn mac_audio_identity_survives_reindexing() {
    let a = parse_avfoundation(
        "[AVFoundation @ abc] AVFoundation video devices:\n[AVFoundation @ abc] [0] Camera\n[AVFoundation @ abc] AVFoundation audio devices:\n[AVFoundation @ abc] [0] Built-in Microphone\n[AVFoundation @ abc] [1] USB 麦克风",
    );
    let b = parse_avfoundation(
        "[AVFoundation @ abc] AVFoundation audio devices:\n[AVFoundation @ abc] [0] USB 麦克风\n[AVFoundation @ abc] [1] Built-in Microphone",
    );
    assert_eq!(a.len(), 2);
    assert_eq!(selected(&b, &a[1].id).unwrap().input, "0");
}
#[test]
fn pulse_filters_monitor_sources_and_preserves_machine_ids() {
    let devices = parse_pulse(
        r#"[
            {"name":"alsa_input.usb","description":"USB 麦克风","monitor_of_sink":null},
            {"name":"alsa_input.pci","description":"Internal Mic","monitor_of_sink":4294967295},
            {"name":"alsa_output.monitor","description":"Monitor","monitor_of_sink":0},
            {"name":"custom","properties":{"device.class":"monitor"}},
            {"name":"other","monitor_of_sink":1}]
        "#,
    )
    .unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].input, "alsa_input.usb");
    assert!(parse_pulse("Connection failure").is_err());
}
#[test]
fn refresh_does_not_silently_replace_disconnected_device() {
    let devices = vec![AudioDevice {
        id: "current".into(),
        name: "Mic".into(),
        input: "1".into(),
    }];
    let mut saved = "unplugged".into();
    reconcile(&devices, &mut saved);
    assert_eq!(saved, "unplugged");
    assert!(selected(&devices, &saved).is_none());
    let mut fresh = String::new();
    reconcile(&devices, &mut fresh);
    assert_eq!(fresh, "current");
    reconcile(&[], &mut fresh);
    assert_eq!(fresh, "current");
}
