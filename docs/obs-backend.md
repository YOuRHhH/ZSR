# OBS recording backend

## Architecture

ZSR's eframe UI sends a serialized capture request to the private `zsr-obs` process. The helper dynamically loads the pinned libobs 32 ABI and only the capture, audio, encoder, filter and MP4 output modules; the OBS frontend, browser and Qt interface are not launched.

- `src/obs_backend.rs`: runtime/helper discovery and device queries.
- `src/recording/obs_worker.rs`: process supervision, progress, bounded native logs and exclusive output promotion.
- `src/obs/api.rs`: audited minimal C ABI bindings resolved before initialization.
- `src/obs/runtime.rs`: runtime initialization, graphics setup, module loading, property enumeration and shutdown.
- `src/obs/source.rs`: platform capture source settings, monitor identity, region crop and aspect-ratio layout.
- `src/obs/audio.rs`: desktop/microphone sources, mixer routing and AAC track ownership.
- `src/obs/audio_meter.rs`: callback-safe input/post-fader peak measurements and source diagnostics.
- `src/obs/encoding.rs`: native encoder preference and software fallback.
- `src/obs/engine.rs`: scene ownership, microphone/AAC, native hybrid MP4, progress and asynchronous stop results.
- `native/obs_bridge.c`: variadic OBS log callback and Windows monitor device identity.

libobs owns capture timing, GPU rendering, audio synchronization and encoding. The UI does not supply frames. Parent and child communicate through JSON lines and a stop command; closing the pipe also requests stop. No FFmpeg command-line capture or DirectShow microphone discovery remains in the active path. OBS's `obs-ffmpeg` plugin supplies internal codec integration; that is not the removed FFmpeg executable pipeline.

## Sources and output

Windows uses OBS `monitor_capture` with automatic DXGI/WGC selection, `window_capture` for selected windows, and WASAPI microphone capture. On the tested hybrid/virtual-display system, OBS's automatic method selected WGC after forced DXGI was found unavailable. Regions use scene-item crop; resolution fitting is performed by OBS on the GPU. Windows Original window recordings size the canvas from the captured client dimensions.

Native MP4 requires an audio encoder, so movies with both audio sources disabled have a silent AAC track. With a microphone enabled, only the explicitly selected OBS device is attached. Old FFmpeg/DirectShow device IDs must be reselected. Hardware initialization failures try the next encoder and then x264. Runtime encoder failures after recording begins retain the partial file rather than silently restarting.

Scenes are private to the helper. Output stop waits for OBS's result signal before resource release, and teardown drains the destroy queue. Required module failures are reported during setup; optional GPU modules can fail without preventing software recording.

The recording owns its stop-callback state and disconnects the callback under OBS's signal lock before releasing the output, including failure paths. The helper revalidates requests at the process boundary, and a disconnected stop channel ends recording instead of spinning. Native display validation no longer depends on the former FFmpeg AVFoundation device index.

## Runtime / packages

Windows verification uses official OBS 32.2.2 x64, SHA-256 `4d6e40e3ab155f56b30de517380566a206d74b63cdf5ad49aa596924768f97e1`. `scripts/fetch_obs_windows.py` verifies the archive before extraction. `scripts/stage_obs.py` installs the built helper alongside OBS's hardware capability probes and dependencies, preserving licenses and source references.

Full bundles OBS; Lite uses a separately configured runtime. The helper and its OBS modules must be for the same architecture. macOS/Linux require native validation and relocatable runtime layouts described in README; they are not claimed tested by Windows results.

## Validation and limits

End-to-end worker tests passed on Windows for 640×360 region capture with software H.264 and full 1920×1080 display capture with WASAPI microphone and AMF H.264. Movies finalized, had matching dimensions and roughly aligned audio/video durations, decoded successfully, and shut down without the double-destroy diagnostics encountered during initial integration.

The migration replaces the old capture scheduling design. It does not establish that all user workloads are stutter-free. An animated application-window test also verified original 640×360 dimensions and alternating captured pixel colors. Extended motion, minimized operation, other GPUs, disk failure and native macOS/Linux runs need their own checks. Existing FFmpeg troubleshooting documents describe the superseded backend and remain as historical investigation records.

Desktop audio uses WASAPI loopback on Windows, PulseAudio output capture on Linux, and ScreenCaptureKit audio capture on macOS (native macOS/Linux validation remains pending). Mixed mode routes enabled sources to mixer 0. Separate mode with both sources uses desktop mixer 0 and microphone mixer 1, with two contiguous AAC output tracks. A single enabled source uses one track. Audio monitoring is not enabled.

## Quiet-audio diagnostics

Each recording log includes `ZSR audio:` JSON entries with source volume multiplier, mute state, mixer mask, callback count, and cumulative maximum input/post-fader peaks in dBFS. `null` peaks mean no finite signal has been measured; zero callbacks means no samples have reached the meter. Peaks are not perceived loudness or RMS measurements. The callback only updates atomics; the recording worker writes logs at its existing 500 ms interval. Meter callbacks are disconnected before their state is freed.

`ZSR audio track:` entries report the encoder mixer index, 160 kbps AAC, 48 kHz and stereo output. Low input peaks point to the incoming capture path; matching input/post-fader peaks show that the source fader is not attenuating the signal. To check the remaining mixing/encoding path, compare decoded audio with the source measurement using a desktop-only recording. The opt-in Windows tone test checks this within 3 dB (allowing AAC peak overshoot). It does not establish the cause of quiet audio in a user's different application/device setup. Desktop capture still follows the system default playback device, and no automatic boost or normalization is applied.
