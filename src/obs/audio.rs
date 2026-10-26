use super::{
    api::Ptr,
    runtime::{Runtime, audio_source, c},
};
use std::ptr::null_mut;
use zsr::settings::{AudioTracks, Settings};

/// Keep capture sources and AAC encoders alive until the recording output has been released.
pub struct Audio<'a> {
    rt: &'a Runtime,
    sources: Vec<Ptr>,
    encoders: Vec<Ptr>,
    meters: Vec<super::audio_meter::Meter<'a>>,
}

/// Assign separate mixer buses only when both enabled sources need independent tracks.
fn routing(settings: &Settings) -> (u32, u32, Vec<&'static str>) {
    if settings.audio_tracks == AudioTracks::Separate
        && settings.desktop_audio
        && settings.microphone
    {
        (1, 2, vec!["Desktop audio", "Microphone"])
    } else {
        let name = match (settings.desktop_audio, settings.microphone) {
            (true, true) => "Mixed audio",
            (true, false) => "Desktop audio",
            (false, true) => "Microphone",
            _ => "Silent audio",
        };
        (1, 1, vec![name])
    }
}

impl<'a> Audio<'a> {
    /// Capture the default playback device and selected microphone, then prepare the requested tracks.
    pub fn new(rt: &'a Runtime, settings: &Settings) -> Result<Self, String> {
        let mut audio = Self {
            rt,
            sources: Vec::new(),
            encoders: Vec::new(),
            meters: Vec::new(),
        };
        let (desktop_mask, mic_mask, tracks) = routing(settings);
        if settings.desktop_audio {
            let id = if cfg!(windows) {
                "wasapi_output_capture"
            } else if cfg!(target_os = "macos") {
                "sck_audio_capture"
            } else {
                "pulse_output_capture"
            };
            audio.add_source(
                id,
                "Desktop audio",
                serde_json::json!({"device_id":"default", "type":0, "use_device_timing":false}),
                desktop_mask,
            )?;
        }
        if settings.microphone {
            if !rt
                .choices(audio_source(), "device_id")
                .iter()
                .any(|d| d["id"] == settings.audio_device)
            {
                return Err(
                    "Selected OBS microphone is unavailable; select it again in ZSR".into(),
                );
            }
            audio.add_source(
                audio_source(),
                "Microphone",
                serde_json::json!({"device_id":settings.audio_device,"use_device_timing":false}),
                mic_mask,
            )?;
        }
        // Native hybrid MP4 requires at least one AAC encoder, even with both sources disabled.
        for (mixer, name) in tracks.iter().enumerate() {
            let data = rt.data(&serde_json::json!({"bitrate":160}));
            let encoder = unsafe {
                (rt.api.obs_audio_encoder_create)(
                    c("ffmpeg_aac").as_ptr(),
                    c(name).as_ptr(),
                    data,
                    mixer,
                    null_mut(),
                )
            };
            unsafe {
                (rt.api.obs_data_release)(data);
            }
            if encoder.is_null() {
                return Err("OBS AAC encoder is unavailable".into());
            }
            audio.encoders.push(encoder);
            unsafe {
                (rt.api.obs_encoder_set_audio)(encoder, (rt.api.obs_get_audio)());
            }
        }
        Ok(audio)
    }

    /// Route each live source only to its selected mixer; never enable local audio monitoring.
    fn add_source(
        &mut self,
        id: &str,
        name: &str,
        settings: serde_json::Value,
        mask: u32,
    ) -> Result<(), String> {
        let a = &self.rt.api;
        let data = self.rt.data(&settings);
        let source =
            unsafe { (a.obs_source_create)(c(id).as_ptr(), c(name).as_ptr(), data, null_mut()) };
        unsafe {
            (a.obs_data_release)(data);
        }
        if source.is_null() {
            return Err(format!("OBS audio capture unavailable: {name} ({id})"));
        }
        self.sources.push(source);
        self.meters
            .push(super::audio_meter::Meter::new(self.rt, source, name)?);
        unsafe {
            (a.obs_source_set_audio_mixers)(source, mask);
            (a.obs_set_output_source)(self.sources.len() as u32, source);
        }
        Ok(())
    }

    /// Attach contiguous MP4 audio tracks in desktop-first order when sources are separated.
    pub fn attach(&self, output: Ptr) {
        for (index, encoder) in self.encoders.iter().enumerate() {
            unsafe {
                (self.rt.api.obs_output_set_audio_encoder)(output, *encoder, index);
                eprintln!(
                    "ZSR audio track: index={index}, mixer={}, AAC=160kbps, sample_rate=48000, channels=2",
                    (self.rt.api.obs_encoder_get_mixer_index)(*encoder)
                );
            }
        }
    }

    /// Sample diagnostics outside the real-time audio callback without changing audio levels.
    pub fn report(&self) {
        for meter in &self.meters {
            meter.report();
        }
    }
}

impl Drop for Audio<'_> {
    /// Detach capture channels and release encoders after the output has stopped using them.
    fn drop(&mut self) {
        self.report();
        self.meters.clear();
        unsafe {
            for (index, source) in self.sources.iter().enumerate() {
                (self.rt.api.obs_set_output_source)(index as u32 + 1, null_mut());
                (self.rt.api.obs_source_release)(*source);
            }
            for encoder in &self.encoders {
                (self.rt.api.obs_encoder_release)(*encoder);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Check all source combinations so separate tracks never duplicate or leak the other source.
    #[test]
    fn sources_are_routed_to_requested_tracks() {
        for desktop_audio in [false, true] {
            for microphone in [false, true] {
                for audio_tracks in [AudioTracks::Mixed, AudioTracks::Separate] {
                    let s = Settings {
                        desktop_audio,
                        microphone,
                        audio_tracks,
                        ..Default::default()
                    };
                    let (desktop, mic, names) = routing(&s);
                    let separate =
                        desktop_audio && microphone && audio_tracks == AudioTracks::Separate;
                    assert_eq!(names.len(), if separate { 2 } else { 1 });
                    assert_eq!(desktop, 1);
                    assert_eq!(mic, if separate { 2 } else { 1 });
                }
            }
        }
    }
}
