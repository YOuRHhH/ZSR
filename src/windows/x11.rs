use super::AppWindow;
#[cfg(target_os = "linux")]
pub(super) fn discover() -> Result<Vec<AppWindow>, String> {
    let text = crate::platform::output(
        crate::platform::command("wmctrl").args(["-lp"]),
        std::time::Duration::from_secs(5),
    )
    .map_err(|e| format!("Install wmctrl for X11 window discovery: {e}"))?;
    if text.contains("Cannot") || text.contains("cannot") {
        return Err(text);
    }
    Ok(parse(&text)
        .into_iter()
        .filter(|w| w.pid != std::process::id())
        .collect())
}
fn parse(text: &str) -> Vec<AppWindow> {
    text.lines()
        .filter_map(|line| {
            let mut remaining = line.trim();
            let mut fields = Vec::new();
            for _ in 0..4 {
                let end = remaining.find(char::is_whitespace)?;
                fields.push(&remaining[..end]);
                remaining = remaining[end..].trim_start();
            }
            let id = u64::from_str_radix(fields[0].strip_prefix("0x")?, 16).ok()?;
            let pid = fields[2].parse().ok()?;
            if id == 0 || remaining.is_empty() {
                return None;
            }
            Some(AppWindow {
                id,
                pid,
                title: remaining.into(),
                app: "X11".into(),
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    #[test]
    fn preserves_unicode_titles_and_distinct_window_ids() {
        let windows = super::parse(
            "0x0123  0  42 host 编辑器 — file one\n0x0124 0 42 host 编辑器 — file one\ninvalid",
        );
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].title, "编辑器 — file one");
        assert_ne!(windows[0].identity(), windows[1].identity());
    }
}
