//! Drawing uses a native Windows worker or an X11 viewport, independent of main-window input.
#[cfg(windows)]
mod windows;
use super::{App, State};
use eframe::egui::{self, Color32, ViewportCommand, ViewportId};
use std::sync::{Arc, Mutex};
use zsr::{
    brush::{Ink, MAX_POINTS, MAX_STROKES, Stroke, Style, Tool, desktop},
    recording::{Capture, Region},
};

#[derive(Default)]
pub(super) struct Brush {
    target: Option<Capture>,
    canvas: Arc<Mutex<Canvas>>,
    area: Option<Region>,
    next_geometry: Option<std::time::Instant>,
    tools_open: bool,
    #[cfg(windows)]
    native: Option<windows::Overlay>,
}

#[derive(Default)]
struct Canvas {
    ink: Ink,
    style: Style,
    enabled: bool,
    drawing: bool,
    dirty: bool,
    #[cfg(windows)]
    repaint: bool,
    #[cfg(windows)]
    native_area: Option<Region>,
    #[cfg(windows)]
    tools_title: Vec<u16>,
    focus: bool,
    previous_focus: u64,
    restore_focus: Option<u64>,
    open: bool,
    #[cfg(not(windows))]
    placed: Option<(i32, i32, i32, i32)>,
}

impl Canvas {
    /// Share the shortcut/button transition across backends and publish an empty canvas on exit.
    fn toggle(&mut self, foreground: u64) {
        if self.enabled {
            self.exit();
        } else {
            self.previous_focus = foreground;
            self.enabled = true;
            self.focus = true;
        }
    }
    /// Closing the floating tools has the same clear-and-exit behavior as the shortcut.
    fn exit(&mut self) {
        self.disable();
        self.ink.strokes.clear();
        // Clearing must reach OBS even after it consumed the last drawing update.
        self.changed();
    }
    /// Return keyboard and pointer control without erasing annotations from the recording.
    fn disable(&mut self) {
        if self.enabled {
            // Native focus changes can synchronously re-enter rendering on Windows.
            // Queue them until the caller releases the canvas lock.
            self.restore_focus = Some(self.previous_focus);
        }
        self.enabled = false;
        self.drawing = false;
        self.focus = false;
    }
    /// Windows owns focus on its overlay thread; other backends restore it after unlocking.
    fn take_restore(&mut self) -> Option<u64> {
        let previous = self.restore_focus.take();
        if cfg!(windows) { None } else { previous }
    }
    fn changed(&mut self) {
        self.dirty = true;
        #[cfg(windows)]
        {
            self.repaint = true;
        }
    }
    fn begin_stroke(&mut self, point: [f32; 2], area: Region) {
        if self.ink.strokes.len() >= MAX_STROKES || self.ink.point_count() >= MAX_POINTS {
            return;
        }
        self.ink.strokes.push(Stroke {
            points: vec![point],
            width: (4.0 / area.width.min(area.height) as f32).min(0.1),
            tool: self.style.tool,
            color: self.style.color,
        });
        self.drawing = true;
        self.changed();
    }
    fn extend_stroke(&mut self, point: [f32; 2], area: Region) {
        if !self.drawing {
            return;
        }
        let full = self.ink.point_count() >= MAX_POINTS;
        if let Some(stroke) = self.ink.strokes.last_mut() {
            let last = *stroke.points.last().unwrap();
            // A shape previews its current endpoint instead of accumulating a freehand trail.
            // Replacing an existing endpoint remains possible even at the point limit.
            if stroke.tool != Tool::Freehand {
                if point != last && !(full && stroke.points.len() == 1) {
                    if stroke.points.len() == 1 {
                        stroke.points.push(point);
                    } else {
                        stroke.points[1] = point;
                    }
                    self.changed();
                }
                return;
            }
            let distance = ((point[0] - last[0]) * area.width as f32)
                .hypot((point[1] - last[1]) * area.height as f32);
            if !full && distance >= 2.0 {
                stroke.points.push(point);
                self.changed();
            }
        }
    }
}

impl Brush {
    pub(super) fn new(target: Capture) -> Self {
        Self {
            target: Some(target),
            ..Default::default()
        }
    }

    /// Create a separate movable tool window only while shortcut-activated drawing is enabled.
    /// Stopping, losing the capture target, Escape and the second shortcut all hide it.
    fn show_tools(&mut self, ctx: &egui::Context, lang: zsr::settings::Language, shortcut: &str) {
        let id = ViewportId::from_hash_of("brush_tools");
        let enabled = self.canvas.lock().unwrap().enabled;
        let Some(area) = self.area.filter(|_| enabled) else {
            if self.tools_open {
                ctx.send_viewport_cmd_to(id, ViewportCommand::Close);
            }
            self.tools_open = false;
            return;
        };
        let title = format!("ZSR — {}", zsr::i18n::tr(lang, "brush_panel"));
        #[cfg(windows)]
        {
            self.canvas.lock().unwrap().tools_title = title.encode_utf16().chain(Some(0)).collect();
        }
        let mut builder = egui::ViewportBuilder::default()
            .with_title(&title)
            .with_inner_size([440.0, 230.0])
            .with_min_inner_size([340.0, 200.0])
            .with_always_on_top()
            .with_taskbar(false)
            .with_minimize_button(false)
            .with_maximize_button(false)
            .with_active(false);
        if !self.tools_open {
            let scale = ctx.pixels_per_point();
            builder =
                builder.with_position([area.x as f32 / scale + 16.0, area.y as f32 / scale + 32.0]);
        }
        self.tools_open = true;
        let shared = Arc::clone(&self.canvas);
        let shortcut = shortcut.to_owned();
        ctx.show_viewport_deferred(id, builder, move |ctx, class| {
            // Never fall back to embedding the tools in the application's main interface.
            if class == egui::ViewportClass::Embedded {
                return;
            }
            let mut canvas = shared.lock().unwrap();
            if !canvas.enabled {
                ctx.send_viewport_cmd(ViewportCommand::Close);
                return;
            }
            if ctx.input(|i| i.viewport().close_requested()) {
                canvas.exit();
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                canvas.disable();
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if control_panel(ui, &mut canvas, lang, &shortcut) {
                        canvas.exit();
                    }
                });
            });
            let enabled = canvas.enabled;
            if canvas.dirty || !enabled {
                ctx.request_repaint_of(ViewportId::ROOT);
            }
            let restore = canvas.take_restore();
            drop(canvas);
            // Native focus/stacking operations must not hold a canvas lock shared with input.
            if let Some(window) = restore {
                desktop::restore(window);
            }
            if !enabled {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            #[cfg(target_os = "linux")]
            if enabled
                && !desktop::tools_above(&format!("ZSR_BRUSH_{}", std::process::id()), &title)
            {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        });
    }
}

impl App {
    /// Toggle drawing during a confirmed recording; toggling off also clears published ink.
    pub(super) fn toggle_brush(&mut self) {
        if self.state != State::Recording || !zsr::windows::supported() {
            return;
        }
        let foreground = desktop::focused();
        let mut canvas = self.brush.canvas.lock().unwrap();
        canvas.toggle(foreground);
        let restore = canvas.take_restore();
        drop(canvas);
        if let Some(window) = restore {
            desktop::restore(window);
        }
    }

    /// Keep the overlay at the selected capture bounds and relay only changed ink to OBS.
    pub(super) fn update_brush(&mut self, ctx: &egui::Context) {
        self.update_canvas(ctx);
        self.brush
            .show_tools(ctx, self.settings.language, &self.settings.brush_shortcut);
    }

    fn update_canvas(&mut self, ctx: &egui::Context) {
        #[cfg(not(windows))]
        let id = ViewportId::from_hash_of("recording_brush");
        if self.state != State::Recording {
            #[cfg(windows)]
            self.brush.native.take();
            let mut canvas = self.brush.canvas.lock().unwrap();
            canvas.disable();
            #[cfg(not(windows))]
            if canvas.open {
                ctx.send_viewport_cmd_to(id, ViewportCommand::Close);
            }
            let restore = canvas.take_restore();
            *canvas = Canvas::default();
            drop(canvas);
            if let Some(window) = restore {
                desktop::restore(window);
            }
            return;
        }
        let now = std::time::Instant::now();
        if self.brush.next_geometry.is_none_or(|next| now >= next) {
            self.brush.area = self.brush.target.as_ref().and_then(desktop::bounds);
            self.brush.next_geometry = Some(now + std::time::Duration::from_millis(250));
        }
        let mut canvas = self.brush.canvas.lock().unwrap();
        if canvas.dirty {
            // Window capture excludes our separate desktop overlay, so OBS composites its ink.
            if matches!(self.brush.target, Some(Capture::Window(_)))
                && let Some(session) = &self.session
            {
                session.set_ink(canvas.ink.clone());
            }
            canvas.dirty = false;
        }
        let show = self.brush.area.is_some() && (canvas.enabled || !canvas.ink.strokes.is_empty());
        if !show {
            if self.brush.area.is_none() {
                canvas.disable();
            }
            #[cfg(not(windows))]
            if canvas.open {
                ctx.send_viewport_cmd_to(id, ViewportCommand::Close);
            }
            canvas.open = false;
            #[cfg(not(windows))]
            {
                canvas.placed = None;
            }
            #[cfg(windows)]
            self.brush.native.take();
            let restore = canvas.take_restore();
            drop(canvas);
            if let Some(window) = restore {
                desktop::restore(window);
            }
            return;
        }
        canvas.open = true;
        drop(canvas);
        let area = self.brush.area.unwrap();
        self.show_brush(ctx, area);
    }

    #[cfg(windows)]
    fn show_brush(&mut self, ctx: &egui::Context, area: Region) {
        self.brush.canvas.lock().unwrap().native_area = Some(area);
        if self.brush.native.is_none() {
            match windows::Overlay::start(Arc::clone(&self.brush.canvas), ctx.clone()) {
                Ok(overlay) => self.brush.native = Some(overlay),
                Err(error) => {
                    self.detail = error;
                    self.brush.canvas.lock().unwrap().disable();
                }
            }
        }
        if let Some(error) = self.brush.native.as_ref().and_then(windows::Overlay::error) {
            self.detail = error;
            let mut canvas = self.brush.canvas.lock().unwrap();
            canvas.disable();
            canvas.open = false;
            canvas.ink.strokes.clear();
            canvas.changed();
            self.brush.native.take();
        }
    }

    #[cfg(not(windows))]
    fn show_brush(&mut self, ctx: &egui::Context, area: Region) {
        let id = ViewportId::from_hash_of("recording_brush");
        let enabled = self.brush.canvas.lock().unwrap().enabled;
        let scale = ctx.pixels_per_point();
        let title = format!("ZSR_BRUSH_{}", std::process::id());
        let builder = egui::ViewportBuilder::default()
            .with_title(&title)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            // Only an explicit brush toggle should take focus, not a passive ink repaint.
            .with_active(false)
            .with_has_shadow(false)
            .with_mouse_passthrough(!enabled)
            .with_position([area.x as f32 / scale, area.y as f32 / scale])
            .with_inner_size([area.width as f32 / scale, area.height as f32 / scale]);
        let shared = Arc::clone(&self.brush.canvas);
        ctx.show_viewport_deferred(id, builder, move |ctx, class| {
            if class == egui::ViewportClass::Embedded {
                return;
            }
            let mut canvas = shared.lock().unwrap();
            if !canvas.open {
                ctx.send_viewport_cmd(ViewportCommand::Close);
                return;
            }
            let bounds = (area.x, area.y, area.width, area.height);
            // Repeated SetWindowPos calls can generate resize/repaint events even at the same size.
            let place = canvas.placed != Some(bounds);
            if place {
                canvas.placed = Some(bounds);
            }
            let focus = std::mem::take(&mut canvas.focus);
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                canvas.disable();
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            }
            paint(ctx, &mut canvas, area);
            // The main window already polls shortcuts. Avoid an endless repaint loop
            // between two viewports when the user is not drawing.
            if canvas.dirty || canvas.enabled != enabled {
                ctx.request_repaint_of(ViewportId::ROOT);
            }
            let enabled = canvas.enabled;
            let restore = canvas.take_restore();
            drop(canvas);
            // SetWindowPos and SetForegroundWindow can dispatch synchronous window events.
            // Never call them while rendering could encounter our locked canvas.
            if place {
                desktop::place(&title, area);
            }
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(!enabled));
            if focus && enabled {
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            if let Some(window) = restore {
                desktop::restore(window);
            }
        });
    }
}

/// A wrapping panel stays usable in narrow app windows and both supported UI languages.
fn control_panel(
    ui: &mut egui::Ui,
    canvas: &mut Canvas,
    lang: zsr::settings::Language,
    shortcut: &str,
) -> bool {
    use zsr::i18n::tr;
    let mut toggle = false;
    ui.add_space(6.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong(tr(lang, "brush_panel"));
            let label = tr(lang, "brush_exit");
            toggle = ui.button(format!("{label} ({shortcut})")).clicked();
            let has_ink = !canvas.ink.strokes.is_empty();
            if ui
                .add_enabled(has_ink, egui::Button::new(tr(lang, "brush_undo")))
                .clicked()
            {
                canvas.ink.strokes.pop();
                canvas.drawing = false;
                canvas.changed();
            }
            if ui
                .add_enabled(has_ink, egui::Button::new(tr(lang, "brush_clear")))
                .clicked()
            {
                canvas.ink.strokes.clear();
                canvas.drawing = false;
                canvas.changed();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr(lang, "brush_shape"));
            for (tool, key) in [
                (Tool::Freehand, "brush_freehand"),
                (Tool::Line, "brush_line"),
                (Tool::Arrow, "brush_arrow"),
                (Tool::Rectangle, "brush_rectangle"),
                (Tool::Ellipse, "brush_ellipse"),
            ] {
                ui.selectable_value(&mut canvas.style.tool, tool, tr(lang, key));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr(lang, "brush_color"));
            for (rgb, key) in [
                (zsr::brush::COLOR, "brush_red"),
                ([255, 210, 40], "brush_yellow"),
                ([50, 210, 100], "brush_green"),
                ([40, 200, 255], "brush_blue"),
                ([190, 110, 255], "brush_purple"),
                ([255, 255, 255], "brush_white"),
                ([0, 0, 0], "brush_black"),
            ] {
                let selected = canvas.style.color == rgb;
                let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                let contrast = if u16::from(rgb[0]) + u16::from(rgb[1]) + u16::from(rgb[2]) > 380 {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                };
                let button = egui::Button::new(
                    egui::RichText::new(if selected { "✓" } else { " " }).color(contrast),
                )
                .fill(color)
                .min_size(egui::vec2(26.0, 24.0))
                .stroke(egui::Stroke::new(
                    if selected { 2.0_f32 } else { 1.0_f32 },
                    Color32::GRAY,
                ));
                let response = ui.add(button).on_hover_text(tr(lang, key));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        true,
                        selected,
                        tr(lang, key),
                    )
                });
                if response.clicked() {
                    canvas.style.color = rgb;
                }
            }
            ui.label(tr(lang, "brush_custom_color"));
            ui.color_edit_button_srgb(&mut canvas.style.color)
                .on_hover_text(tr(lang, "brush_custom_color"));
        });
        ui.small(tr(lang, "brush_help"));
        if canvas.ink.point_count() >= MAX_POINTS || canvas.ink.strokes.len() >= MAX_STROKES {
            ui.colored_label(Color32::YELLOW, tr(lang, "brush_limit"));
        }
    });
    toggle
}

/// Paint and sample in capture-relative coordinates, including click-only marks.
#[cfg(any(not(windows), test))]
fn paint(ctx: &egui::Context, canvas: &mut Canvas, area: Region) {
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    if canvas.enabled {
        // A nearly transparent fill keeps Windows hit-testing active between strokes.
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(1));
        ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z)) {
            canvas.ink.strokes.pop();
            canvas.drawing = false;
            canvas.changed();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
            canvas.ink.strokes.clear();
            canvas.drawing = false;
            canvas.changed();
        }
        ctx.input(|i| {
            if let Some(pos) = i.pointer.interact_pos() {
                let point = [
                    ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
                    ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0),
                ];
                if i.pointer.primary_pressed() && rect.contains(pos) {
                    canvas.begin_stroke(point, area);
                }
                if canvas.drawing {
                    canvas.extend_stroke(point, area);
                }
            }
            if !i.pointer.primary_down() {
                canvas.drawing = false;
            }
        });
    }
    for stroke in &canvas.ink.strokes {
        let width = stroke.width * rect.width().min(rect.height());
        let color = Color32::from_rgb(stroke.color[0], stroke.color[1], stroke.color[2]);
        for path in stroke.paths(rect.width(), rect.height()) {
            let points: Vec<_> = path
                .iter()
                .map(|p| rect.min + egui::vec2(p[0], p[1]))
                .collect();
            for point in &points {
                painter.circle_filled(*point, width * 0.5, color);
            }
            if points.len() > 1 {
                painter.add(egui::Shape::line(points, egui::Stroke::new(width, color)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Starting a recording must not create tools; only enabling drawing creates a native viewport.
    #[test]
    fn floating_tools_follow_shortcut_and_close_lifecycle() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        let id = ViewportId::from_hash_of("brush_tools");
        let area = Region {
            x: -1200,
            y: 20,
            width: 1000,
            height: 700,
        };
        let mut brush = Brush {
            area: Some(area),
            ..Default::default()
        };
        let run = |brush: &mut Brush| {
            ctx.run(egui::RawInput::default(), |ctx| {
                brush.show_tools(ctx, zsr::settings::Language::English, "Ctrl+Shift+B");
            })
        };
        // egui may retire an unused viewport entirely instead of emitting its queued Close.
        let closed = |output: &egui::FullOutput| {
            output
                .viewport_output
                .get(&id)
                .is_none_or(|tools| tools.commands.contains(&ViewportCommand::Close))
        };
        assert!(!run(&mut brush).viewport_output.contains_key(&id));
        brush.canvas.lock().unwrap().toggle(123);
        let output = run(&mut brush);
        let tools = &output.viewport_output[&id];
        assert!(tools.class == egui::ViewportClass::Deferred);
        assert_eq!(
            tools.builder.window_level,
            Some(egui::WindowLevel::AlwaysOnTop)
        );
        assert!(tools.viewport_ui_cb.is_some());
        assert!(
            output.shapes.is_empty(),
            "tools must not be painted in the root window"
        );
        {
            let mut canvas = brush.canvas.lock().unwrap();
            canvas.begin_stroke([0.1, 0.1], area);
            canvas.disable(); // Escape preserves ink while hiding the tools.
        }
        let output = run(&mut brush);
        assert!(closed(&output));
        assert!(!brush.tools_open);
        assert_eq!(brush.canvas.lock().unwrap().ink.strokes.len(), 1);
        brush.canvas.lock().unwrap().toggle(123);
        run(&mut brush);
        brush.canvas.lock().unwrap().exit(); // Window close / Exit button clears too.
        let output = run(&mut brush);
        assert!(closed(&output));
        assert!(brush.canvas.lock().unwrap().ink.strokes.is_empty());
        brush.canvas.lock().unwrap().toggle(123);
        run(&mut brush);
        *brush.canvas.lock().unwrap() = Canvas::default(); // Recording stop resets the canvas.
        let output = run(&mut brush);
        assert!(closed(&output));
    }
    /// Exercise actual panel clicks and wrapping without controlling the user's desktop.
    #[test]
    fn panel_selects_shapes_and_colors_without_modifying_existing_ink() {
        use zsr::settings::Language;
        let ctx = egui::Context::default();
        let mut canvas = Canvas::default();
        canvas.begin_stroke(
            [0.5, 0.5],
            Region {
                x: 0,
                y: 0,
                width: 800,
                height: 400,
            },
        );
        canvas.drawing = false;
        let run = |canvas: &mut Canvas, lang, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        control_panel(ui, canvas, lang, "Ctrl+Shift+B");
                        assert!(
                            ui.min_rect().right() <= 400.0,
                            "panel overflowed a narrow window"
                        );
                    });
                },
            )
        };
        run(&mut canvas, Language::English, vec![]);
        let output = run(&mut canvas, Language::English, vec![]);
        let rectangle = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Rectangle" => {
                    Some(t.pos + t.galley.size() * 0.5)
                }
                _ => None,
            })
            .expect("rectangle selector is visible");
        run(
            &mut canvas,
            Language::English,
            vec![pointer(rectangle.x, rectangle.y, true)],
        );
        let output = run(
            &mut canvas,
            Language::English,
            vec![pointer(rectangle.x, rectangle.y, false)],
        );
        assert_eq!(canvas.style.tool, Tool::Rectangle);
        let yellow = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == Color32::from_rgb(255, 210, 40) => {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("yellow preset is visible");
        run(
            &mut canvas,
            Language::English,
            vec![pointer(yellow.x, yellow.y, true)],
        );
        run(
            &mut canvas,
            Language::English,
            vec![pointer(yellow.x, yellow.y, false)],
        );
        assert_eq!(canvas.style.color, [255, 210, 40]);
        assert_eq!(canvas.ink.strokes[0].color, zsr::brush::COLOR);
        assert_eq!(canvas.ink.strokes[0].tool, Tool::Freehand);
        run(&mut canvas, Language::Chinese, vec![]);
    }
    /// Dragging a shape replaces its endpoint, and changing tools never restyles earlier ink.
    #[test]
    fn shapes_preview_endpoints_and_keep_their_selected_color() {
        let mut canvas = Canvas::default();
        let area = Region {
            x: 0,
            y: 0,
            width: 800,
            height: 400,
        };
        for (i, tool) in [Tool::Line, Tool::Arrow, Tool::Rectangle, Tool::Ellipse]
            .into_iter()
            .enumerate()
        {
            canvas.style = Style {
                tool,
                color: [i as u8 * 50, 210, 70],
            };
            canvas.begin_stroke([0.1, 0.2], area);
            for j in 0..100 {
                canvas.extend_stroke([0.3 + j as f32 / 200.0, 0.6], area);
            }
            canvas.extend_stroke([0.8, 0.7], area);
            let stroke = &canvas.ink.strokes[i];
            assert_eq!(stroke.points, [[0.1, 0.2], [0.8, 0.7]]);
            assert_eq!(stroke.tool, tool);
            assert_eq!(stroke.color, [i as u8 * 50, 210, 70]);
        }
        canvas.disable();
        assert_eq!(canvas.ink.strokes.len(), 4);
        assert_eq!(canvas.ink.strokes[0].color, [0, 210, 70]);
        assert_eq!(canvas.ink.strokes[0].tool, Tool::Line);
        assert!(canvas.ink.valid());
    }
    /// A second press must clear a stroke already published to the video, even mid-drag;
    /// enabling again must not bring any previous marks back.
    #[test]
    fn shortcut_exits_clears_and_reenters_with_an_empty_canvas() {
        let mut canvas = Canvas::default();
        let area = Region {
            x: 0,
            y: 0,
            width: 800,
            height: 400,
        };
        canvas.toggle(123);
        assert!(canvas.enabled && canvas.focus);
        canvas.begin_stroke([0.2, 0.3], area);
        canvas.extend_stroke([0.8, 0.7], area);
        assert!(canvas.drawing && !canvas.ink.strokes.is_empty());
        // Simulate both consumers having displayed the existing drawing.
        canvas.dirty = false;
        #[cfg(windows)]
        {
            canvas.repaint = false;
        }
        canvas.toggle(456);
        assert!(!canvas.enabled && !canvas.drawing && !canvas.focus);
        assert!(canvas.ink.strokes.is_empty() && canvas.dirty);
        assert_eq!(canvas.restore_focus, Some(123));
        #[cfg(windows)]
        assert!(canvas.repaint);
        canvas.take_restore();
        canvas.toggle(789);
        assert!(canvas.enabled && canvas.ink.strokes.is_empty());
        assert_eq!(canvas.previous_focus, 789);
    }
    fn frame(ctx: &egui::Context, canvas: &mut Canvas, events: Vec<egui::Event>) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                paint(
                    ctx,
                    canvas,
                    Region {
                        x: -1000,
                        y: 50,
                        width: 800,
                        height: 400,
                    },
                )
            },
        );
    }
    fn pointer(x: f32, y: f32, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(x, y),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    /// Exercise real egui input at 2x scale, edge clamping, editing and pointer-mode guards.
    #[test]
    fn drawing_uses_capture_coordinates_and_supports_undo_clear() {
        let ctx = egui::Context::default();
        let mut canvas = Canvas {
            enabled: true,
            ..Default::default()
        };
        frame(&ctx, &mut canvas, vec![pointer(100.0, 50.0, true)]);
        frame(
            &ctx,
            &mut canvas,
            vec![egui::Event::PointerMoved(egui::pos2(450.0, 250.0))],
        );
        frame(&ctx, &mut canvas, vec![pointer(450.0, 250.0, false)]);
        assert_eq!(canvas.ink.strokes[0].points, [[0.25, 0.25], [1.0, 1.0]]);
        assert_eq!(canvas.ink.strokes[0].width, 0.01);
        assert!(!canvas.drawing);
        frame(
            &ctx,
            &mut canvas,
            vec![pointer(200.0, 100.0, true), pointer(200.0, 100.0, false)],
        );
        assert_eq!(canvas.ink.strokes.len(), 2);
        frame(
            &ctx,
            &mut canvas,
            vec![key(egui::Key::Z, egui::Modifiers::CTRL)],
        );
        assert_eq!(canvas.ink.strokes.len(), 1);
        frame(
            &ctx,
            &mut canvas,
            vec![key(egui::Key::Delete, egui::Modifiers::NONE)],
        );
        assert!(canvas.ink.strokes.is_empty());
        canvas.enabled = false;
        canvas.dirty = false;
        frame(&ctx, &mut canvas, vec![pointer(100.0, 50.0, true)]);
        assert!(canvas.ink.strokes.is_empty() && !canvas.dirty);
    }
}
