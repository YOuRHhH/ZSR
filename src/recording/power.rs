/// Keep Windows capture scheduling independent of whether the recorder UI is visible.
pub(super) struct RecordingPower;

#[cfg(windows)]
unsafe extern "C" {
    fn zsr_recording_power_begin() -> i32;
    fn zsr_recording_power_end();
    fn zsr_recording_power_child(handle: *mut std::ffi::c_void) -> i32;
}

impl RecordingPower {
    /// Disable minimized-window timer throttling for this recording's lifetime.
    pub fn new() -> Self {
        #[cfg(windows)]
        // SAFETY: The native implementation owns a synchronized, reference-counted policy.
        unsafe {
            if zsr_recording_power_begin() == 0 {
                eprintln!("ZSR: Windows timer policy override unavailable; using system defaults");
            }
        }
        Self
    }

    /// Opt the owned FFmpeg process out of background timer and CPU throttling when supported.
    pub fn protect_child(&self, child: &std::process::Child) {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            // SAFETY: Child owns a live process handle; C borrows it without closing it.
            unsafe {
                if zsr_recording_power_child(child.as_raw_handle()) == 0 {
                    eprintln!(
                        "ZSR: encoder power policy override unavailable; using system defaults"
                    );
                }
            }
        }
        #[cfg(not(windows))]
        let _ = child;
    }
}

impl Drop for RecordingPower {
    /// Return the parent process to its previous policy on all worker exit paths.
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: Each guard balances exactly one native begin call.
        unsafe {
            zsr_recording_power_end()
        }
    }
}
