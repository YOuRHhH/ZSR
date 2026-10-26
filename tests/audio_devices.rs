use zsr::audio;

#[test]
#[ignore = "requires local audio hardware and a staged OBS runtime; enumerates only, never records"]
fn discovers_local_inputs_with_unique_backend_identifiers() {
    let devices =
        audio::discover(&zsr::obs_backend::default_runtime()).expect("local audio enumeration");
    let mut ids = std::collections::HashSet::new();
    for device in &devices {
        assert!(!device.name.is_empty());
        assert!(!device.input.is_empty());
        assert!(ids.insert(&device.id), "device identities must be unique");
        assert_eq!(audio::selected(&devices, &device.id), Some(device));
    }
    println!("Detected {} audio input devices", devices.len());
}
