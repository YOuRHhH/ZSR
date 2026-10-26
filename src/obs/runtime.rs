use super::api::{Api, Ptr};
use std::{
    ffi::{CStr, CString},
    path::{Path, PathBuf},
};

/// Own libobs for one helper process; recording resources must be released before shutdown.
pub struct Runtime {
    pub api: Api,
    pub root: PathBuf,
}
/// Convert trusted configuration strings into NUL-terminated native arguments.
pub fn c(value: impl AsRef<str>) -> CString {
    CString::new(value.as_ref()).expect("validated string contains NUL")
}
/// Copy an optional libobs string while its owner is still alive.
pub unsafe fn text(value: *const std::ffi::c_char) -> String {
    if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    }
}
impl Runtime {
    /// Initialize only capture, encoder and output plugins; never load OBS frontend plugins.
    pub fn new(root: &Path) -> Result<Self, String> {
        let bin = if cfg!(windows) {
            root.join("bin/64bit")
        } else {
            root.join("lib")
        };
        let library = bin.join(if cfg!(windows) {
            "obs.dll"
        } else if cfg!(target_os = "macos") {
            "libobs.dylib"
        } else {
            "libobs.so"
        });
        #[cfg(windows)]
        {
            unsafe extern "system" {
                fn SetDllDirectoryW(path: *const u16) -> i32;
            }
            let wide: Vec<u16> = bin
                .to_string_lossy()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            // Only this helper's DLL search path changes; plugins can resolve their shipped dependencies.
            if unsafe { SetDllDirectoryW(wide.as_ptr()) } == 0 {
                return Err("Cannot set OBS dependency directory".into());
            }
        }
        // SAFETY: API signatures match the pinned libobs 32 public ABI.
        let api = unsafe { Api::load(&library)? };
        unsafe extern "C" {
            fn zsr_obs_log_handler() -> Ptr;
        }
        unsafe {
            (api.base_set_log_handler)(zsr_obs_log_handler(), std::ptr::null_mut());
        }
        let version = unsafe { (api.obs_get_version)() };
        if version >> 24 != 32 {
            return Err(format!(
                "OBS runtime major version 32 required; got {}",
                version >> 24
            ));
        }
        // OBS plugins load sibling runtime DLLs; this child alone owns the working directory.
        std::env::set_current_dir(&bin).map_err(|e| e.to_string())?;
        if !unsafe {
            (api.obs_startup)(c("en-US").as_ptr(), std::ptr::null(), std::ptr::null_mut())
        } {
            return Err("libobs startup failed".into());
        }
        let runtime = Self {
            api,
            root: root.into(),
        };
        unsafe {
            // libobs concatenates filenames directly, so custom data paths need a trailing separator.
            (runtime.api.obs_add_data_path)(
                c(format!("{}/", root.join("data/libobs").to_string_lossy())).as_ptr(),
            );
        }
        let graphics = c(bin
            .join(if cfg!(windows) {
                "libobs-d3d11.dll"
            } else if cfg!(target_os = "macos") {
                "libobs-opengl.dylib"
            } else {
                "libobs-opengl.so"
            })
            .to_string_lossy());
        let mut video = super::api::VideoInfo {
            graphics: graphics.as_ptr(),
            fps_num: 30,
            fps_den: 1,
            base_width: 16,
            base_height: 16,
            output_width: 16,
            output_height: 16,
            format: 2,
            adapter: 0,
            gpu_conversion: true,
            colorspace: 2,
            range: 1,
            scale_type: 2,
        };
        let code = unsafe { (runtime.api.obs_reset_video)(&mut video) };
        if code != 0 {
            return Err(format!("OBS graphics startup failed ({code})"));
        }
        let modules = if cfg!(windows) {
            vec![
                "win-capture",
                "win-wasapi",
                "obs-x264",
                "obs-ffmpeg",
                "obs-outputs",
                "obs-filters",
                "obs-nvenc",
                "obs-qsv11",
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                "mac-capture",
                "obs-x264",
                "obs-ffmpeg",
                "obs-outputs",
                "obs-filters",
            ]
        } else {
            vec![
                "linux-capture",
                "linux-pulseaudio",
                "obs-x264",
                "obs-ffmpeg",
                "obs-outputs",
                "obs-filters",
            ]
        };
        for name in modules {
            let loaded = runtime.load_module(name);
            if !loaded && !["obs-nvenc", "obs-qsv11"].contains(&name) {
                return Err(format!("Required OBS module failed to load: {name}"));
            }
        }
        unsafe {
            (runtime.api.obs_post_load_modules)();
        }
        Ok(runtime)
    }
    /// Load one plugin from the private distribution, leaving optional hardware plugins optional.
    fn load_module(&self, name: &str) -> bool {
        let path = if cfg!(windows) {
            self.root
                .join("obs-plugins/64bit")
                .join(format!("{name}.dll"))
        } else {
            self.root.join("lib/obs-plugins").join(format!("{name}.so"))
        };
        let data = self.root.join("data/obs-plugins").join(name);
        let mut module: Ptr = std::ptr::null_mut();
        unsafe {
            let code = (self.api.obs_open_module)(
                &mut module,
                c(path.to_string_lossy()).as_ptr(),
                c(data.to_string_lossy()).as_ptr(),
            );
            code == 0 && (self.api.obs_init_module)(module)
        }
    }
    /// Build OBS settings without retaining pointers to Rust JSON storage.
    pub fn data(&self, value: &serde_json::Value) -> Ptr {
        unsafe { (self.api.obs_data_create_from_json)(c(value.to_string()).as_ptr()) }
    }
    /// Enumerate OBS device identities rather than DirectShow/FFmpeg identifiers.
    pub fn choices(&self, source: &str, property: &str) -> Vec<serde_json::Value> {
        unsafe {
            let props = (self.api.obs_get_source_properties)(c(source).as_ptr());
            if props.is_null() {
                return vec![];
            }
            let prop = (self.api.obs_properties_get)(props, c(property).as_ptr());
            let mut values = Vec::new();
            if !prop.is_null() {
                for i in 0..(self.api.obs_property_list_item_count)(prop) {
                    values.push(serde_json::json!({"name":text((self.api.obs_property_list_item_name)(prop,i)),"id":text((self.api.obs_property_list_item_string)(prop,i))}));
                }
            }
            (self.api.obs_properties_destroy)(props);
            values
        }
    }
}
impl Drop for Runtime {
    /// Stop libobs threads before the dynamically loaded library is unloaded.
    fn drop(&mut self) {
        unsafe {
            (self.api.obs_wait_for_destroy_queue)();
            (self.api.obs_shutdown)();
        }
    }
}
/// Select the platform's native OBS microphone plugin.
pub fn audio_source() -> &'static str {
    if cfg!(windows) {
        "wasapi_input_capture"
    } else if cfg!(target_os = "macos") {
        "coreaudio_input_capture"
    } else {
        "pulse_input_capture"
    }
}
