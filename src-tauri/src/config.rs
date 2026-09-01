//! Watched channels and their settings, persisted as JSON.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::appearance::Appearance;
use crate::detect::BarRegion;

/// A channel to watch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Twitch login, e.g. `zoinkgd`. Not the display name.
    pub login: String,
    /// Alert threshold, falling back to [`Settings::alert_at`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_at: Option<f64>,
    /// Pause without removing the channel.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Bar region from a previous session, or one the user set by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<SavedBar>,
}

impl Channel {
    #[must_use]
    pub fn new(login: impl Into<String>) -> Self {
        Self {
            login: login.into(),
            alert_at: None,
            enabled: true,
            calibration: None,
        }
    }
}

/// A calibrated bar, stored between sessions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SavedBar {
    pub x0: u32,
    pub x1: u32,
    pub y_top: u32,
    pub y_bottom: u32,
    /// Set when the user positioned it, so auto-calibration leaves it alone.
    #[serde(default)]
    pub manual: bool,
}

impl From<BarRegion> for SavedBar {
    fn from(bar: BarRegion) -> Self {
        Self {
            x0: bar.x0,
            x1: bar.x1,
            y_top: bar.y_top,
            y_bottom: bar.y_bottom,
            manual: false,
        }
    }
}

impl From<SavedBar> for BarRegion {
    fn from(saved: SavedBar) -> Self {
        Self {
            x0: saved.x0,
            x1: saved.x1,
            y_top: saved.y_top,
            y_bottom: saved.y_bottom,
        }
    }
}

/// Everything the watcher persists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub channels: Vec<Channel>,
    /// Default alert threshold.
    #[serde(default = "default_alert")]
    pub alert_at: f64,
    /// Ignore runs that began from a Start Position.
    #[serde(default = "yes")]
    pub ignore_start_pos: bool,
    /// Open the stream when a notification is clicked.
    #[serde(default = "yes")]
    pub open_on_click: bool,
    /// OCR analysis floor: the slowest rate the percentage is read at, per
    /// channel. Reads speed up on their own while a bar is moving.
    #[serde(default = "default_fps")]
    pub fps: f32,
    /// Frames pulled from the decoder per second, per channel. Every one gets a
    /// cheap bar check; the pricey OCR runs on a subset. Higher cuts latency.
    #[serde(default = "default_sample_fps")]
    pub sample_fps: f32,
    /// Path to mpv; empty means look on `PATH`.
    #[serde(default)]
    pub mpv_path: String,
    /// Path to ffmpeg; empty means the bundled sidecar, then `PATH`.
    #[serde(default)]
    pub ffmpeg_path: String,
    #[serde(default)]
    pub appearance: Appearance,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
            alert_at: default_alert(),
            ignore_start_pos: true,
            open_on_click: true,
            fps: default_fps(),
            sample_fps: default_sample_fps(),
            mpv_path: String::new(),
            ffmpeg_path: String::new(),
            appearance: Appearance::default(),
        }
    }
}

impl Settings {
    /// Loads settings, falling back to defaults when the file is missing.
    ///
    /// A corrupt file is an error rather than a silent reset, so a bad parse
    /// cannot quietly discard someone's channel list.
    pub fn load(path: &Path) -> Result<Self, Error> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(Error::Parse),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(Error::Io(err)),
        }
    }

    /// Location of the user stylesheet, beside the settings file.
    #[must_use]
    pub fn user_css_path(settings_path: &Path) -> PathBuf {
        settings_path.with_file_name("widget.css")
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(Error::Parse)?;
        std::fs::write(path, text).map_err(Error::Io)
    }

    /// Threshold for a channel, preferring its own over the default.
    #[must_use]
    pub fn threshold_for(&self, channel: &Channel) -> f64 {
        channel.alert_at.unwrap_or(self.alert_at)
    }

    /// Clamped, coherent `(sample_fps, analyze_fps)`: the analysis floor never
    /// exceeds the delivery rate, and both stay in a sane range.
    #[must_use]
    pub fn sampling(&self) -> (f32, f32) {
        let analyze = self.fps.clamp(0.5, 30.0);
        let sample = self.sample_fps.clamp(analyze, 30.0);
        (sample, analyze)
    }

    #[must_use]
    pub fn channel(&self, login: &str) -> Option<&Channel> {
        self.channels.iter().find(|c| c.login == login)
    }

    pub fn channel_mut(&mut self, login: &str) -> Option<&mut Channel> {
        self.channels.iter_mut().find(|c| c.login == login)
    }

    /// Adds a channel, or returns false if it is already watched.
    pub fn add_channel(&mut self, login: &str) -> bool {
        let login = login.trim().trim_start_matches('@').to_lowercase();
        if login.is_empty() || self.channel(&login).is_some() {
            return false;
        }
        self.channels.push(Channel::new(login));
        true
    }

    pub fn remove_channel(&mut self, login: &str) -> bool {
        let before = self.channels.len();
        self.channels.retain(|c| c.login != login);
        self.channels.len() != before
    }
}

/// Default location under the OS config directory.
#[must_use]
pub fn default_path(app_config_dir: &Path) -> PathBuf {
    app_config_dir.join("settings.json")
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read or write settings: {0}")]
    Io(std::io::Error),

    #[error("settings file is not valid JSON: {0}")]
    Parse(serde_json::Error),
}

fn yes() -> bool {
    true
}

fn default_alert() -> f64 {
    70.0
}

fn default_fps() -> f32 {
    3.0
}

fn default_sample_fps() -> f32 {
    8.0
}

#[cfg(test)]
mod tests {
    use super::{Channel, Settings};

    #[test]
    fn adding_normalises_the_login() {
        let mut s = Settings::default();
        assert!(s.add_channel("  @ZoinkGD "));
        assert_eq!(s.channels[0].login, "zoinkgd");
    }

    #[test]
    fn the_same_channel_is_not_added_twice() {
        let mut s = Settings::default();
        assert!(s.add_channel("zoinkgd"));
        assert!(!s.add_channel("ZOINKGD"));
        assert_eq!(s.channels.len(), 1);
    }

    #[test]
    fn an_empty_login_is_rejected() {
        let mut s = Settings::default();
        assert!(!s.add_channel("   "));
        assert!(s.channels.is_empty());
    }

    #[test]
    fn removing_reports_whether_it_existed() {
        let mut s = Settings::default();
        s.add_channel("zoinkgd");
        assert!(s.remove_channel("zoinkgd"));
        assert!(!s.remove_channel("zoinkgd"));
    }

    #[test]
    fn a_channel_threshold_overrides_the_default() {
        let mut s = Settings::default();
        s.add_channel("zoinkgd");
        assert!((s.threshold_for(&s.channels[0]) - 70.0).abs() < f64::EPSILON);

        s.channel_mut("zoinkgd").expect("channel").alert_at = Some(85.0);
        assert!((s.threshold_for(&s.channels[0]) - 85.0).abs() < f64::EPSILON);
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let dir = std::env::temp_dir().join("geowatcher-config-test");
        let path = dir.join("settings.json");
        let _ = std::fs::remove_file(&path);

        let mut original = Settings::default();
        original.add_channel("zoinkgd");
        original.alert_at = 65.0;
        original.save(&path).expect("save");

        let loaded = Settings::load(&path).expect("load");
        assert_eq!(loaded.channels.len(), 1);
        assert_eq!(loaded.channels[0].login, "zoinkgd");
        assert!((loaded.alert_at - 65.0).abs() < f64::EPSILON);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_file_gives_defaults() {
        let path = std::env::temp_dir().join("geowatcher-does-not-exist.json");
        let _ = std::fs::remove_file(&path);
        let loaded = Settings::load(&path).expect("defaults");
        assert!(loaded.channels.is_empty());
    }

    #[test]
    fn a_corrupt_file_is_an_error_not_a_silent_reset() {
        let path = std::env::temp_dir().join("geowatcher-corrupt.json");
        std::fs::write(&path, "{ not json").expect("write");
        assert!(Settings::load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_fields_do_not_break_loading() {
        // Older or newer builds may write fields this one does not know.
        let path = std::env::temp_dir().join("geowatcher-extra.json");
        std::fs::write(&path, r#"{"channels":[],"someFutureField":true}"#).expect("write");
        let loaded = Settings::load(&path).expect("load");
        assert!(loaded.channels.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_channel_defaults_to_enabled() {
        assert!(Channel::new("x").enabled);
    }
}
