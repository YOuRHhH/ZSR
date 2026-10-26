use super::Shortcuts;
use eframe::egui;
use global_hotkey::hotkey::HotKey;
use zsr::{i18n::tr, settings::Language};

impl Shortcuts {
    /// Stop listening when Settings closes; the next input poll restores OS bindings.
    pub(in crate::app) fn cancel_capture(&mut self) {
        self.capturing = None;
    }

    /// Arm a start/stop capture button, temporarily releasing OS shortcuts so key presses reach egui.
    pub(super) fn capture_button(&mut self, ui: &mut egui::Ui, start: bool, language: Language) {
        let label = if self.capturing == Some(start) {
            tr(language, "shortcut_listening")
        } else if start {
            &self.start
        } else {
            &self.stop
        };
        if ui.button(label).clicked() {
            if !self.suspended {
                if let Some(manager) = &self.manager
                    && let Err(error) = manager.unregister_all(&self.keys)
                {
                    self.error = Some(error.to_string());
                    return;
                }
                self.suspended = true;
            }
            self.capturing = Some(start);
            self.error = None;
        }
    }

    /// Capture one chord; Escape/focus loss cancels, and bindings resume only after release.
    pub(super) fn capture_input(&mut self, ctx: &egui::Context) -> bool {
        if !self.suspended {
            return false;
        }
        if let Some(start) = self.capturing {
            if !ctx.input(|i| i.focused) {
                self.capturing = None;
            } else {
                for event in ctx.input(|i| i.events.clone()) {
                    if let egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        repeat: false,
                        ..
                    } = event
                    {
                        if key == egui::Key::Escape {
                            self.capturing = None;
                            break;
                        }
                        match chord(key, modifiers) {
                            Ok(value) => {
                                if start {
                                    self.start = value;
                                } else {
                                    self.stop = value;
                                }
                                self.capturing = None;
                                self.error = None;
                            }
                            Err(error) => self.error = Some(error),
                        }
                        break;
                    }
                }
            }
        } else if ctx.input(|i| !i.focused || (i.keys_down.is_empty() && i.modifiers.is_none())) {
            if let Some(manager) = &self.manager
                && let Err(error) = manager.register_all(&self.keys)
            {
                let _ = manager.unregister_all(&self.keys);
                self.keys.clear();
                self.error = Some(error.to_string());
            }
            self.suspended = false;
        }
        // Suppress queued release events even on the frame registrations resume.
        true
    }
}

/// Convert egui modifiers and the pressed key into a validated global accelerator string.
fn chord(key: egui::Key, modifiers: egui::Modifiers) -> Result<String, String> {
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.mac_cmd || (modifiers.command && !modifiers.ctrl) {
        parts.push("Super");
    }
    parts.push(key.name());
    let value = parts.join("+");
    value.parse::<HotKey>().map_err(|e| e.to_string())?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Physical key capture produces valid accelerators without duplicating Ctrl as Command.
    #[test]
    fn captures_function_keys_letters_and_command() {
        assert_eq!(
            chord(
                egui::Key::F9,
                egui::Modifiers {
                    ctrl: true,
                    shift: true,
                    command: true,
                    ..Default::default()
                }
            )
            .unwrap(),
            "Ctrl+Shift+F9"
        );
        assert_eq!(
            chord(
                egui::Key::K,
                egui::Modifiers {
                    mac_cmd: true,
                    command: true,
                    ..Default::default()
                }
            )
            .unwrap(),
            "Super+K"
        );
        assert_eq!(chord(egui::Key::F10, egui::Modifiers::NONE).unwrap(), "F10");
    }
}
