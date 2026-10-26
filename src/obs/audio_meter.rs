//! Audio-thread-safe source level diagnostics; no logging or allocation in the OBS callback.
use super::{api::Ptr, runtime::Runtime};
use std::sync::atomic::{AtomicU32, Ordering};

struct Levels {
    input: AtomicU32,
    output: AtomicU32,
    callbacks: AtomicU32,
}

/// Retain the loudest peak seen so quiet intervals cannot hide an earlier captured signal.
fn retain_peak(target: &AtomicU32, value: f32) {
    if value.is_finite() {
        // Explicit CAS remains compatible with the minimum Rust version and newer atomic APIs.
        let mut current = target.load(Ordering::Relaxed);
        while value > f32::from_bits(current) {
            match target.compare_exchange_weak(
                current,
                value.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }
}

/// Copy maximum channel peaks only; OBS supplies eight entries in each callback array.
unsafe extern "C" fn updated(
    data: Ptr,
    _magnitude: *const f32,
    peak: *const f32,
    input: *const f32,
) {
    let levels = unsafe { &*data.cast::<Levels>() };
    for channel in 0..8 {
        unsafe {
            retain_peak(&levels.input, *input.add(channel));
            retain_peak(&levels.output, *peak.add(channel));
        }
    }
    levels.callbacks.fetch_add(1, Ordering::Relaxed);
}

pub struct Meter<'a> {
    rt: &'a Runtime,
    meter: Ptr,
    source: Ptr,
    name: String,
    levels: Box<Levels>,
}
impl<'a> Meter<'a> {
    /// Attach diagnostics without changing the source volume, mute state, or mixer assignment.
    pub fn new(rt: &'a Runtime, source: Ptr, name: &str) -> Result<Self, String> {
        let meter = unsafe { (rt.api.obs_volmeter_create)(2) };
        if meter.is_null() {
            return Err("OBS audio meter creation failed".into());
        }
        let result = Self {
            rt,
            meter,
            source,
            name: name.into(),
            levels: Box::new(Levels {
                input: AtomicU32::new(f32::NEG_INFINITY.to_bits()),
                output: AtomicU32::new(f32::NEG_INFINITY.to_bits()),
                callbacks: AtomicU32::new(0),
            }),
        };
        unsafe {
            (rt.api.obs_volmeter_add_callback)(
                meter,
                updated,
                (&*result.levels as *const Levels).cast_mut().cast(),
            );
            if !(rt.api.obs_volmeter_attach_source)(meter, source) {
                return Err("OBS audio meter attach failed".into());
            }
        }
        Ok(result)
    }

    /// Log source configuration and cumulative input/post-fader peaks in dBFS from the worker thread.
    pub fn report(&self) {
        let a = &self.rt.api;
        let input = f32::from_bits(self.levels.input.load(Ordering::Relaxed));
        let output = f32::from_bits(self.levels.output.load(Ordering::Relaxed));
        unsafe {
            eprintln!(
                "ZSR audio: {}",
                serde_json::json!({
                    "source": self.name, "volume_multiplier": (a.obs_source_get_volume)(self.source),
                    "muted": (a.obs_source_muted)(self.source), "mixer_mask": (a.obs_source_get_audio_mixers)(self.source),
                    "input_peak_dbfs": input.is_finite().then_some(input),
                    "source_peak_dbfs": output.is_finite().then_some(output),
                    "callbacks": self.levels.callbacks.load(Ordering::Relaxed)
                })
            );
        }
    }
}
impl Drop for Meter<'_> {
    /// Remove the callback under OBS's callback lock before releasing its Rust state.
    fn drop(&mut self) {
        unsafe {
            (self.rt.api.obs_volmeter_remove_callback)(
                self.meter,
                updated,
                (&*self.levels as *const Levels).cast_mut().cast(),
            );
            (self.rt.api.obs_volmeter_destroy)(self.meter);
        }
    }
}
