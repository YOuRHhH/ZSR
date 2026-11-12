//! Native capture bounds and focus restoration for a transparent drawing viewport.
use crate::recording::{Capture, Region};

/// Resolve the captured client area in physical desktop pixels; unavailable windows hide ink.
pub fn bounds(capture: &Capture) -> Option<Region> {
    match capture {
        Capture::Screen { source, region } => {
            let d = source.display;
            let r = region.unwrap_or(Region {
                x: 0,
                y: 0,
                width: d.width,
                height: d.height,
            });
            Some(Region {
                x: d.x + r.x,
                y: d.y + r.y,
                ..r
            })
        }
        Capture::Window(window) => window_bounds(window.id, window.pid),
        Capture::TestPattern => None,
    }
}

#[cfg(windows)]
mod native {
    use super::Region;
    use std::ffi::c_void;
    type Hwnd = *mut c_void;
    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetClientRect(window: Hwnd, rect: *mut Rect) -> i32;
        fn ClientToScreen(window: Hwnd, point: *mut Point) -> i32;
        fn GetWindowThreadProcessId(window: Hwnd, pid: *mut u32) -> u32;
        fn IsWindowVisible(window: Hwnd) -> i32;
        fn IsIconic(window: Hwnd) -> i32;
        fn GetForegroundWindow() -> Hwnd;
        fn SetForegroundWindow(window: Hwnd) -> i32;
        fn FindWindowW(class: *const u16, title: *const u16) -> Hwnd;
        fn SetWindowPos(
            window: Hwnd,
            after: Hwnd,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            flags: u32,
        ) -> i32;
    }
    pub fn window_bounds(id: u64, expected_pid: u32) -> Option<Region> {
        let window = id as usize as Hwnd;
        let mut rect = Rect::default();
        let mut point = Point::default();
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut pid);
            if pid != expected_pid
                || IsWindowVisible(window) == 0
                || IsIconic(window) != 0
                || GetClientRect(window, &mut rect) == 0
                || ClientToScreen(window, &mut point) == 0
            {
                return None;
            }
        }
        (rect.right > 0 && rect.bottom > 0).then_some(Region {
            x: point.x,
            y: point.y,
            width: rect.right,
            height: rect.bottom,
        })
    }
    pub fn focused() -> u64 {
        unsafe { GetForegroundWindow() as usize as u64 }
    }
    pub fn restore(id: u64) {
        if id != 0 {
            unsafe {
                SetForegroundWindow(id as usize as Hwnd);
            }
        }
    }
    /// Set physical bounds after winit creates the viewport, avoiding mixed-DPI drift.
    pub fn place(title: &str, rect: Region) {
        let title: Vec<_> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let window = FindWindowW(std::ptr::null(), title.as_ptr());
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            if pid == std::process::id() {
                SetWindowPos(
                    window,
                    -1isize as Hwnd,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    0x10,
                );
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod native {
    use super::Region;
    use x11rb::{
        connection::Connection,
        protocol::xproto::{AtomEnum, ConnectionExt, InputFocus, MapState},
        wrapper::ConnectionExt as _,
    };
    pub fn window_bounds(id: u64, expected_pid: u32) -> Option<Region> {
        // A separate protocol connection returns errors for destroyed windows instead of
        // installing a process-wide Xlib error handler that could interfere with winit.
        let (conn, screen) = x11rb::connect(None).ok()?;
        let id = u32::try_from(id).ok()?;
        // Match the process identity too, so a recycled XID cannot redirect drawing to another app.
        if expected_pid != 0 {
            let atom = conn
                .intern_atom(true, b"_NET_WM_PID")
                .ok()?
                .reply()
                .ok()?
                .atom;
            let property = conn
                .get_property(false, id, atom, AtomEnum::CARDINAL, 0, 1)
                .ok()?
                .reply()
                .ok()?;
            if property.value32()?.next()? != expected_pid {
                return None;
            }
        }
        let attr = conn.get_window_attributes(id).ok()?.reply().ok()?;
        if attr.map_state != MapState::VIEWABLE {
            return None;
        }
        let g = conn.get_geometry(id).ok()?.reply().ok()?;
        let root = conn.setup().roots[screen].root;
        let pos = conn
            .translate_coordinates(id, root, 0, 0)
            .ok()?
            .reply()
            .ok()?;
        Some(Region {
            x: pos.dst_x.into(),
            y: pos.dst_y.into(),
            width: g.width.into(),
            height: g.height.into(),
        })
    }
    pub fn focused() -> u64 {
        let Ok((conn, _)) = x11rb::connect(None) else {
            return 0;
        };
        conn.get_input_focus()
            .ok()
            .and_then(|r| r.reply().ok())
            .map_or(0, |r| r.focus.into())
    }
    pub fn restore(id: u64) {
        if id <= 1 {
            return;
        }
        if let Ok((conn, _)) = x11rb::connect(None) {
            if let Ok(cookie) =
                conn.set_input_focus(InputFocus::PARENT, id as u32, x11rb::CURRENT_TIME)
            {
                let _ = cookie.check();
            }
            let _ = conn.flush();
        }
    }
    pub fn place(_title: &str, _rect: Region) {}

    /// Tell the window manager the tools belong above the drawing canvas, without stealing focus.
    /// Retry until both viewports have been mapped; never change another process's windows.
    pub fn tools_above(canvas_title: &str, tools_title: &str) -> bool {
        fn attach(canvas_title: &str, tools_title: &str) -> Option<()> {
            let (conn, screen) = x11rb::connect(None).ok()?;
            let atom = |name: &[u8]| {
                conn.intern_atom(true, name)
                    .ok()?
                    .reply()
                    .ok()
                    .map(|r| r.atom)
            };
            let clients = atom(b"_NET_CLIENT_LIST")?;
            let name = atom(b"_NET_WM_NAME")?;
            let pid = atom(b"_NET_WM_PID")?;
            let list = conn
                .get_property(
                    false,
                    conn.setup().roots[screen].root,
                    clients,
                    AtomEnum::WINDOW,
                    0,
                    4096,
                )
                .ok()?
                .reply()
                .ok()?;
            let mut canvas = None;
            let mut tools = None;
            for id in list.value32()? {
                let title = conn
                    .get_property(false, id, name, AtomEnum::ANY, 0, 256)
                    .ok()?
                    .reply()
                    .ok()?;
                if title.value != canvas_title.as_bytes() && title.value != tools_title.as_bytes() {
                    continue;
                }
                let owner = conn
                    .get_property(false, id, pid, AtomEnum::CARDINAL, 0, 1)
                    .ok()?
                    .reply()
                    .ok()?;
                if owner.value32()?.next()? != std::process::id() {
                    continue;
                }
                if title.value == canvas_title.as_bytes() {
                    canvas = Some(id);
                } else {
                    tools = Some(id);
                }
            }
            let (canvas, tools) = (canvas?, tools?);
            let current = conn
                .get_property(
                    false,
                    tools,
                    AtomEnum::WM_TRANSIENT_FOR,
                    AtomEnum::WINDOW,
                    0,
                    1,
                )
                .ok()?
                .reply()
                .ok()?;
            if current.value32().and_then(|mut ids| ids.next()) != Some(canvas) {
                conn.change_property32(
                    x11rb::protocol::xproto::PropMode::REPLACE,
                    tools,
                    AtomEnum::WM_TRANSIENT_FOR,
                    AtomEnum::WINDOW,
                    &[canvas],
                )
                .ok()?
                .check()
                .ok()?;
                conn.flush().ok()?;
            }
            Some(())
        }
        attach(canvas_title, tools_title).is_some()
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod native {
    use super::Region;
    pub fn window_bounds(_id: u64, _pid: u32) -> Option<Region> {
        None
    }
    pub fn focused() -> u64 {
        0
    }
    pub fn restore(_id: u64) {}
    pub fn place(_title: &str, _rect: Region) {}
}
#[cfg(target_os = "linux")]
pub use native::tools_above;
use native::window_bounds;
pub use native::{focused, place, restore};
