use super::App;
use std::time::{Duration, Instant};
impl App {
    /// Keep screenshot and capture regression settings out of the user's configuration.
    pub(super) fn persist_preferences(&self) -> bool {
        #[cfg(feature = "ui-smoke")]
        if self.smoke.is_some()
            || self.performance.is_some()
            || std::env::var_os("ZSR_CAPTURE_TEST_DIR").is_some()
        {
            return false;
        }
        true
    }
    pub(super) fn save_if_changed(&mut self) {
        let serialized = serde_json::to_string(&self.settings).unwrap_or_default();
        if serialized != self.last_settings {
            self.last_settings = serialized;
            self.dirty = Some(Instant::now());
        }
        if self
            .dirty
            .is_some_and(|t| t.elapsed() > Duration::from_millis(700))
        {
            if self.persist_preferences() {
                self.settings_error = self.settings.save().err();
            }
            self.dirty = None;
        }
    }
}
