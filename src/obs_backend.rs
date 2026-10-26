//! Discovery and launching of ZSR's isolated libobs helper.
use std::path::{Path, PathBuf};
/// Prefer a private runtime, with an explicit environment override for development.
pub fn default_runtime() -> String {
    if let Some(path) = std::env::var_os("ZSR_OBS_RUNTIME") {
        return path.to_string_lossy().into_owned();
    }
    let exe = std::env::current_exe().ok();
    let candidates = [
        exe.as_deref()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .join("obs"),
        exe.as_deref()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."))
            .join("../lib/zsr/obs"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(target_os = "linux") {
            "target/obs-runtime/32.2.2-linux"
        } else {
            "target/obs-runtime/32.2.2"
        }),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}
/// Resolve the runtime before spawning; never search PATH for a differently versioned engine.
pub fn resolve(configured: &str) -> Result<PathBuf, String> {
    let root = Path::new(configured).canonicalize().map_err(|_| {
        "OBS runtime unavailable. Set the OBS runtime folder in Settings.".to_string()
    })?;
    let library = if cfg!(windows) {
        "bin/64bit/obs.dll"
    } else if cfg!(target_os = "macos") {
        "lib/libobs.dylib"
    } else {
        "lib/libobs.so"
    };
    if !root.join(library).is_file() {
        return Err(format!("OBS runtime is missing {library}"));
    }
    Ok(root)
}
/// Keep the helper beside OBS's own hardware capability probes and dependencies.
pub fn helper(root: &Path) -> Result<PathBuf, String> {
    let path = root.join(if cfg!(windows) {
        "bin/64bit/zsr-obs.exe"
    } else {
        "bin/zsr-obs"
    });
    if !path.is_file() {
        if let Ok(exe) = std::env::current_exe() {
            let sibling = exe.with_file_name(if cfg!(windows) {
                "zsr-obs.exe"
            } else {
                "zsr-obs"
            });
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
        return Err(
            "ZSR OBS helper is missing. Run scripts/stage_obs.py after cargo build.".into(),
        );
    }
    Ok(path)
}

/// Give the isolated Linux helper access to private OBS shared-library dependencies.
pub fn command(root: &Path) -> Result<std::process::Command, String> {
    let helper = helper(root)?;
    let mut command = crate::platform::command(helper.to_str().ok_or("Invalid helper path")?);
    if cfg!(target_os = "linux") {
        let mut paths = vec![root.join("lib")];
        if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        command.env(
            "LD_LIBRARY_PATH",
            std::env::join_paths(paths).map_err(|e| e.to_string())?,
        );
    }
    command.arg(root);
    Ok(command)
}
/// Query native audio properties in a bounded child, keeping OBS initialization off the UI thread.
pub fn probe(configured: &str) -> Result<serde_json::Value, String> {
    let root = resolve(configured)?;
    let output = crate::platform::output(
        command(&root)?.arg("probe"),
        std::time::Duration::from_secs(20),
    )?;
    for line in output.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value["event"] == "probe" {
                return Ok(value);
            }
            if value["event"] == "error" {
                return Err(value["detail"]
                    .as_str()
                    .unwrap_or("OBS startup failed")
                    .into());
            }
        }
    }
    Err(format!(
        "OBS did not return device information: {}",
        output
            .chars()
            .rev()
            .take(1000)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    ))
}
