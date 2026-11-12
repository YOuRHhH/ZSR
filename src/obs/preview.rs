//! Low-rate video-only preview, using the recording source and crop implementation.
use super::{
    api::{Ptr, VideoData, VideoInfo},
    runtime::{Runtime, c},
};
use std::{ptr::null_mut, sync::mpsc, time::Duration};
use zsr::{
    preview::Frame,
    recording::{Capture, Request},
};

struct Callback {
    width: u32,
    height: u32,
    frames: mpsc::SyncSender<Frame>,
}
struct Preview<'a> {
    rt: &'a Runtime,
    source: Ptr,
    scene: Ptr,
    callback: Option<Box<Callback>>,
}
impl Drop for Preview<'_> {
    /// Disconnect synchronously before freeing callback data or releasing capture sources.
    fn drop(&mut self) {
        unsafe {
            if let Some(callback) = &mut self.callback {
                (self.rt.api.obs_remove_raw_video_callback)(
                    frame_ready,
                    (&mut **callback as *mut Callback).cast(),
                );
            }
            (self.rt.api.obs_set_output_source)(0, null_mut());
            if !self.scene.is_null() {
                (self.rt.api.obs_scene_release)(self.scene);
            }
            if !self.source.is_null() {
                (self.rt.api.obs_source_release)(self.source);
            }
        }
    }
}

/// Copy rows respecting OBS pitch; never block OBS's callback on pipe IO.
unsafe extern "C" fn frame_ready(data: Ptr, video: *mut VideoData) {
    let callback = unsafe { &*(data as *const Callback) };
    let video = unsafe { &*video };
    let row = callback.width as usize * 4;
    if video.data[0].is_null() || (video.linesize[0] as usize) < row {
        return;
    }
    let mut rgba = vec![0; row * callback.height as usize];
    for y in 0..callback.height as usize {
        let source = unsafe {
            std::slice::from_raw_parts(video.data[0].add(y * video.linesize[0] as usize), row)
        };
        rgba[y * row..(y + 1) * row].copy_from_slice(source);
    }
    // OBS's unconverted graphics surface is BGRA; the UI texture protocol is RGBA.
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let _ = callback.frames.try_send(Frame {
        width: callback.width,
        height: callback.height,
        rgba,
    });
}

/// Fit the preview within its bounds and satisfy OBS's four-pixel output-width alignment.
fn dimensions(width: u32, height: u32) -> (u32, u32) {
    let scale = (640.0 / width.max(2) as f64)
        .min(360.0 / height.max(2) as f64)
        .min(1.0);
    (
        // obs_reset_video rounds widths down to four pixels. A two-pixel width becomes zero.
        ((width as f64 * scale) as u32 / 4 * 4).max(4),
        ((height as f64 * scale) as u32 / 2 * 2).max(2),
    )
}

/// Stream bounded RGBA frames until stdin closes; propagate setup or pipe errors to the parent.
pub fn run(rt: &Runtime, request: &Request, stop: mpsc::Receiver<()>) -> Result<(), String> {
    let a = &rt.api;
    let graphics = c(rt
        .root
        .join(if cfg!(windows) {
            "bin/64bit/libobs-d3d11.dll"
        } else if cfg!(target_os = "macos") {
            "lib/libobs-opengl.dylib"
        } else {
            "lib/libobs-opengl.so"
        })
        .to_string_lossy());
    let mut info = VideoInfo {
        graphics: graphics.as_ptr(),
        fps_num: 5,
        fps_den: 1,
        base_width: 640,
        base_height: 360,
        output_width: 640,
        output_height: 360,
        format: 7, // VIDEO_FORMAT_BGRA matches OBS's unconverted graphics surface.
        adapter: 0,
        gpu_conversion: false,
        colorspace: 2,
        range: 2,
        scale_type: 2,
    };
    if unsafe { (a.obs_reset_video)(&mut info) } != 0 {
        return Err("Could not initialize preview graphics".into());
    }
    let mut preview = Preview {
        rt,
        source: null_mut(),
        scene: null_mut(),
        callback: None,
    };
    let (source, crop) = super::source::video(rt, request)?;
    preview.source = source;
    unsafe {
        preview.scene = (a.obs_scene_create_private)(c("ZSR preview").as_ptr());
        if preview.scene.is_null() {
            return Err("Could not create preview scene".into());
        }
        let item = (a.obs_scene_add)(preview.scene, source);
        if item.is_null() {
            return Err("Could not add preview source".into());
        }
        (a.obs_set_output_source)(0, (a.obs_scene_get_source)(preview.scene));
        super::source::layout(rt, source, item, crop.clone(), 640, 360)?;
        let native = match &request.capture {
            Capture::Screen { source, region } => region
                .map(|r| (r.width as u32, r.height as u32))
                .unwrap_or((source.display.width as u32, source.display.height as u32)),
            _ => (
                (a.obs_source_get_width)(source),
                (a.obs_source_get_height)(source),
            ),
        };
        let target = request.settings.resolution.dimensions().unwrap_or(native);
        let (width, height) = dimensions(target.0, target.1);
        info.base_width = width;
        info.base_height = height;
        info.output_width = width;
        info.output_height = height;
        if (a.obs_reset_video)(&mut info) != 0 {
            return Err("Could not size preview".into());
        }
        // Reuse the recording crop when fitting the final preview canvas.
        super::source::layout(rt, source, item, crop, width, height)?;
        let (tx, rx) = mpsc::sync_channel(1);
        preview.callback = Some(Box::new(Callback {
            // OBS may normalize the requested size; rows and headers must match its actual output.
            width: info.output_width,
            height: info.output_height,
            frames: tx,
        }));
        (a.obs_add_raw_video_callback)(
            null_mut(),
            frame_ready,
            (&mut **preview.callback.as_mut().unwrap() as *mut Callback).cast(),
        );
        let mut stdout = std::io::stdout().lock();
        loop {
            if !matches!(stop.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                break;
            }
            if let Ok(frame) = rx.recv_timeout(Duration::from_millis(100)) {
                frame.write(&mut stdout).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn fits_portrait_landscape_and_tiny_sources() {
        assert_eq!(super::dimensions(1920, 1080), (640, 360));
        assert_eq!(super::dimensions(1080, 1920), (200, 360));
        assert_eq!(super::dimensions(2, 2), (4, 2));
    }

    #[test]
    fn preview_sizes_survive_obs_alignment_without_losing_rows() {
        // Cover arbitrary window widths, including very narrow and portrait captures.
        for width in 2..2048 {
            for height in [2, 101, 360, 640, 1080, 1920] {
                let (width, height) = super::dimensions(width, height);
                assert!((4..=640).contains(&width));
                assert!((2..=360).contains(&height));
                assert_eq!(width & !3, width);
                assert_eq!(height & !1, height);
            }
        }
    }
}
