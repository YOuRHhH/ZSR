use super::diagnostics::Diagnostics;
/// Keep Windows capture scheduling independent of whether the recorder UI is visible.
pub(super) struct RecordingPower;

#[cfg(windows)]
unsafe extern "C" {
    fn zsr_recording_power_begin() -> i32;
    fn zsr_recording_power_end();
    fn zsr_recording_power_child(handle: *mut std::ffi::c_void) -> i32;
    fn zsr_recording_priority(handle: *mut std::ffi::c_void) -> u32;
}

impl RecordingPower {
    /// Disable minimized-window timer throttling for this recording's lifetime.
    pub fn new(log: &mut Diagnostics) -> Self {
        #[cfg(windows)]
        // SAFETY: The native implementation owns a synchronized, reference-counted policy.
        unsafe {
            if zsr_recording_power_begin() == 0 {
                log.write(
                    "parent_power",
                    "timer policy override failed; system defaults remain",
                );
            } else {
                log.write("parent_power", "timer policy override applied");
            }
        }
        #[cfg(not(windows))]
        let _ = log;
        Self
    }

    /// Opt the owned FFmpeg process out of background timer and CPU throttling when supported.
    pub fn protect_child(&self, child: &std::process::Child, log: &mut Diagnostics) {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            // SAFETY: Child owns a live process handle; C borrows it without closing it.
            unsafe {
                let result = zsr_recording_power_child(child.as_raw_handle());
                log.write(
                    "child_power",
                    match result {
                        1 => "timer and execution-speed policy overrides applied",
                        2 => "execution-speed override applied; timer override unavailable",
                        _ => "power policy override failed; system defaults remain",
                    },
                );
                let error = zsr_recording_priority(child.as_raw_handle());
                log.write("child_priority", serde_json::json!({"requested": "AboveNormal", "applied": error == 0, "windows_error": error}));
            }
        }
        #[cfg(not(windows))]
        let _ = (child, log);
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
