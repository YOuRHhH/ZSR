# OBS reference review: desktop selection

The local OBS source is now checked out. The following implementation areas were inspected:

| Source | Behavior relevant to ZSR |
| --- | --- |
| `obs-studio/plugins/win-capture/monitor-capture.c`, `monitor_capture_tick` / `monitor_capture_render` | Captures updated desktop content and renders it as a source. |
| `obs-studio/plugins/win-capture/duplicator-monitor-capture.c` | Provides monitor capture with DXGI duplication and frame updates. |
| `obs-studio/frontend/widgets/OBSBasicPreview.cpp`, `CropItem` | Adjusts source cropping through preview manipulation, translating preview coordinates into source pixels. |
| `obs-studio/frontend/widgets/OBSBasic.cpp` | Uses Windows display affinity when hiding OBS windows from capture. |

No standalone desktop rubber-band picker was found in these frontend and capture-plugin searches. OBS's source-preview cropping is different from ZSR's desktop selection window. Linking the OBS rendering engine would not by itself fix the opaque picker surface.

ZSR's Windows picker now uses its own native event-loop thread and atomically presents a reusable BGRA bitmap with UpdateLayeredWindow. Its almost-transparent background keeps the desktop visible and receives input; the frame and toolbar are opaque. It no longer combines an OpenGL viewport with color-key transparency or dynamic window regions. The main application minimizes during selection and restores on completion. See [implementation status](implementation.md) for validation and limitations.

The earlier frozen-desktop fallback was removed. No OBS source code was copied or linked for this change. The existing recording backend remains separate from picker window composition.

## Blank application recordings

OBS window-capture.c selects Windows Graphics Capture for Chrome/Mozilla classes and many Office, WinUI, and other composited windows. ZSR previously used GDI HWND capture for every application, which did not cover these rendering paths. Windows application recording now uses FFmpeg gfxcapture (WGC) with the selected HWND, followed by hwdownload and BGRA conversion. The desktop/region path is unchanged. No OBS code is copied or linked.

The capture filter is documented in the [FFmpeg source](https://www.ffmpeg.org/doxygen/trunk/vsrc__gfxcapture_8c_source.html). Required filter options are verified from the configured binary's help output; unsupported builds produce an explicit error instead of silently falling back to GDI.