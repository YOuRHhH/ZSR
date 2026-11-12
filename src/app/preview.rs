//! A memory-only view of the selected capture, separate from recording status.
use super::{App, CaptureMode};
use eframe::egui::{self, Color32};
use std::time::{Duration, Instant};
use zsr::{
    i18n::tr,
    recording::{Capture, Request},
    settings::Settings,
};

#[derive(Default)]
pub(super) struct Preview {
    key: String,
    changed: Option<Instant>,
    session: Option<zsr::preview::Session>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
}

impl App {
    /// Capture only after opting in; disabling also releases the worker and cached pixels.
    pub(super) fn update_preview(&mut self, ctx: &egui::Context) {
        if !self.settings.preview_enabled
            || self.active()
            || self.picker.is_some()
            || self.checking
            || ctx.input(|i| i.viewport().minimized == Some(true))
        {
            self.preview = Preview::default();
            return;
        }
        let capture = if self.capture_mode == CaptureMode::Window {
            self.windows
                .selected(self.selected_window, &self.settings.obs_runtime)
                .cloned()
                .map(Capture::Window)
        } else {
            self.capabilities
                .as_ref()
                .and_then(|c| c.sources.get(self.display))
                .cloned()
                .map(|source| Capture::Screen {
                    source,
                    region: (self.capture_mode == CaptureMode::Region).then_some(self.region),
                })
        };
        let Some(capture) = capture else {
            self.preview = Preview::default();
            return;
        };
        // Audio and encoding preferences must not prevent a video-only preview or restart it.
        let settings = Settings {
            obs_runtime: self.settings.obs_runtime.clone(),
            cursor: self.settings.cursor,
            resolution: self.settings.resolution,
            ..Settings::default()
        };
        let request = Request { settings, capture };
        let key = serde_json::to_string(&(
            &request.capture,
            &request.settings.obs_runtime,
            request.settings.cursor,
            request.settings.resolution,
        ))
        .expect("serializable preview selection");
        if self.preview.key != key {
            self.preview = Preview {
                key,
                changed: Some(Instant::now()),
                ..Default::default()
            };
        }
        // Debounce coordinate edits so dragging a region does not launch a helper per frame.
        if self.preview.session.is_none()
            && self.preview.error.is_none()
            && self
                .preview
                .changed
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(250))
        {
            match zsr::preview::Session::start(request) {
                Ok(session) => self.preview.session = Some(session),
                Err(error) => self.preview.error = Some(error),
            }
        }
        if let Some(result) = self.preview.session.as_ref().and_then(|s| s.take_frame()) {
            match result {
                Ok(frame) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [frame.width as usize, frame.height as usize],
                        &frame.rgba,
                    );
                    if let Some(texture) = &mut self.preview.texture {
                        texture.set(image, egui::TextureOptions::LINEAR);
                    } else {
                        self.preview.texture = Some(ctx.load_texture(
                            "capture-preview",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
                Err(error) => {
                    self.preview.error = Some(error);
                    self.preview.texture = None;
                    self.preview.session = None;
                }
            }
        }
    }

    /// Use a stable preview surface with aspect-fit pixels and explicit loading/error states.
    pub(super) fn preview_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.heading(tr(lang, "preview"));
        let status = if self.active() {
            "preview_paused"
        } else if self.preview.error.is_some() {
            "preview_failed"
        } else if self.checking {
            "preview_loading"
        } else if self.preview.key.is_empty() {
            "preview_select"
        } else if self.preview.texture.is_none() {
            "preview_loading"
        } else {
            "preview_live"
        };
        ui.small(tr(lang, status));
        // A fixed-height canvas avoids layout jumps between portrait and landscape sources.
        let size = egui::vec2(
            ui.available_width(),
            (ui.available_width() * 9.0 / 16.0).min(220.0),
        );
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect_filled(rect, 6.0, Color32::BLACK);
        if !self.active()
            && let Some(texture) = &self.preview.texture
        {
            let source = texture.size_vec2();
            let scale = (size.x / source.x).min(size.y / source.y);
            ui.painter().image(
                texture.id(),
                egui::Rect::from_center_size(rect.center(), source * scale),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr(lang, status),
                egui::FontId::proportional(14.0),
                Color32::LIGHT_GRAY,
            );
        }
        if let Some(error) = &self.preview.error {
            ui.label(tr(lang, "preview_failed")).on_hover_text(error);
            if ui.button(tr(lang, "preview_retry")).clicked() {
                self.preview = Preview::default();
            }
        }
        ui.small(tr(lang, "preview_hint"));
    }
}
