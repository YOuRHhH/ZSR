use super::{
    api::Ptr,
    runtime::{Runtime, c, text},
};
use std::ptr::null_mut;
/// Try native hardware encoders, then software, retaining only the successfully started encoder.
pub fn start(rt: &Runtime, output: Ptr, settings: &zsr::settings::Settings) -> Result<Ptr, String> {
    let a = &rt.api;
    // SAFETY: output is owned by the recording session; failed encoder references are detached first.
    unsafe {
        let candidates = if settings.hardware_encoding {
            vec![
                "h264_texture_amf",
                "obs_nvenc_h264_tex",
                "obs_qsv11_v2",
                "obs_x264",
            ]
        } else {
            vec!["obs_x264"]
        };
        for id in candidates {
            let quality = settings.crf;
            let preset = match id {
                "obs_x264" => "ultrafast",
                "obs_nvenc_h264_tex" => "p1",
                _ => "speed",
            };
            let data = rt.data(&serde_json::json!({
                "rate_control": if id == "obs_x264" { "CRF" } else { "CQP" },
                "crf": quality, "cqp": quality, "qp": quality,
                "qpi": quality, "qpp": quality, "qpb": quality,
                "preset": preset, "target_usage": "TU7", "keyint_sec": 2, "bf": 0,
            }));
            let video = (a.obs_video_encoder_create)(
                c(id).as_ptr(),
                c("ZSR H264").as_ptr(),
                data,
                null_mut(),
            );
            (a.obs_data_release)(data);
            if video.is_null() {
                continue;
            }
            (a.obs_encoder_set_video)(video, (a.obs_get_video)());
            (a.obs_output_set_video_encoder)(output, video);
            if (a.obs_output_start)(output) {
                super::emit(serde_json::json!({"event":"started","encoder":id}));
                return Ok(video);
            }
            (a.obs_output_set_video_encoder)(output, null_mut());
            (a.obs_encoder_release)(video);
        }
        Err(format!(
            "OBS encoder/output initialization failed: {}",
            text((a.obs_output_get_last_error)(output))
        ))
    }
}
