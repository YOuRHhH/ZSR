# ZSR module boundaries

Keep modules focused on one responsibility. Prefer files below roughly 200–250 lines; split by behavior as code grows rather than moving unrelated helpers into a generic utilities file. Public APIs expose domain operations, while platform commands and parsing stay internal to their backend modules.

| Module | Responsibility | Boundary |
| --- | --- | --- |
| `app/mod.rs` | Application state, initialization, frame coordination | Delegates workflows and rendering; contains no capture commands |
| `app/controller.rs` | Recording transitions and background-result handling | Resolves a selected device to its current recording input at start |
| `app/ui/` | Layout, capture, audio, output, recording status, and notices | Widgets use domain state; never run device probes or media processes |
| `app/region/` | Region overlay lifecycle, interaction, painting, and placement | Converts confirmed selections to capture pixels; no media process ownership |
| `region.rs` | Display-relative coordinate conversion | Pure geometry with scaling, bounds, and minimum-size validation |
| `app/appearance.rs` | Theme and system font configuration | Isolates platform font paths from layout |
| `app/preferences.rs` | Debounced preference persistence | Independent of panel rendering |
| `app/smoke.rs` | Optional application-frame snapshot verification | Excluded from ordinary builds |
| `audio/mod.rs` | Audio device model, identity matching, backend dispatch | Separates human-readable names, persisted identities, and capture inputs |
| `audio/manager.rs` | Asynchronous discovery, refresh, and selection reconciliation | Owns background work and keeps probes off the UI thread |
| `audio/dshow.rs` | Windows DirectShow device enumeration format | Uses alternative identifiers to distinguish same-named devices |
| `audio/avfoundation.rs` | macOS audio device enumeration format | Separates audio/video sections and resolves current audio indices |
| `audio/pulse.rs` | Linux PulseAudio device enumeration format | Reads source descriptions and excludes output monitors from microphone choices |
| `recording/mod.rs` | Shared capture requests and recording events | Defines the recording API without UI dependencies |
| `recording/validation.rs` | Recording-request validation | Rejects unsupported or out-of-bounds input before starting workers |
| `recording/command.rs` | Shell-free FFmpeg argument construction | Knows backend capture syntax; no process ownership |
| `recording/session.rs` | Recording worker lifecycle and stop channel | Exposes start, events, stop, and cleanup |
| `recording/worker.rs` | Media process supervision and output finalization | Owns FFmpeg and reports typed events |
| `platform.rs`, `native/` | Display discovery and bounded setup subprocesses | Contains the C ABI and platform process flags |
| `settings.rs`, `i18n.rs`, `locales/` | Persisted settings and translated text | Shared by application workflows and UI |

Audio discovery runs at startup and on refresh. While microphone recording is enabled and the recorder is idle, ZSR refreshes the inventory every 15 seconds. A new installation selects the first detected input without enabling microphone recording. An unavailable saved input remains unavailable until the user explicitly chooses another; it is never silently replaced. Device inventory is frozen during recording, and backend errors handle devices unplugged during capture.

Linux discovery requires `pactl` with JSON output support (for example, `pulseaudio-utils` on Debian/Ubuntu) and a PulseAudio server or PipeWire PulseAudio compatibility server. macOS currently persists names with duplicate ordinals because the enumerated recording backend exposes indices; identical device names and renaming still require platform qualification. Discovery enumerates device information and does not record audio samples.

Windows region selection uses a dedicated native thread coordinated by app/region/windows.rs. Native picker_window.c owns the message loop and resources, picker_input.c handles geometry, and picker_paint.c updates a reusable alpha bitmap atomically. Only a completed selection, cancellation, or error crosses back to Rust. macOS/Linux use app/region/window.rs with an independent deferred egui viewport. The minimized main window does not render the Windows picker.

Application-window discovery lives in src/windows/: WindowInventory owns asynchronous refresh and source identity, win32.rs bridges native/windows.c, and x11.rs parses wmctrl output. The UI selects an (ID, PID) pair. Capture::Window carries that identity into the recording worker, which revalidates it before launching FFmpeg. Titles are labels only and never determine the recording target.
