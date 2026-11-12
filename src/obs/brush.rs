//! Composite window annotations after the captured scene, before video encoding.
use super::{
    api::{Api, Ptr, Vec4},
    runtime::c,
};
use std::sync::{Arc, Mutex};
use zsr::brush::Ink;

pub type Updates = Arc<Mutex<Option<Ink>>>;

pub struct Overlay<'a> {
    api: &'a Api,
    state: *mut Render<'a>,
}
struct Render<'a> {
    api: &'a Api,
    source: Ptr,
    width: f32,
    height: f32,
    updates: Updates,
    ink: Ink,
    source_size: (u32, u32),
    vertices: Ptr,
    // Preserve stroke order at overlaps while changing color within one cached vertex buffer.
    batches: Vec<Batch>,
}
struct Batch {
    color: [u8; 3],
    start: u32,
    count: u32,
}
impl<'a> Overlay<'a> {
    /// The boxed state remains stable until OBS removes the callback under its render lock.
    pub fn new(api: &'a Api, source: Ptr, width: u32, height: u32, updates: Updates) -> Self {
        let state = Box::into_raw(Box::new(Render {
            api,
            source,
            width: width as f32,
            height: height as f32,
            updates,
            ink: Ink::default(),
            source_size: (0, 0),
            vertices: std::ptr::null_mut(),
            batches: Vec::new(),
        }));
        unsafe {
            (api.obs_add_main_rendered_callback)(render, state.cast());
        }
        Self { api, state }
    }
}
impl Drop for Overlay<'_> {
    fn drop(&mut self) {
        unsafe {
            // Do not borrow render state until removal has synchronized with the video thread.
            (self.api.obs_remove_main_rendered_callback)(render, self.state.cast());
            let state = Box::from_raw(self.state);
            (self.api.obs_enter_graphics)();
            (self.api.gs_vertexbuffer_destroy)(state.vertices);
            (self.api.obs_leave_graphics)();
        }
    }
}

/// Only OBS's video thread touches the mesh; stdin publishes bounded snapshots separately.
unsafe extern "C" fn render(data: Ptr) {
    let state = unsafe { &mut *data.cast::<Render<'_>>() };
    let api = state.api;
    let next = state.updates.lock().ok().and_then(|mut ink| ink.take());
    let size = unsafe {
        (
            (api.obs_source_get_width)(state.source),
            (api.obs_source_get_height)(state.source),
        )
    };
    let changed = next.is_some() || size != state.source_size;
    // Retain edits even while a minimized source temporarily has no drawable dimensions.
    if let Some(ink) = next {
        state.ink = ink;
    }
    if size.0 == 0 || size.1 == 0 {
        state.source_size = size;
        return;
    }
    if changed {
        state.source_size = size;
        // Match scene-item SCALE_INNER: preserve aspect ratio and center any letterboxing.
        let scale = (state.width / size.0 as f32).min(state.height / size.1 as f32);
        let width = size.0 as f32 * scale;
        let height = size.1 as f32 * scale;
        let mut mesh = Vec::new();
        state.batches.clear();
        for stroke in &state.ink.strokes {
            let vertices = stroke.triangles(width, height);
            if !vertices.is_empty() {
                state.batches.push(Batch {
                    color: stroke.color,
                    start: mesh.len() as u32,
                    count: vertices.len() as u32,
                });
                mesh.extend(vertices);
            }
        }
        for p in &mut mesh {
            p[0] += (state.width - width) * 0.5;
            p[1] += (state.height - height) * 0.5;
        }
        // Cache the GPU buffer between edits; OBS immediate rendering is limited to 512 vertices.
        unsafe {
            (api.gs_vertexbuffer_destroy)(state.vertices);
            state.vertices = std::ptr::null_mut();
            if !mesh.is_empty() {
                (api.gs_render_start)(true);
                for p in &mesh {
                    (api.gs_vertex2f)(p[0], p[1]);
                }
                state.vertices = (api.gs_render_save)();
            }
        }
    }
    if state.vertices.is_null() {
        return;
    }
    unsafe {
        let effect = (api.obs_get_base_effect)(3); // OBS_EFFECT_SOLID, libobs 32.
        (api.gs_matrix_push)();
        (api.gs_matrix_identity)();
        for batch in &state.batches {
            let color = Vec4([
                batch.color[0] as f32 / 255.0,
                batch.color[1] as f32 / 255.0,
                batch.color[2] as f32 / 255.0,
                1.0,
            ]);
            (api.gs_effect_set_vec4)(
                (api.gs_effect_get_param_by_name)(effect, c("color").as_ptr()),
                &color,
            );
            while (api.gs_effect_loop)(effect, c("Solid").as_ptr()) {
                (api.gs_load_vertexbuffer)(state.vertices);
                (api.gs_load_indexbuffer)(std::ptr::null_mut());
                (api.gs_draw)(3, batch.start, batch.count); // GS_TRIS, in original drawing order.
            }
        }
        (api.gs_matrix_pop)();
    }
}
