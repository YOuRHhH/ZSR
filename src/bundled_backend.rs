use std::path::{Path, PathBuf};

/// Locate the installer-managed FFmpeg without depending on PATH or the working directory.
pub fn bundled_ffmpeg() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    locate(executable.parent()?)
}

/// Support a complete FFmpeg distribution with bin/ and a flat runtime bundle.
fn locate(directory: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    [
        directory.join("ffmpeg/bin").join(name),
        directory.join("ffmpeg").join(name),
        directory.join("../lib/zsr/ffmpeg/bin").join(name),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    /// Lite leaves the configured backend alone; Full resolves its private executable.
    #[test]
    fn resolves_only_existing_bundle_executables() {
        let folder = tempfile::tempdir().unwrap();
        assert!(super::locate(folder.path()).is_none());
        let bin = folder.path().join("ffmpeg/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        std::fs::write(bin.join(name), b"fixture").unwrap();
        assert_eq!(super::locate(folder.path()), Some(bin.join(name)));
    }
}
