use super::{App, State};
use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use zsr::i18n::tr;
mod capture;

pub(super) struct Shortcuts {
    manager: Option<GlobalHotKeyManager>,
    keys: Vec<HotKey>,
    start: String,
    stop: String,
    error: Option<String>,
    capturing: Option<bool>,
    suspended: bool,
}

impl Shortcuts {
    /// Create OS-wide bindings on the UI thread, retaining errors for the settings panel.
    pub(super) fn new(start: String, stop: String) -> Self {
        let manager = GlobalHotKeyManager::new();
        let error = manager.as_ref().err().map(ToString::to_string);
        let mut shortcuts = Self {
            manager: manager.ok(),
            keys: Vec::new(),
            start,
            stop,
            error,
            capturing: None,
            suspended: false,
        };
        if shortcuts.manager.is_some() {
            shortcuts.error = shortcuts.apply().err();
        }
        shortcuts
    }

    /// Replace both bindings together and restore the old pair if registration fails.
    fn apply(&mut self) -> Result<(), String> {
        let next = parse_pair(&self.start, &self.stop)?;
        let manager = self
            .manager
            .as_ref()
            .ok_or("Global shortcuts are unavailable on this desktop")?;
        manager
            .unregister_all(&self.keys)
            .map_err(|e| e.to_string())?;
        for (index, key) in next.iter().enumerate() {
            if let Err(error) = manager.register(*key) {
                let _ = manager.unregister_all(&next[..index]);
                if let Err(restore) = manager.register_all(&self.keys) {
                    self.keys.clear();
                    return Err(format!(
                        "{error}; could not restore previous shortcuts: {restore}"
                    ));
                }
                return Err(error.to_string());
            }
        }
        self.keys = next.to_vec();
        Ok(())
    }
}

/// Parse distinct nonempty accelerators before changing any system registrations.
fn parse_pair(start: &str, stop: &str) -> Result<[HotKey; 2], String> {
    let start: HotKey = start.trim().parse().map_err(|e| format!("{e}"))?;
    let stop: HotKey = stop.trim().parse().map_err(|e| format!("{e}"))?;
    if start.id() == stop.id() {
        return Err("Start and stop shortcuts must differ.".into());
    }
    Ok([start, stop])
}

impl Drop for Shortcuts {
    /// Release global bindings when the application exits.
    fn drop(&mut self) {
        if let Some(manager) = &self.manager {
            let _ = manager.unregister_all(&self.keys);
        }
    }
}

impl App {
    /// Dispatch released global keys through the normal recording lifecycle guards.
    pub(super) fn poll_shortcuts(&mut self, ctx: &egui::Context) {
        let suppress = self.shortcuts.capture_input(ctx);
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if suppress || event.state != HotKeyState::Released || self.shortcuts.keys.len() != 2 {
                continue;
            }
            if event.id == self.shortcuts.keys[1].id() {
                if let Some(session) = &self.session {
                    session.stop();
                    self.state = State::Stopping;
                }
            } else if event.id == self.shortcuts.keys[0].id()
                && !self.active()
                && !self.checking
                && self.picker.is_none()
                && self.probed_ffmpeg == self.settings.ffmpeg
                && self
                    .capabilities
                    .as_ref()
                    .is_some_and(|c| c.error.is_none() && !c.wayland && !c.sources.is_empty())
            {
                self.start();
            }
        }
    }

    /// Capture key combinations and persist only successfully applied shortcut pairs.
    pub(in crate::app) fn shortcut_controls(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.separator();
        ui.label(tr(lang, "shortcuts"));
        ui.label(tr(lang, "shortcut_start"));
        self.shortcuts.capture_button(ui, true, lang);
        ui.label(tr(lang, "shortcut_stop"));
        self.shortcuts.capture_button(ui, false, lang);
        ui.small(tr(lang, "shortcut_hint"));
        if ui
            .add_enabled(
                !self.shortcuts.suspended,
                egui::Button::new(tr(lang, "shortcut_apply")),
            )
            .clicked()
        {
            match self.shortcuts.apply() {
                Ok(()) => {
                    self.settings.start_shortcut = self.shortcuts.start.clone();
                    self.settings.stop_shortcut = self.shortcuts.stop.clone();
                    self.shortcuts.error = None;
                }
                Err(error) => self.shortcuts.error = Some(error),
            }
        }
        if let Some(error) = &self.shortcuts.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }
}

#[cfg(test)]
mod tests {
    /// Invalid and duplicate bindings cannot replace a working pair.
    #[test]
    fn validates_pair_before_registration() {
        assert!(super::parse_pair("Ctrl+Shift+F9", "Ctrl+Shift+F10").is_ok());
        assert!(super::parse_pair("Ctrl+F9", "Ctrl+F9").is_err());
        assert!(super::parse_pair("", "garbage").is_err());
    }
}
