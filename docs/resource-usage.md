# Memory and package size

## Changes (2026-10-26)

- System CJK font bytes are loaded once and borrowed by egui and its font parser. Previously the parser cloned the owned buffer (18.79 MiB for this machine's Microsoft YaHei font). All glyphs and both UI languages remain available.
- Release builds use `opt-level = "s"`, full LTO, one code generation unit, and stripped symbols. Compiler/linker time increases; capture settings and encoding quality are unchanged.
- The shared Windows/macOS/Linux staging helper excludes unused `bin/ffplay` and `bin/ffprobe` executables. It retains FFmpeg, libraries, resources, notices, and supplied source/provenance material. It never alters the source distribution. A companion serving as the FFmpeg symlink target is retained.

## Local Windows measurements

Compared with the previously built release executable and the same local FFmpeg distribution:

| Measurement | Before | After |
| --- | ---: | ---: |
| ZSR release executable | 5.93 MiB | 4.42 MiB |
| Idle process private bytes (median) | 129.50 MiB | 110.46 MiB |
| Idle process working set (median) | 127.71 MiB | 106.79 MiB |
| Uncompressed FFmpeg payload | 428.43 MiB | 149.68 MiB |
| Windows Full installer | 120.31 MiB | 42.60 MiB |
| Windows Lite installer | 3.93 MiB | 3.60 MiB |

Memory samples use three separate launches per executable, sampled four seconds after launch with the same saved preferences. The comparison includes the recent main-screen layout change; it is not an isolated font-only benchmark. Measurements exclude recording/FFmpeg processes and GPU memory, and vary with system fonts, drivers and OS caching. No working-set trimming is used to hide resident memory.

Validation: 32 Rust tests and Clippy, four Python staging/discovery tests, Chinese and English UI screenshots, a one-second H.264 test encode using the staged FFmpeg, and successful compilation of both Windows installers. Native macOS/Linux packaging and memory measurements require those platforms. Installer compilation does not establish install/uninstall behavior.
