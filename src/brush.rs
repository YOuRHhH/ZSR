//! Bounded, resolution-independent ink shared by the desktop overlay and OBS.
use serde::{Deserialize, Serialize};
pub mod desktop;

pub const MAX_POINTS: usize = 8192;
pub const MAX_STROKES: usize = 256;
pub const COLOR: [u8; 3] = [255, 70, 60];

/// Shapes store only their drag endpoints; all renderers share the same outline geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    #[default]
    Freehand,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
}

/// Current selections apply to new strokes without recoloring existing annotations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub tool: Tool,
    pub color: [u8; 3],
}
impl Default for Style {
    fn default() -> Self {
        Self {
            tool: Tool::Freehand,
            color: COLOR,
        }
    }
}
fn default_color() -> [u8; 3] {
    COLOR
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ink {
    pub strokes: Vec<Stroke>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stroke {
    /// Coordinates and thickness are relative to the captured image, not the monitor DPI.
    pub points: Vec<[f32; 2]>,
    pub width: f32,
    // Defaults also accept ink produced by older helpers/tests.
    #[serde(default)]
    pub tool: Tool,
    #[serde(default = "default_color")]
    pub color: [u8; 3],
}

impl Ink {
    /// Reject malformed or oversized IPC before it reaches the render thread.
    pub fn valid(&self) -> bool {
        self.strokes.len() <= MAX_STROKES
            && self.point_count() <= MAX_POINTS
            && self.strokes.iter().all(|s| {
                s.width.is_finite()
                    && s.width > 0.0
                    && s.width <= 0.1
                    && (s.tool == Tool::Freehand || s.points.len() <= 2)
                    && s.points
                        .iter()
                        .flatten()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            })
    }

    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(|s| s.points.len()).sum()
    }
}

impl Stroke {
    /// Return pixel-space outline paths for the native overlay, X11 UI and recorded video.
    pub fn paths(&self, width: f32, height: f32) -> Vec<Vec<[f32; 2]>> {
        let points: Vec<_> = self
            .points
            .iter()
            .map(|p| [p[0] * width, p[1] * height])
            .collect();
        let (Some(&a), Some(&b)) = (points.first(), points.last()) else {
            return Vec::new();
        };
        if self.tool == Tool::Freehand || points.len() == 1 || a == b {
            return vec![points];
        }
        match self.tool {
            Tool::Freehand => unreachable!(),
            Tool::Line => vec![vec![a, b]],
            Tool::Arrow => {
                let [dx, dy] = [b[0] - a[0], b[1] - a[1]];
                let length = dx.hypot(dy);
                // Limit the arrowhead on short drags, and size it in physical source pixels.
                let head = (self.width * width.min(height) * 4.0).min(length * 0.4);
                let [ux, uy] = [dx / length, dy / length];
                vec![
                    vec![a, b],
                    vec![
                        [
                            b[0] - ux * head - uy * head * 0.55,
                            b[1] - uy * head + ux * head * 0.55,
                        ],
                        b,
                        [
                            b[0] - ux * head + uy * head * 0.55,
                            b[1] - uy * head - ux * head * 0.55,
                        ],
                    ],
                ]
            }
            Tool::Rectangle => vec![vec![a, [b[0], a[1]], b, [a[0], b[1]], a]],
            Tool::Ellipse => {
                let center = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
                let radius = [(b[0] - a[0]).abs() * 0.5, (b[1] - a[1]).abs() * 0.5];
                // A fixed, bounded tessellation prevents large captures from inflating IPC/render work.
                let mut path: Vec<_> = (0..96)
                    .map(|i| {
                        let angle = i as f32 * std::f32::consts::TAU / 96.0;
                        [
                            center[0] + radius[0] * angle.cos(),
                            center[1] + radius[1] * angle.sin(),
                        ]
                    })
                    .collect();
                path.push(path[0]);
                vec![path]
            }
        }
    }

    /// Build opaque round-ended strokes in source pixels, including single-click dots.
    pub fn triangles(&self, width: f32, height: f32) -> Vec<[f32; 2]> {
        let mut mesh = Vec::new();
        let radius = self.width * width.min(height) * 0.5;
        for points in self.paths(width, height) {
            for &p in &points {
                for i in 0..12 {
                    let angle = |n: usize| n as f32 * std::f32::consts::TAU / 12.0;
                    mesh.extend([
                        p,
                        [
                            p[0] + radius * angle(i).cos(),
                            p[1] + radius * angle(i).sin(),
                        ],
                        [
                            p[0] + radius * angle(i + 1).cos(),
                            p[1] + radius * angle(i + 1).sin(),
                        ],
                    ]);
                }
            }
            for pair in points.windows(2) {
                let [a, b] = [pair[0], pair[1]];
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length = dx.hypot(dy);
                if length < 0.001 {
                    continue;
                }
                let [x, y] = [-dy / length * radius, dx / length * radius];
                let corners = [
                    [a[0] + x, a[1] + y],
                    [a[0] - x, a[1] - y],
                    [b[0] + x, b[1] + y],
                    [b[0] - x, b[1] - y],
                ];
                mesh.extend([
                    corners[0], corners[1], corners[2], corners[2], corners[1], corners[3],
                ]);
            }
        }
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_ipc_and_draws_single_clicks() {
        let mut ink = Ink {
            strokes: vec![Stroke {
                points: vec![[0.5, 0.5]],
                width: 0.01,
                tool: Tool::Freehand,
                color: COLOR,
            }],
        };
        assert!(ink.valid());
        let mesh = ink.strokes[0].triangles(200.0, 100.0);
        assert_eq!(mesh.len(), 36);
        assert!(
            mesh.iter()
                .all(|p| (99.5..=100.5).contains(&p[0]) && (49.5..=50.5).contains(&p[1]))
        );
        ink.strokes[0].points[0][0] = f32::NAN;
        assert!(!ink.valid());
        ink.strokes[0].points = vec![[0.5, 0.5]; MAX_POINTS + 1];
        assert!(!ink.valid());
    }

    #[test]
    fn shapes_share_bounded_geometry_and_roundtrip_colors() {
        for tool in [Tool::Line, Tool::Arrow, Tool::Rectangle, Tool::Ellipse] {
            let mut stroke = Stroke {
                points: vec![[0.8, 0.7], [0.2, 0.3]],
                width: 0.01,
                tool,
                color: [0, 0, 0],
            };
            let paths = stroke.paths(800.0, 400.0);
            assert!(paths.iter().flatten().flatten().all(|v| v.is_finite()));
            match tool {
                Tool::Line => {
                    assert_eq!(paths[0].len(), 2);
                    assert!((paths[0][1][0] - 160.0).abs() < 0.001);
                    assert!((paths[0][1][1] - 120.0).abs() < 0.001);
                }
                Tool::Arrow => {
                    assert_eq!(paths.len(), 2);
                    assert_eq!(paths[1][1], paths[0][1]);
                }
                Tool::Rectangle | Tool::Ellipse => assert_eq!(paths[0].first(), paths[0].last()),
                _ => unreachable!(),
            }
            let ink = Ink {
                strokes: vec![stroke.clone()],
            };
            let decoded: Ink = serde_json::from_str(&serde_json::to_string(&ink).unwrap()).unwrap();
            assert!(decoded.valid());
            assert_eq!(decoded.strokes[0].tool, tool);
            assert_eq!(decoded.strokes[0].color, [0, 0, 0]);
            stroke.points[1] = stroke.points[0];
            assert!(
                stroke
                    .triangles(800.0, 400.0)
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite())
            );
        }
        let legacy: Stroke =
            serde_json::from_str(r#"{"points":[[0.5,0.5]],"width":0.01}"#).unwrap();
        assert_eq!(legacy.tool, Tool::Freehand);
        assert_eq!(legacy.color, COLOR);
    }
}
