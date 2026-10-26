# Implementation status

Date: 2026-10-23  
Milestone: Initial local recording application

## Decisions made for this milestone

Windows display/region capture now prefers FFmpeg `ddagrab` (DXGI desktop duplication), using a native DXGI adapter/output lookup matched to the selected display rectangle. Monitor IDs are never assumed to be DXGI output indexes. Region offsets are relative to that output. Backends without `ddagrab` or a matching DXGI output retain the WGC/GDI compatibility paths; runtime duplication failures surface as capture errors. Application-window capture still uses WGC. The main OpenGL window is opaque; region overlays keep their own transparency.

An opt-in UI timing harness (`ui-smoke`, `ZSR_UI_PERFORMANCE=<json path>`) records bounded repaint/CPU samples without saving preferences or starting capture. `ZSR_UI_TRANSPARENT_BASELINE=1` reproduces the former main-window setting for comparison. On this Windows machine both modes produced approximately 16.7 ms repaint intervals and 2 ms UI CPU time, so transparency was not established as the cause of the reported 1 FPS behavior. A live user recording was confirmed to use WGC at 1080p/60 with microphone audio; sampled CPU/GPU utilization did not show saturation. Desktop duplication startup and a short region encode are tested separately; those tests do not establish that the user's intermittent UI/recording lag is resolved.

Windows capture now temporarily opts the ZSR process out of ignored timer-resolution requests and opts its FFmpeg child out of timer and execution-speed power throttling. A reference-counted native guard restores ZSR's previous policy on worker exit, including startup failures. Unsupported Windows policies fall back to system defaults with a diagnostic. Process priority and the system power plan are unchanged. This addresses a suspected minimized-window scheduling cause; the user's desktop stutter has not yet been reproduced or confirmed resolved. Synthetic capture was verified to continue producing changing frames with UI event consumption paused for four seconds.

Reference: [Microsoft process power-throttling documentation](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setprocessinformation).

The main screen keeps capture source, microphone, resolution, frame rate, and recording controls visible. The Settings window groups language and cursor preferences, compression and save location, keyboard shortcuts, and an expandable Advanced section for FFmpeg. Preferences are locked during recording; shortcuts retain their explicit Apply action. Closing Settings cancels pending key capture and restores global bindings after key release.

| Area | Implementation |
| --- | --- |
| UI | Rust with egui/eframe, shared layout and state across desktop platforms |
| Languages | English and Simplified Chinese, JSON catalogs, runtime switching, persisted selection |
| Platform integration | Small native C adapter for display discovery, accessed through a documented C ABI |
| Media pipeline | External FFmpeg process with platform capture inputs; owned and supervised by a Rust worker |
| Encoding | Software H.264 via libx264; AAC when microphone recording is enabled |
| Output | Fragmented MP4 intermediate, final filename published only after successful process completion and recorded frames |
| Privacy | Local files, no network dependency or telemetry |
| OBS | Capture and preview-cropping source reviewed; no code reused |

## Platform coverage

| Capability | Windows | macOS | Linux |
| --- | --- | --- | --- |
| UI and languages | Implemented | Implemented; pending runtime qualification | Implemented; pending runtime qualification |
| Capture backend | FFmpeg gdigrab | FFmpeg AVFoundation; pending runtime qualification | FFmpeg x11grab; pending runtime qualification |
| Display selection | Native monitor enumeration | Enumerated screen devices; mapping needs multi-monitor qualification | Complete default X11 screen |
| Visual region selection | Implemented | Implemented; pending runtime qualification | Implemented for X11; pending runtime qualification |
| Optional microphone | DirectShow device name | AVFoundation audio index | PulseAudio source name |
| Wayland recording | N/A | N/A | Pending; UI reports unavailable |
| System audio | Pending | Pending | Pending |

Build and synthetic pipeline verification on the development host are distinct from live capture qualification. No platform is release-qualified yet.

## Development-host verification

On the Windows development host, the application compiled successfully and passed formatting, Clippy, unit tests, and the opt-in synthetic recording test. The synthetic test verified H.264 metadata and full decoding after stop/finalization. The English and Chinese interfaces were launched and their application-frame snapshots visually inspected; native display discovery and FFmpeg setup checks succeeded. No desktop or microphone recording was performed during these checks.

Application-only snapshots can be reproduced with the optional `ui-smoke` feature. Set `ZSR_UI_SNAPSHOT` to an absolute PNG path and `ZSR_UI_LANGUAGE` to `en` or `zh`, then run `cargo run --features ui-smoke`. The app captures its own rendered frame after setup completes, closes, and skips saving preferences. The normal build does not include this snapshot code. Local verification images are under `target/ui-check/` and are not source assets.

## Next milestones

1. Qualify actual screen and microphone capture on each declared OS, including permissions, mixed display scaling, device loss, and stop behavior.
2. Add a preview and audio activity meters. Visual region selection and automatic microphone discovery are implemented.
3. Implement native Wayland portal capture and platform system-audio sources; qualify macOS display cropping.
4. Add storage monitoring, recovery workflows, configurable output scaling, and settings-file migration.
5. Add pause/resume, shortcuts, hardware encoding, and signed distribution packages after the core recording path is stable.

The [requirements table](requirements.md) remains the target specification. This milestone implements a subset; pending entries are not claimed as complete.

## Audio discovery and modularization update

Microphone devices are automatically enumerated on startup and refresh. Windows uses DirectShow names and alternative identifiers, macOS enumerates AVFoundation audio devices and resolves current indices, and Linux reads PulseAudio sources using pactl JSON output. The bilingual UI provides a device dropdown and disconnected-device guidance. Capture remains blocked when an enabled microphone cannot be resolved.

The application, audio inventory, backend parsers, UI panels, and recording pipeline now live in focused modules. See [architecture](architecture.md). macOS and Linux discovery implementations still require runtime validation on those platforms.

## Visual region picker

Windows uses a dedicated native event-loop thread, launched from `app/region/windows.rs`. The main window minimizes before launch and restores after confirmation, cancellation, or an error. The C implementation is split into window lifecycle, input, and bitmap painting modules. There is no egui/OpenGL viewport or changing window region in the Windows picker.

The native window presents a premultiplied BGRA bitmap atomically with `UpdateLayeredWindow`. The background has alpha 1/255 to receive pointer input while showing the live desktop; the frame, handles, and bilingual toolbar are opaque. No desktop screenshot is taken. One bitmap and DC are reused for the session. Dragging erases only the old outline and redraws the frame/toolbar, and dirty updates are coalesced by a 16 ms timer. Hover events do not repaint. This replaces both earlier color-key and window-region/OpenGL approaches after users reported flickering.

Drawing, moving, corner resizing, confirmation (button/Enter), and cancellation (button/Escape) operate in physical display-relative pixels. Moving clamps to the selected monitor. Cancel preserves the previous selection. Loss of focus or display changes cancel the native picker. Numeric controls remain available in the main UI. The picker closes before recording is enabled.

macOS/Linux retain the independent deferred egui viewport, with their own input/render callback and alpha overlay. macOS captures the full display and applies the selected crop. Native macOS/Linux rendering and mixed-DPI multi-monitor behavior still require runtime qualification.

Automated coverage includes native drag/move/resize/clamping, button outcomes, background alpha, removal of the old border, and an actual UpdateLayeredWindow call on a hidden test window. A 100-frame offscreen 1920x1080 drawing check measured approximately 0.93 ms/frame in the Windows debug build after bitmap reuse (previously 8.18 ms with full clears). This measures CPU drawing, not visible compositor frame rate or end-to-end mouse latency. Live flicker still requires visual validation on the user's display.

The optional `ui-smoke` region preview renders the shared egui fallback only; it does not exercise the Windows native picker. Run `cargo test --bin ZSR native_picker_pixels_input_and_frame_cost -- --nocapture` for the native offscreen regression check.
## Application-window recording

The source selector now includes Application window / 应用窗口. It lists window titles, application/process information, and unique IDs, refreshes every five seconds while the mode is idle, and provides manual refresh. A vanished selection is marked unavailable and never replaced automatically. Before launching FFmpeg, the recording worker revalidates the selected window ID and process ID. No title-based or desktop fallback is used. Microphone recording remains available.

Windows discovers visible, non-minimized, non-cloaked top-level windows through native EnumWindows, excluding ZSR and tool windows. It captures the client area through Windows Graphics Capture using the FFmpeg gfxcapture filter with an explicit HWND. GPU frames are downloaded as BGRA for the existing libx264 encoder. Filter-help inspection verifies the required options before enabling recording and does not capture frames. There is no GDI or desktop fallback. Linux/X11 discovers windows with `wmctrl -lp` and records through x11grab `-window_id`. macOS AVFoundation and Wayland do not support this feature in the current backend; the UI explains the limitation.

This feature selects one window, not every window of a process. Keep the target open, visible, and at a fixed size. Protected surfaces remain subject to OS capture restrictions. Linux/X11 occlusion behavior depends on its backend. System/application audio is not added by selecting a window.

Modules: native/windows.c (Win32 discovery), src/windows/ (inventory and platform adapters), src/app/ui/windows.rs (bilingual selector), and the existing recording request/worker (validation and capture).

Validation: the Windows integration test opens a generated solid-color window in a separate process, discovers it, records by HWND through the real Session pipeline, verifies an H.264 640x360 MP4, decodes its pixels, and confirms that the closed target is rejected. It passed locally. Unit tests cover identity preservation, duplicate/Unicode titles, zero-handle rejection, and microphone mapping. Linux runtime capture and macOS window capture remain unqualified. Run the opt-in Windows test with `cargo test --test window_capture -- --ignored`.

Backend reference: [FFmpeg device documentation](https://ffmpeg.org/ffmpeg-devices.html#gdigrab).
The blank-picture fix adds a second Windows integration fixture using an animated WPF compositor window. Both the plain WinForms and WPF recordings passed locally with gfxcapture. Decoded frames contain the expected blue and orange colors, verifying changing content rather than merely counting encoded frames. The user's particular application has not yet been retested.

## Output resolution

The bilingual Output resolution selector offers Original (default), 480p (854x480), 720p (1280x720), 1080p (1920x1080), 1440p (2560x1440), and 2160p/4K (3840x2160). Resolution is independent of frame rate and compression quality, persisted in settings, and defaults to Original when loading older preferences.

The shared encoder filter applies the preset after capture and, on macOS, after region cropping. Presets scale proportionally into the selected landscape canvas with black padding and square pixels. Smaller sources upscale. Original retains source dimensions rounded down to even dimensions for H.264 compatibility. These controls apply to display, region, and application-window recordings.

Verification includes legacy-settings compatibility and preset serialization. The opt-in resolution_output integration test encodes generated portrait frames through every preset, verifies exact MP4 dimensions and square pixels with ffprobe, and decodes pixels to check the red source and black side bars. All presets passed locally. Run `cargo test --test resolution_output -- --ignored` to repeat without capturing desktop content.
## Persistent recording-area border

Confirming a region now leaves a passive border on the desktop while restoring the main app. It remains visible before, during, and after recording while Region mode is selected. Opening the picker temporarily hides it; cancellation restores the previous confirmed area. Switching capture mode or closing ZSR removes it. Numeric region changes update the border, and invalid bounds hide it.

Windows uses four small native windows with click-through, no-activation, and topmost styles. The two-pixel edges sit outside the recording rectangle and request capture exclusion where Windows supports it. Unchanged geometry performs no native updates. macOS/Linux use an independent transparent, mouse-passthrough egui viewport; live behavior there is not yet qualified.

Native Windows tests verify edge positions, no overlap with capture pixels, click-through/no-activation styles, and window destruction. Geometry tests cover negative monitor origins and out-of-bounds selections. Tests do not establish visual behavior on all desktop configurations.
## Recording shortcuts

Output settings provide editable global start/stop shortcuts, defaulting to Ctrl+Shift+F9 and Ctrl+Shift+F10. Apply registers both accelerators and persists them only on success. Duplicate or invalid combinations are rejected; registration conflicts restore the previous bindings where possible and display errors. Start obeys recording readiness and picker guards; stop uses the existing finalize workflow. Bindings are released on exit. Windows/macOS and Linux X11 use the global-hotkey platform backend; Wayland global shortcuts are not supported. Native key dispatch outside the application still requires interactive qualification.
`nShortcut entry now uses click-to-capture buttons. Press a combination directly; Escape or loss of focus cancels. Global registrations are temporarily suspended during capture and restored after key release to prevent recording actions while editing. Apply still validates and saves both shortcuts. Key conversion and duplicate validation tests pass; native interactive behavior remains to be qualified.
