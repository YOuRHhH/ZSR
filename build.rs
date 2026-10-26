/// Build native platform adapters and embed the Windows application resources.
fn main() {
    println!("cargo:rerun-if-changed=native/recording_power.c");
    println!("cargo:rerun-if-changed=native/desktop_capture.c");
    println!("cargo:rerun-if-changed=native/display.c");
    println!("cargo:rerun-if-changed=native/display.h");
    println!("cargo:rerun-if-changed=native/windows.c");
    println!("cargo:rerun-if-changed=native/region_marker.c");
    for file in [
        "picker.h",
        "picker_window.c",
        "picker_input.c",
        "picker_paint.c",
        "picker_test.c",
    ] {
        println!("cargo:rerun-if-changed=native/{file}");
    }
    println!("cargo:rerun-if-changed=native/obs_bridge.c");
    println!("cargo:rerun-if-changed=native/window_theme.c");
    cc::Build::new()
        .file("native/window_theme.c")
        .file("native/obs_bridge.c")
        .file("native/recording_power.c")
        .file("native/desktop_capture.c")
        .file("native/display.c")
        .file("native/windows.c")
        .file("native/region_marker.c")
        .file("native/picker_window.c")
        .file("native/picker_input.c")
        .file("native/picker_paint.c")
        .file("native/picker_test.c")
        .warnings(true)
        .compile("zsr_display");
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => {
            println!("cargo:rerun-if-changed=native/desktop_duplication.cpp");
            cc::Build::new()
                .cpp(true)
                .file("native/desktop_duplication.cpp")
                .warnings(true)
                .compile("zsr_duplication");
            println!("cargo:rustc-link-lib=dxgi");
            println!("cargo:rerun-if-changed=native/zsr.rc");
            println!("cargo:rerun-if-changed=logo/windows/zsr.ico");
            embed_resource::compile("native/zsr.rc", embed_resource::NONE)
                .manifest_required()
                .expect("compile ZSR Windows icon resource");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=dwmapi");
            println!("cargo:rustc-link-lib=gdi32");
        }
        Ok("macos") => println!("cargo:rustc-link-lib=framework=CoreGraphics"),
        Ok("linux") => println!("cargo:rustc-link-lib=dl"),
        _ => {}
    }
}
