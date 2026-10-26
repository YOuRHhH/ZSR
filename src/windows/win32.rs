use super::AppWindow;
#[repr(C)]
struct NativeWindow {
    id: u64,
    pid: u32,
    title: [u16; 512],
    app: [u16; 260],
}
pub(super) fn discover() -> Result<Vec<AppWindow>, String> {
    unsafe extern "C" {
        fn zsr_windows(out: *mut NativeWindow, capacity: i32) -> i32;
    }
    let mut buffer: Vec<NativeWindow> = (0..1024)
        .map(|_| NativeWindow {
            id: 0,
            pid: 0,
            title: [0; 512],
            app: [0; 260],
        })
        .collect();
    // SAFETY: repr(C) matches native/windows.c; the adapter respects capacity
    // and retains no pointers. UTF-16 buffers remain owned by Rust.
    let count = unsafe { zsr_windows(buffer.as_mut_ptr(), buffer.len() as i32) };
    if count < 0 {
        return Err("Windows could not enumerate application windows.".into());
    }
    let decode = |s: &[u16]| {
        String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())])
    };
    Ok(buffer
        .into_iter()
        .take(count as usize)
        .map(|w| {
            let path = decode(&w.app);
            AppWindow {
                id: w.id,
                pid: w.pid,
                title: decode(&w.title),
                app: path.rsplit('\\').next().unwrap_or_default().to_owned(),
            }
        })
        .collect())
}
