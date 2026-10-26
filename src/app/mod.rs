mod appearance;
mod controller;
mod file_opener;
mod preferences;
mod recordings;
mod region;
mod shortcuts;
#[cfg(feature = "ui-smoke")]
mod smoke;
mod ui;
use eframe::egui;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
use zsr::{
    audio::AudioDevices,
    platform::Capabilities,
    recording::{Region, Session},
    settings::Settings,
};

enum Background {
    Probe(Capabilities, String),
    Folder(PathBuf),
}
#[derive(PartialEq, Eq)]
enum State {
    Idle,
    Preparing,
    Recording,
    Stopping,
    Saved,
    Failed,
}
#[derive(PartialEq, Eq)]
enum CaptureMode {
    Display,
    Region,
    Window,
}

pub struct App {
    shortcuts: shortcuts::Shortcuts,
    settings: Settings,
    settings_open: bool,
    file_opener: file_opener::FileOpener,
    recordings: recordings::Recordings,
    capabilities: Option<Capabilities>,
    checking: bool,
    probed_ffmpeg: String,
    audio: AudioDevices,
    tx: Sender<Background>,
    rx: Receiver<Background>,
    display: usize,
    capture_mode: CaptureMode,
    windows: zsr::windows::WindowInventory,
    selected_window: Option<(u64, u32)>,
    region: Region,
    picker: Option<region::PickerWindow>,
    region_marker: region::RegionMarker,
    state: State,
    session: Option<Session>,
    frames: u64,
    seconds: f64,
    saved: Option<PathBuf>,
    detail: String,
    settings_error: Option<String>,
    font_missing: bool,
    dirty: Option<Instant>,
    last_settings: String,
    #[cfg(feature = "ui-smoke")]
    smoke: Option<(PathBuf, Instant, bool)>,
    #[cfg(feature = "ui-smoke")]
    performance: Option<smoke::Performance>,
}

impl App {
    /// Initialize preferences, global shortcuts, and asynchronous device discovery.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let font_missing = !appearance::configure(&cc.egui_ctx);
        let loaded = Settings::load();
        let settings_error = loaded.as_ref().err().cloned();
        let mut settings = loaded.unwrap_or_default();
        if let Some(ffmpeg) = zsr::bundled_backend::bundled_ffmpeg() {
            settings.ffmpeg = ffmpeg.to_string_lossy().into_owned();
        } else if !std::path::Path::new(&settings.ffmpeg).is_file()
            && std::path::Path::new(&settings.ffmpeg).is_absolute()
        {
            // A removed Full installation must not leave Lite pointing to a missing bundle.
            settings.ffmpeg = "ffmpeg".into();
        }
        #[cfg(feature = "ui-smoke")]
        let settings = {
            let mut settings = settings;
            // Isolate real-window capture regression runs from personal preferences/audio.
            if let Some(path) = std::env::var_os("ZSR_CAPTURE_TEST_DIR") {
                settings.destination = path.into();
                settings.microphone = false;
                settings.fps = 60;
                settings.resolution = zsr::resolution::Resolution::Original;
            }
            // Let UI snapshots inspect fixture recordings without changing saved preferences.
            if let Some(path) = std::env::var_os("ZSR_UI_RECORDINGS_DIR") {
                settings.destination = path.into();
            }
            if let Ok(lang) = std::env::var("ZSR_UI_LANGUAGE") {
                settings.language = if lang == "zh" {
                    zsr::settings::Language::Chinese
                } else {
                    zsr::settings::Language::English
                };
            }
            settings
        };
        let last_settings = serde_json::to_string(&settings).unwrap_or_default();
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            shortcuts: shortcuts::Shortcuts::new(
                settings.start_shortcut.clone(),
                settings.stop_shortcut.clone(),
            ),
            settings,
            // Allow the screenshot harness to inspect preferences without UI automation.
            settings_open: cfg!(feature = "ui-smoke")
                && std::env::var_os("ZSR_UI_SETTINGS").is_some(),
            recordings: Default::default(),
            file_opener: Default::default(),
            capabilities: None,
            checking: false,
            probed_ffmpeg: String::new(),
            audio: AudioDevices::default(),
            tx,
            rx,
            display: 0,
            capture_mode: CaptureMode::Display,
            windows: Default::default(),
            selected_window: None,
            region: Region {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            picker: None,
            region_marker: Default::default(),
            state: State::Idle,
            session: None,
            frames: 0,
            seconds: 0.0,
            saved: None,
            detail: String::new(),
            settings_error,
            font_missing,
            dirty: None,
            last_settings,
            #[cfg(feature = "ui-smoke")]
            smoke: std::env::var_os("ZSR_UI_SNAPSHOT")
                .map(|path| (PathBuf::from(path), Instant::now(), false)),
            #[cfg(feature = "ui-smoke")]
            performance: smoke::Performance::from_env(),
        };
        #[cfg(feature = "ui-smoke")]
        {
            app.recordings.open = std::env::var_os("ZSR_UI_RECORDINGS_DIR").is_some();
            if std::env::var_os("ZSR_CAPTURE_TEST_DIR").is_some() {
                app.capture_mode = CaptureMode::Region;
                if std::env::var_os("ZSR_CAPTURE_TEST_FULL").is_some() {
                    app.capture_mode = CaptureMode::Display;
                }
                app.region = Region {
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                };
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        800.0, 100.0,
                    )));
            }
        }
        app.probe();
        app
    }
}
impl eframe::App for App {
    /// Poll recording and shortcut events, then update the interface and overlays.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        #[cfg(feature = "ui-smoke")]
        if let Some(performance) = &mut self.performance {
            performance.sample(ctx, _frame);
        }
        self.file_opener.poll();
        self.recordings.poll(&self.settings.destination);
        self.poll_shortcuts(ctx);
        self.windows.poll();
        if self.capture_mode == CaptureMode::Window && !self.active() {
            self.windows.refresh_due(&self.settings.ffmpeg);
        }
        if !self.active() && self.picker.is_none() && self.settings.microphone {
            self.audio.refresh_if_due(&self.settings.ffmpeg);
        }
        #[cfg(feature = "ui-smoke")]
        self.snapshot(ctx);
        if self.active() && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.detail = zsr::i18n::tr(self.settings.language, "close_hint").into();
        }
        #[cfg(feature = "ui-smoke")]
        let preview = self.region_preview(ctx);
        #[cfg(not(feature = "ui-smoke"))]
        let preview = false;
        if !preview {
            if self.picker.is_none() {
                self.render(ctx);
            }
            self.region_picker(ctx);
            let target = if self.capture_mode == CaptureMode::Region && self.picker.is_none() {
                self.capabilities
                    .as_ref()
                    .and_then(|c| c.sources.get(self.display))
                    .map(|s| (s.display, self.region))
            } else {
                None
            };
            self.region_marker.update(ctx, target);
        }
        self.save_if_changed();
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.persist_preferences() {
            let _ = self.settings.save();
        }
    }
    /// Avoid transparent OpenGL composition for the main recording controls.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        #[cfg(feature = "ui-smoke")]
        if std::env::var_os("ZSR_UI_TRANSPARENT_BASELINE").is_some() {
            return [0.0; 4];
        }
        eframe::egui::Rgba::from(visuals.panel_fill).to_array()
    }
}
