//! Convert overlay coordinates into display-relative capture pixels.
use crate::recording::Region;

pub fn from_points(
    start: [f32; 2],
    end: [f32; 2],
    canvas: [f32; 2],
    pixels: [i32; 2],
) -> Option<Region> {
    if canvas.iter().any(|n| !n.is_finite() || *n <= 0.0)
        || start.iter().chain(end.iter()).any(|n| !n.is_finite())
        || pixels.iter().any(|n| *n < 2)
    {
        return None;
    }
    let axis = |axis: usize| {
        let scale = pixels[axis] as f32 / canvas[axis];
        // Suppress sub-millipixel floating-point noise when reusing a frame.
        let a = (start[axis].min(end[axis]) * scale + 0.001)
            .floor()
            .clamp(0.0, pixels[axis] as f32) as i32;
        let b = (start[axis].max(end[axis]) * scale - 0.001)
            .ceil()
            .clamp(0.0, pixels[axis] as f32) as i32;
        (a, b - a)
    };
    let (x, width) = axis(0);
    let (y, height) = axis(1);
    (width >= 2 && height >= 2).then_some(Region {
        x,
        y,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reopening_a_scaled_selection_preserves_exact_pixel_bounds() {
        let sx = 1040.0_f32 / 1920.0;
        let sy = 760.0_f32 / 1080.0;
        let r = from_points(
            [240.0 * sx, 180.0 * sy],
            [1520.0 * sx, 900.0 * sy],
            [1040.0, 760.0],
            [1920, 1080],
        )
        .unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (240, 180, 1280, 720));
    }
    #[test]
    fn reversed_drag_and_dpi_are_converted_to_capture_pixels() {
        let r = from_points([700.0, 400.0], [100.0, 50.0], [1280.0, 720.0], [1920, 1080]).unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (150, 75, 900, 525));
    }
    #[test]
    fn selection_is_clamped_to_display_and_never_adds_global_origin() {
        let r = from_points(
            [-50.0, -20.0],
            [5000.0, 5000.0],
            [960.0, 540.0],
            [1920, 1080],
        )
        .unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (0, 0, 1920, 1080));
    }
    #[test]
    fn invalid_and_subpixel_selections_are_rejected() {
        for (a, b, canvas) in [
            ([0.0, 0.0], [0.2, 0.2], [100.0, 100.0]),
            ([f32::NAN, 0.0], [10.0, 10.0], [100.0, 100.0]),
            ([0.0, 0.0], [10.0, 10.0], [0.0, 100.0]),
        ] {
            assert!(from_points(a, b, canvas, [100, 100]).is_none());
        }
    }
}
