use super::{Capture, Request};

/// Select DXGI desktop duplication without assuming that monitor IDs are output indexes.
pub(super) fn input(request: &Request) -> Result<Option<super::Input>, String> {
    let Capture::Screen { source, region } = &request.capture else {
        return Ok(None);
    };
    let help = crate::platform::output(
        crate::platform::command(&request.settings.ffmpeg).args([
            "-hide_banner",
            "-h",
            "filter=ddagrab",
        ]),
        std::time::Duration::from_secs(5),
    )?;
    if !help.contains("Filter ddagrab")
        || ![
            "output_idx",
            "framerate",
            "draw_mouse",
            "video_size",
            "offset_x",
            "offset_y",
            "output_fmt",
        ]
        .iter()
        .all(|option| {
            help.lines()
                .any(|line| line.split_whitespace().next() == Some(option))
        })
    {
        return Ok(None);
    }
    unsafe extern "C" {
        fn zsr_duplication_output(
            display: *const crate::platform::Display,
            adapter: *mut u32,
            output: *mut u32,
        ) -> i32;
    }
    let (mut adapter, mut output) = (0, 0);
    // SAFETY: The adapter reads one display and writes two valid u32 outputs, retaining neither.
    if unsafe { zsr_duplication_output(&source.display, &mut adapter, &mut output) } == 0 {
        return Ok(None);
    }
    let region = region.unwrap_or(crate::recording::Region {
        x: 0,
        y: 0,
        width: source.display.width,
        height: source.display.height,
    });
    Ok(Some(super::Input {
        graph: format!(
            "ddagrab=output_idx={output}:framerate={}:draw_mouse={}:video_size={}x{}:offset_x={}:offset_y={}:output_fmt=bgra,hwdownload,format=bgra",
            request.settings.fps,
            u8::from(request.settings.cursor),
            region.width,
            region.height,
            region.x,
            region.y
        ),
        device: Some(format!("d3d11va=zsr_capture:{adapter}")),
    }))
}
