# ZSR Requirements

Status: Proposed baseline for implementation planning  
Date: 2026-10-23  
Platforms: macOS, Windows, Linux  
Implementation: Rust application and recording core, with C/C++ adapters for lower-level system integration where needed.

ZSR is a desktop screen-recording application for capturing a display, application window, or selected region with optional audio. The initial release focuses on reliable local recording and a simple, accessible interface.

## Scope and priorities

- **P0 — Initial release:** Required for the first usable release on every supported platform, subject to explicitly reported platform capabilities.
- **P1 — Follow-up:** Planned after the initial recording workflow is stable.
- **P2 — Optional:** Candidate enhancement requiring separate scope approval.

Exact minimum OS versions, supported Linux desktop environments, distribution formats, and performance targets must be fixed before release qualification. A feature counts as supported only when tested on the declared platform configuration. Unavailable capabilities must be explained in the UI rather than failing silently.

The local `obs-studio/` directory currently contains Git metadata without checked-out source or a valid HEAD commit. The OBS paths below are therefore candidate reference areas to inspect when source is available, not findings from an implementation review. OBS is a reference candidate; embedding or linking its components is not assumed.

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria / expected behavior |
| --- | --- | --- | --- |
| FR-01 | P0 | Cross-platform desktop application | ZSR installs, launches, records, and plays back a finalized output file on each declared macOS, Windows, and Linux test configuration. |
| FR-02 | P0 | Capture-source selection | Users can select an entire display, an application window, or a rectangular region where available. Source labels and previews identify the selected target; unavailable modes have an explanation. |
| FR-03 | P0 | Multiple displays | Users can select among connected displays. Selection and region coordinates remain correct across mixed scaling factors and display arrangements. Source loss triggers an explicit warning and controlled stop if recording cannot continue. |
| FR-04 | P0 | Recording controls | Users can start and stop recording and cancel any pre-recording countdown. Stopping finalizes the file and reports its location; repeated actions cannot create duplicate recording sessions. |
| FR-05 | P1 | Pause and resume | Users can pause and resume a recording when supported by the recording pipeline. Paused time is excluded from the output timeline, with audio and video synchronized after resuming. |
| FR-06 | P0 | Microphone recording | ZSR automatically enumerates connected microphone inputs and presents their readable names. Users can refresh the list, enable, disable, and select a microphone, adjust input gain, and see an activity meter. Preserve a valid selection; report a disconnected selection without silently choosing another input. Silent video recording works without microphone permission. |
| FR-07 | P0 | System-audio recording | Users can enable system audio where available. ZSR detects capability and permission state, explains limitations, and lets users continue with the available audio sources. |
| FR-08 | P0 | Audio mixing and synchronization | Enabled audio sources are mixed into the recording without sustained drift. A proposed acceptance target is no more than 100 ms audio/video offset over a 30-minute reference recording; confirm against the release test matrix. |
| FR-09 | P0 | Output quality settings | Users can select Original, 480p, 720p, 1080p, 1440p, or 2160p (4K) output resolution, 30 or 60 FPS where supported, and a quality preset. Invalid combinations are rejected before recording; the active configuration is visible. |
| FR-10 | P0 | Encoding and finalization | ZSR provides a software encoding path and produces at least one documented, interoperable video/container combination. The proposed baseline is H.264 video with AAC audio in MP4, subject to dependency and distribution review. A finalized output opens in the release reference players. |
| FR-11 | P1 | Hardware encoding | ZSR detects supported hardware encoders and offers them when usable. Initialization failure offers a software fallback, with any quality or performance change shown to the user. |
| FR-12 | P0 | Output-file management | Users can choose a writable destination. ZSR generates collision-safe filenames, shows the final path, and checks available storage before starting and during recording. |
| FR-13 | P0 | Permissions and capability detection | ZSR checks required screen and audio permissions before recording, explains missing access, and supports retry after permission changes. Linux capture behavior is qualified separately for the declared Wayland and X11 environments. |
| FR-14 | P0 | Failure handling and recovery | Permission revocation, source removal, encoder failure, and storage exhaustion produce actionable errors. ZSR attempts safe finalization and identifies any incomplete output. A recoverable intermediate-file strategy must be chosen and tested before release. |
| FR-15 | P0 | Persistent preferences | ZSR retains destination, quality, audio preferences, and other safe settings across launches. Previously selected sources and devices are revalidated before use. |
| FR-16 | P1 | Global shortcuts | Users can configure start/stop and pause/resume shortcuts where the OS or desktop allows them. Conflicts and registration failures are reported; in-app controls remain available. |
| FR-17 | P0 | Local operation and privacy | Recording and saving work without an account or network connection. ZSR does not upload captured media or transmit diagnostics without an explicit user action. Logs exclude captured content and avoid sensitive identifiers. |
| FR-18 | P0 | Performance and diagnostics | Capture, encoding, and file writing run outside the UI thread. ZSR measures actual FPS and dropped frames and reports sustained recording degradation. Release qualification includes a documented 30-minute 1080p/30 FPS reference test per platform; reference hardware and resource budgets remain to be agreed. |
| FR-19 | P1 | Recording history | Users can see completed recordings with duration, date, and file location, open a recording, or reveal it in the file manager. Missing files are handled gracefully. |
| FR-20 | P2 | Advanced recording features | Webcam overlays, cursor emphasis, click effects, scheduled recording, and multiple-source composition may be added as separately specified features. Live streaming and a full video editor are outside the initial scope. |
| FR-21 | P0 | Multiple interface languages | Provide English and Simplified Chinese at initial release. Users can switch languages without restarting or interrupting recording; retain the selection across launches. All application-owned controls and status messages use translation catalogs; native device names and external diagnostic output may retain their original language. |

## User interface specifications

| ID | Priority | UI area | Specification / acceptance criteria |
| --- | --- | --- | --- |
| UI-01 | P0 | Main window | Present source selection, preview, microphone/system-audio toggles, output settings, destination, and the primary Record control in one clear workflow. Advanced options belong in settings. |
| UI-02 | P0 | Source picker | Show distinct display, window, and region modes, with recognizable names and previews where available. Refresh targets and indicate unavailable selections. If an OS-managed picker is required, integrate its result into the same workflow. |
| UI-03 | P0 | Region selection | Use an overlay with a visible selection boundary and dimensions, support dragging/resizing and keyboard cancellation, and preserve correct coordinates on scaled displays. Validate the selection before recording. |
| UI-04 | P0 | Preview | Show the selected source and output aspect ratio before recording. Clearly distinguish a loading, unavailable, or frozen preview from live content. Avoid capturing ZSR's own controls where platform support permits. |
| UI-05 | P0 | Recording states | Define Idle, Preparing, Countdown, Recording, Stopping, Completed, and Error states; add Paused when FR-05 is implemented. Each state has clear text, permitted actions, and predictable transitions. Disable conflicting actions while preparing or stopping. |
| UI-06 | P0 | Active-recording indicator | Keep elapsed recording time, an explicit recording label, audio status, and a prominent Stop control visible. Use text or icons in addition to color. Expose the active capture target. |
| UI-07 | P1 | Compact controls and background operation | Offer a compact recording controller and a tray/menu-bar entry where supported. Provide Restore and Stop actions. Window-close behavior must be explicit and must never silently terminate an active recording. |
| UI-08 | P0 | Audio controls | Label microphone and system audio separately. Provide automatically populated device selection and refresh, mute state, gain controls, and a labeled activity meter. Explain unavailable devices and permission problems near the affected control. |
| UI-09 | P0 | Settings | Group capture, audio, output, and general preferences; add shortcuts when supported. Explain quality presets in plain language, validate changes, and indicate settings that apply to the next recording. |
| UI-10 | P0 | Permission guidance and errors | State what access is needed, why it is needed, and how to retry. Error messages describe the failed action, whether a recording was saved, its path when applicable, and a practical next step. |
| UI-11 | P0 | Completion view | Confirm successful save only after finalization. Show filename, duration, and location, with Open recording, Show in folder, and Record again actions. Keep failed or incomplete recordings visibly distinct. |
| UI-12 | P0 | Accessibility | Core workflows support keyboard navigation, visible focus, meaningful screen-reader labels, and readable contrast. Status changes are accessible, and essential information does not rely on color alone. Verify with the declared assistive-technology test matrix. |
| UI-13 | P0 | Layout and display scaling | The primary workflow fits a proposed minimum logical window size of 960 × 640. Smaller work areas remain usable through scrolling. Text and controls stay legible at 100%, 150%, and 200% scaling without clipped primary actions. |
| UI-14 | P1 | Appearance and platform conventions | Support system light/dark appearance and user override. Follow platform conventions for shortcuts, dialogs, and window controls. |
| UI-15 | P1 | Recording history view | Show recent recordings with clear metadata and an empty state. Separate removing a history entry from deleting its file; file deletion requires an explicit destructive action. |
| UI-16 | P0 | Language selection and text rendering | Provide a clearly labeled English / 简体中文 language selector. Render Chinese characters through an available CJK font, allow translated text to expand, and update the interface immediately after a language change. Test both languages at supported display scaling factors. |

## Engineering and integration requirements

| ID | Priority | Area | Requirement / acceptance criteria |
| --- | --- | --- | --- |
| ER-01 | P0 | Rust ownership | Implement application state, recording orchestration, preferences, error handling, and UI coordination in Rust. Keep platform adapters separate from shared recording logic. |
| ER-02 | P0 | C/C++ integration | Use narrow C-compatible interfaces for lower-level native functionality that needs C/C++. Document buffer ownership, lifetimes, threading, error codes, and cleanup. Prevent exceptions or panics from crossing FFI boundaries. |
| ER-03 | P0 | Platform abstraction | Define shared interfaces for source enumeration, capture, audio input, encoding, permissions, and shortcuts. Expose runtime capabilities so the UI can explain unsupported features. |
| ER-04 | P0 | OBS reference review | Once source is available, inspect relevant capture plugins, audio handling, encoding/output components, and hotkey implementation. Record useful design patterns and platform constraints. Review applicable licenses before copying code or shipping dependencies. |
| ER-05 | P0 | Build and release | Provide reproducible build instructions, pinned dependencies, and CI builds for each supported platform. Choose installer/package formats, signing requirements, and supported CPU architectures before release. |
| ER-06 | P0 | Verification | Test shared recording state transitions and FFI lifecycle behavior. Run platform integration tests for capture, permissions, audio synchronization, output playback, mixed display scaling, device removal, storage failure, and recovery. |

## Decisions to finalize before implementation

| Decision | Proposed starting point | Required resolution |
| --- | --- | --- |
| Product scope | Local screen recorder for individual users | Confirm P0/P1 priorities and intended users. |
| OS and desktop support | macOS, Windows, and Linux | Specify minimum versions, CPU architectures, Linux distributions/desktops, and Wayland/X11 coverage. |
| UI toolkit | Shared Rust application model with platform integration | Evaluate accessibility, packaging, preview rendering, and native integration before selecting a toolkit. |
| Media pipeline | Modular capture, audio, encoding, and output stages | Select media dependencies, output codecs, crash-recovery container strategy, and licensing approach. |
| Quality targets | 1080p/30 FPS baseline, optional higher settings | Confirm reference hardware, resource budgets, audio synchronization threshold, and acceptable dropped-frame rate. |
| Release packaging | Platform-specific desktop packages | Confirm installation, signing, update delivery, and uninstall behavior. |
