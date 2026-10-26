//! Minimal dynamically loaded libobs 32 ABI; no OBS frontend or Qt dependency.
use std::ffi::{c_char, c_void};
pub type Ptr = *mut c_void;
pub type MeterCallback = unsafe extern "C" fn(Ptr, *const f32, *const f32, *const f32);
#[repr(C)]
pub struct VideoInfo {
    pub graphics: *const c_char,
    pub fps_num: u32,
    pub fps_den: u32,
    pub base_width: u32,
    pub base_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub format: i32,
    pub adapter: u32,
    pub gpu_conversion: bool,
    pub colorspace: i32,
    pub range: i32,
    pub scale_type: i32,
}
#[repr(C)]
pub struct AudioInfo {
    pub samples: u32,
    pub speakers: i32,
}
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
#[repr(C)]
pub struct Crop {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// All symbols are resolved before calling into OBS; the library outlives every function pointer.
macro_rules! api {
    ($($name:ident($($arg:ty),*) -> $ret:ty;)+) => {
        pub struct Api { $(pub $name: unsafe extern "C" fn($($arg),*) -> $ret,)+ _library:libloading::Library }
        impl Api {
            /// Load the selected runtime and reject missing ABI symbols before startup.
            pub unsafe fn load(path:&std::path::Path) -> Result<Self,String> {
                #[cfg(windows)]
                let library:libloading::Library = unsafe { libloading::os::windows::Library::load_with_flags(path, 0x100 | 0x1000) }.map_err(|e|e.to_string())?.into();
                #[cfg(not(windows))]
                let library = unsafe { libloading::Library::new(path) }.map_err(|e|e.to_string())?;
                Ok(Self { $($name: *unsafe { library.get(concat!(stringify!($name), "\0").as_bytes()) }.map_err(|e|format!("{}: {e}",stringify!($name)))?,)+ _library:library })
            }
        }
    }
}
api! {
    base_set_log_handler(Ptr,Ptr) -> ();
    obs_get_version() -> u32;
    obs_startup(*const c_char,*const c_char,Ptr) -> bool;
    obs_wait_for_destroy_queue() -> bool;
    obs_shutdown() -> ();
    obs_add_data_path(*const c_char) -> ();
    obs_reset_video(*mut VideoInfo) -> i32;
    obs_reset_audio(*const AudioInfo) -> bool;
    obs_open_module(*mut Ptr,*const c_char,*const c_char) -> i32;
    obs_init_module(Ptr) -> bool;
    obs_post_load_modules() -> ();
    obs_data_create_from_json(*const c_char) -> Ptr;
    obs_data_release(Ptr) -> ();
    obs_source_create(*const c_char,*const c_char,Ptr,Ptr) -> Ptr;
    obs_source_release(Ptr) -> ();
    obs_source_set_audio_mixers(Ptr,u32) -> ();
    obs_source_get_audio_mixers(Ptr) -> u32;
    obs_source_get_volume(Ptr) -> f32;
    obs_source_muted(Ptr) -> bool;
    obs_volmeter_create(i32) -> Ptr;
    obs_volmeter_destroy(Ptr) -> ();
    obs_volmeter_attach_source(Ptr,Ptr) -> bool;
    obs_volmeter_add_callback(Ptr,MeterCallback,Ptr) -> ();
    obs_volmeter_remove_callback(Ptr,MeterCallback,Ptr) -> ();
    obs_encoder_get_mixer_index(Ptr) -> usize;
    obs_source_get_width(Ptr) -> u32;
    obs_source_get_height(Ptr) -> u32;
    obs_set_output_source(u32,Ptr) -> ();
    obs_get_source_properties(*const c_char) -> Ptr;
    obs_properties_get(Ptr,*const c_char) -> Ptr;
    obs_property_list_item_count(Ptr) -> usize;
    obs_property_list_item_name(Ptr,usize) -> *const c_char;
    obs_property_list_item_string(Ptr,usize) -> *const c_char;
    obs_properties_destroy(Ptr) -> ();
    obs_scene_create_private(*const c_char) -> Ptr;
    obs_scene_get_source(Ptr) -> Ptr;
    obs_scene_release(Ptr) -> ();
    obs_scene_add(Ptr,Ptr) -> Ptr;
    obs_sceneitem_set_crop(Ptr,*const Crop) -> ();
    obs_sceneitem_set_bounds_type(Ptr,i32) -> ();
    obs_sceneitem_set_bounds(Ptr,*const Vec2) -> ();
    obs_sceneitem_set_bounds_alignment(Ptr,u32) -> ();
    obs_video_encoder_create(*const c_char,*const c_char,Ptr,Ptr) -> Ptr;
    obs_audio_encoder_create(*const c_char,*const c_char,Ptr,usize,Ptr) -> Ptr;
    obs_encoder_release(Ptr) -> ();
    obs_encoder_set_video(Ptr,Ptr) -> ();
    obs_encoder_set_audio(Ptr,Ptr) -> ();
    obs_get_video() -> Ptr;
    obs_get_lagged_frames() -> u32;
    video_output_get_skipped_frames(Ptr) -> u32;
    obs_get_audio() -> Ptr;
    obs_output_create(*const c_char,*const c_char,Ptr,Ptr) -> Ptr;
    obs_output_release(Ptr) -> ();
    obs_output_set_video_encoder(Ptr,Ptr) -> ();
    obs_output_set_audio_encoder(Ptr,Ptr,usize) -> ();
    obs_output_start(Ptr) -> bool;
    obs_output_stop(Ptr) -> ();
    obs_output_force_stop(Ptr) -> ();
    obs_output_active(Ptr) -> bool;
    obs_output_get_total_frames(Ptr) -> i32;
    obs_output_get_frames_dropped(Ptr) -> i32;
    obs_output_get_signal_handler(Ptr) -> Ptr;
    signal_handler_connect(Ptr,*const c_char,unsafe extern "C" fn(Ptr,Ptr),Ptr) -> ();
    signal_handler_disconnect(Ptr,*const c_char,unsafe extern "C" fn(Ptr,Ptr),Ptr) -> ();
    calldata_get_data(Ptr,*const c_char,Ptr,usize) -> bool;
    obs_output_get_last_error(Ptr) -> *const c_char;
}
