//! A low-volume native playback signal for verifying real loopback capture.
use std::io::Write;

pub struct Tone {
    _file: tempfile::NamedTempFile,
}
#[link(name = "winmm")]
unsafe extern "system" {
    fn PlaySoundW(sound: *const u16, module: *mut std::ffi::c_void, flags: u32) -> i32;
}
impl Tone {
    /// Play a quiet 440 Hz wave on the default output until this guard is dropped.
    pub fn start() -> Self {
        let mut file = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        let samples: Vec<i16> = (0..48000)
            .map(|i| (1200.0 * (i as f64 * 440.0 * std::f64::consts::TAU / 48000.0).sin()) as i16)
            .collect();
        let size = samples.len() as u32 * 2;
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + size).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        for field in [1u16, 1] {
            file.write_all(&field.to_le_bytes()).unwrap();
        }
        for field in [48000u32, 96000] {
            file.write_all(&field.to_le_bytes()).unwrap();
        }
        for field in [2u16, 16] {
            file.write_all(&field.to_le_bytes()).unwrap();
        }
        file.write_all(b"data").unwrap();
        file.write_all(&size.to_le_bytes()).unwrap();
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
        let path: Vec<u16> = file
            .path()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect();
        assert_ne!(
            unsafe { PlaySoundW(path.as_ptr(), std::ptr::null_mut(), 0x20000 | 1 | 8 | 2) },
            0
        );
        Self { _file: file }
    }
}
impl Drop for Tone {
    /// Stop this process's test sound even if an assertion fails.
    fn drop(&mut self) {
        unsafe {
            PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0);
        }
    }
}
