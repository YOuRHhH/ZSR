mod api;
mod audio;
mod audio_meter;
mod encoding;
mod engine;
mod runtime;
mod source;
use std::io::{BufRead, Write};
/// Emit only JSON on stdout; native OBS diagnostics are redirected to stderr.
fn emit(value: serde_json::Value) {
    println!("{value}");
    let _ = std::io::stdout().flush();
}
/// Run an isolated native engine without constructing an eframe window.
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let root = std::path::Path::new(args.get(1).ok_or("OBS runtime directory required")?)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let normalized = root.to_string_lossy().replace('\\', "/");
    let root = std::path::PathBuf::from(normalized.trim_start_matches("//?/"));
    let rt = runtime::Runtime::new(&root)?;
    if args.get(2).is_some_and(|a| a == "probe") {
        emit(
            serde_json::json!({"event":"probe","audio":rt.choices(runtime::audio_source(),"device_id")}),
        );
        return Ok(());
    }
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&input).map_err(|e| e.to_string())?;
    let request: zsr::recording::Request =
        serde_json::from_value(value["request"].clone()).map_err(|e| e.to_string())?;
    // Validate again at the process boundary before passing dimensions and rates into native code.
    request.validate()?;
    let output = value["output"]
        .as_str()
        .ok_or("output path missing")?
        .to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = std::io::stdin().lock().lines().next();
        let _ = tx.send(());
    });
    engine::record(&rt, request, &output, rx)
}
/// Surface initialization failures to the parent without a console dialog.
fn main() {
    if let Err(error) = run() {
        emit(serde_json::json!({"event":"error","detail":error}));
        std::process::exit(1);
    }
}
