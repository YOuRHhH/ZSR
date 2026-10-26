use super::{
    api::{AudioInfo, Ptr, VideoInfo},
    runtime::{Runtime, c, text},
};
use std::{
    ptr::null_mut,
    sync::{
        atomic::{AtomicI32, Ordering},
        mpsc::Receiver,
    },
    time::{Duration, Instant},
};
use zsr::recording::{Capture, Request};

/// Own every recording resource and release references in dependency order on every exit path.
struct Recording<'a> {
    rt: &'a Runtime,
    output: Ptr,
    video: Ptr,
    audio: Option<super::audio::Audio<'a>>,
    source: Ptr,
    scene: Ptr,
    stop_state: Box<StopState>,
}
impl Drop for Recording<'_> {
    /// Detach global channels before releasing scenes, sources and encoders.
    fn drop(&mut self) {
        unsafe {
            let a = &self.rt.api;
            if !self.output.is_null() {
                // Disconnect under OBS's signal lock before freeing callback state, including error exits.
                (a.signal_handler_disconnect)(
                    (a.obs_output_get_signal_handler)(self.output),
                    c("stop").as_ptr(),
                    stopped,
                    (&*self.stop_state as *const StopState).cast_mut().cast(),
                );
                if (a.obs_output_active)(self.output) {
                    (a.obs_output_force_stop)(self.output);
                }
                (a.obs_output_release)(self.output);
            }
            (a.obs_set_output_source)(0, null_mut());
            if !self.video.is_null() {
                (a.obs_encoder_release)(self.video);
            }
            drop(self.audio.take());
            if !self.scene.is_null() {
                (a.obs_scene_release)(self.scene);
            }
            if !self.source.is_null() {
                (a.obs_source_release)(self.source);
            }
        }
    }
}
struct StopState {
    code: AtomicI32,
    get: unsafe extern "C" fn(Ptr, *const std::ffi::c_char, Ptr, usize) -> bool,
}
/// Save the asynchronous output result without calling Rust UI code from OBS threads.
unsafe extern "C" fn stopped(data: Ptr, call: Ptr) {
    let state = unsafe { &*(data as *const StopState) };
    let mut code: i64 = -1;
    unsafe {
        (state.get)(call, c("code").as_ptr(), (&mut code as *mut i64).cast(), 8);
    }
    state.code.store(code as i32, Ordering::Release);
}
/// Record on libobs's own video/audio threads and report progress through the helper protocol.
pub fn record(
    rt: &Runtime,
    request: Request,
    path: &str,
    stop: Receiver<()>,
) -> Result<(), String> {
    let a = &rt.api;
    let size = match &request.capture {
        Capture::Screen { source, region } => region
            .map(|r| (r.width as u32, r.height as u32))
            .unwrap_or((source.display.width as u32, source.display.height as u32)),
        _ => (1920, 1080),
    };
    let (width, height) = request.settings.resolution.dimensions().unwrap_or(size);
    let graphics = rt.root.join(if cfg!(windows) {
        "bin/64bit/libobs-d3d11.dll"
    } else if cfg!(target_os = "macos") {
        "lib/libobs-opengl.dylib"
    } else {
        "lib/libobs-opengl.so"
    });
    let graphics = c(graphics.to_string_lossy());
    let mut info = VideoInfo {
        graphics: graphics.as_ptr(),
        fps_num: request.settings.fps,
        fps_den: 1,
        base_width: width / 2 * 2,
        base_height: height / 2 * 2,
        output_width: width / 2 * 2,
        output_height: height / 2 * 2,
        format: 2,
        adapter: 0,
        gpu_conversion: true,
        colorspace: 2,
        range: 1,
        scale_type: 2,
    };
    let status = unsafe { (a.obs_reset_video)(&mut info) };
    if status != 0 {
        return Err(format!("OBS graphics initialization failed: {status}"));
    }
    if !unsafe {
        (a.obs_reset_audio)(&AudioInfo {
            samples: 48000,
            speakers: 2,
        })
    } {
        return Err("OBS audio initialization failed".into());
    }
    let mut rec = Recording {
        rt,
        output: null_mut(),
        video: null_mut(),
        audio: None,
        source: null_mut(),
        scene: null_mut(),
        stop_state: Box::new(StopState {
            code: AtomicI32::new(i32::MIN),
            get: a.calldata_get_data,
        }),
    };
    let (source, crop) = super::source::video(rt, &request)?;
    rec.source = source;
    unsafe {
        rec.scene = (a.obs_scene_create_private)(c("ZSR scene").as_ptr());
        if rec.scene.is_null() {
            return Err("OBS scene creation failed".into());
        }
        let item = (a.obs_scene_add)(rec.scene, rec.source);
        if item.is_null() {
            return Err("OBS could not add capture to scene".into());
        }
        (a.obs_set_output_source)(0, (a.obs_scene_get_source)(rec.scene));
        super::source::layout(
            rt,
            rec.source,
            item,
            crop,
            info.base_width,
            info.base_height,
        )?;
        if matches!(request.capture, Capture::Window(_))
            && request.settings.resolution == zsr::resolution::Resolution::Original
        {
            info.base_width = (a.obs_source_get_width)(rec.source) / 2 * 2;
            info.base_height = (a.obs_source_get_height)(rec.source) / 2 * 2;
            info.output_width = info.base_width;
            info.output_height = info.base_height;
            if (a.obs_reset_video)(&mut info) != 0 {
                return Err("OBS could not size the window recording".into());
            }
            super::source::layout(
                rt,
                rec.source,
                item,
                None,
                info.base_width,
                info.base_height,
            )?;
        }
        rec.audio = Some(super::audio::Audio::new(rt, &request.settings)?);
        let data = rt.data(&serde_json::json!({"path":path,"allow_overwrite":false}));
        rec.output = (a.obs_output_create)(
            c("mp4_output").as_ptr(),
            c("ZSR MP4").as_ptr(),
            data,
            null_mut(),
        );
        (a.obs_data_release)(data);
        if rec.output.is_null() {
            return Err("OBS native MP4 output is unavailable".into());
        }
        (a.signal_handler_connect)(
            (a.obs_output_get_signal_handler)(rec.output),
            c("stop").as_ptr(),
            stopped,
            (&*rec.stop_state as *const StopState).cast_mut().cast(),
        );
        rec.audio.as_ref().unwrap().attach(rec.output);
        rec.video = super::encoding::start(rt, rec.output, &request.settings)?;
        loop {
            if !matches!(
                stop.recv_timeout(Duration::from_millis(500)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ) {
                break;
            }
            let frames = (a.obs_output_get_total_frames)(rec.output);
            let progress = serde_json::json!({"event":"progress","frames":frames,"seconds":f64::from(frames)/f64::from(request.settings.fps),"dropped":(a.obs_output_get_frames_dropped)(rec.output),"render_lagged":(a.obs_get_lagged_frames)(),"encoding_skipped":(a.video_output_get_skipped_frames)((a.obs_get_video)())});
            eprintln!("ZSR metrics: {progress}");
            rec.audio.as_ref().unwrap().report();
            super::emit(progress);
            if !(a.obs_output_active)(rec.output) {
                return Err(format!(
                    "OBS output ended unexpectedly: {}",
                    text((a.obs_output_get_last_error)(rec.output))
                ));
            }
        }
        (a.obs_output_stop)(rec.output);
        let deadline = Instant::now() + Duration::from_secs(15);
        while rec.stop_state.code.load(Ordering::Acquire) == i32::MIN {
            if Instant::now() > deadline {
                return Err("OBS stop timed out; partial recording retained".into());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        let code = rec.stop_state.code.load(Ordering::Acquire);
        if code != 0 {
            return Err(format!(
                "OBS output failed ({code}): {}",
                text((a.obs_output_get_last_error)(rec.output))
            ));
        }
    }
    drop(rec);
    super::emit(serde_json::json!({"event":"complete"}));
    Ok(())
}
