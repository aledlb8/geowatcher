//! Runs one watcher task per channel and reports what they see.
//!
//! Each task owns its own reconnect loop. A channel being offline is the normal
//! case, not an error: the task waits and tries again, so adding a channel that
//! streams once a week costs one request every few minutes.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc::UnboundedSender;

use crate::capture::{Capture, CaptureOptions};
use crate::config::{Channel, Settings};
use crate::detect::{
    builtin, read_percentage, BarRegion, Calibration, Calibrator, SourcePicker, TemplateSet,
};
use crate::run::{self, Sample, Tracker};
use crate::twitch::{Error as TwitchError, TwitchClient};

/// How long to wait before checking an offline channel again.
const OFFLINE_POLL: Duration = Duration::from_secs(90);

/// Wait after a stream drops before reconnecting.
const RECONNECT_DELAY: Duration = Duration::from_secs(10);

/// Wait after an unexpected failure, longer than a normal reconnect.
const ERROR_BACKOFF: Duration = Duration::from_secs(60);

/// Bar fill must shift at least this many percent between frames to count as
/// activity worth a fresh OCR pass. Below it the last reading is reused.
const BAR_ACTIVITY_EPS: f64 = 0.3;

/// Identical progress readings are not re-emitted; a change must clear this.
const PROGRESS_EPS: f64 = 0.001;

/// What a channel is currently doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Status {
    Offline,
    Connecting,
    /// Watching, but the bar has not been located yet.
    Calibrating,
    Watching,
    Error {
        message: String,
    },
}

/// Something the UI or notifier should know about.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Update {
    Status {
        login: String,
        status: Status,
    },
    Progress {
        login: String,
        value: f64,
        from_start: bool,
    },
    Alert {
        login: String,
        value: f64,
        /// Direct HLS URL, so the viewer can skip the Twitch player.
        stream_url: Option<String>,
    },
    RunEnded {
        login: String,
        peak: f64,
        from_start: bool,
    },
}

/// Owns the running watcher tasks.
#[derive(Debug)]
pub struct Supervisor {
    tasks: HashMap<String, JoinHandle<()>>,
    updates: UnboundedSender<Update>,
    templates: TemplateSet,
}

impl Supervisor {
    #[must_use]
    pub fn new(updates: UnboundedSender<Update>) -> Self {
        Self {
            tasks: HashMap::new(),
            updates,
            templates: builtin(),
        }
    }

    #[must_use]
    pub fn is_watching(&self, login: &str) -> bool {
        self.tasks.contains_key(login)
    }

    #[must_use]
    pub fn watched(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tasks.keys().cloned().collect();
        names.sort();
        names
    }

    /// Starts watching a channel, replacing any existing task for it.
    pub fn watch(&mut self, channel: &Channel, settings: &Settings) {
        self.stop(&channel.login);

        let (sample_fps, analyze_fps) = settings.sampling();
        let watcher = Watcher {
            login: channel.login.clone(),
            config: run::Config {
                alert_at: settings.threshold_for(channel),
                ..run::Config::default()
            },
            saved_bar: channel.calibration.map(Into::into),
            sample_fps,
            analyze_fps,
            updates: self.updates.clone(),
            templates: self.templates.clone(),
        };

        let login = channel.login.clone();
        // Tauri's runtime, not `tokio::spawn`: synchronous commands run on a
        // blocking pool with no tokio context, where `tokio::spawn` panics.
        self.tasks
            .insert(login, tauri::async_runtime::spawn(watcher.run()));
    }

    pub fn stop(&mut self, login: &str) {
        if let Some(task) = self.tasks.remove(login) {
            task.abort();
        }
    }

    pub fn stop_all(&mut self) {
        for (_, task) in self.tasks.drain() {
            task.abort();
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop_all();
    }
}

/// One channel's reconnect loop.
struct Watcher {
    login: String,
    config: run::Config,
    saved_bar: Option<BarRegion>,
    /// Frames pulled from ffmpeg per second, and the OCR floor rate.
    sample_fps: f32,
    analyze_fps: f32,
    updates: UnboundedSender<Update>,
    templates: TemplateSet,
}

impl Watcher {
    async fn run(self) {
        let Ok(client) = TwitchClient::new() else {
            self.report(Status::Error {
                message: "could not create an HTTP client".to_owned(),
            });
            return;
        };

        loop {
            let delay = self.attempt(&client).await;
            tokio::time::sleep(delay).await;
        }
    }

    /// One connect-and-watch cycle, returning how long to wait before retrying.
    async fn attempt(&self, client: &TwitchClient) -> Duration {
        self.report(Status::Connecting);

        let playlist = match client.master_playlist(&self.login, true).await {
            Ok(playlist) => playlist,
            Err(TwitchError::Offline(_)) => {
                self.report(Status::Offline);
                return OFFLINE_POLL;
            }
            Err(err) => {
                self.report(Status::Error {
                    message: err.to_string(),
                });
                return ERROR_BACKOFF;
            }
        };

        let Some(variant) = playlist.best_for_analysis() else {
            self.report(Status::Error {
                message: "no usable quality".to_owned(),
            });
            return ERROR_BACKOFF;
        };

        let options = CaptureOptions::new(variant.url.clone()).fps(self.sample_fps);
        let capture = match Capture::start(&options) {
            Ok(capture) => capture,
            Err(err) => {
                self.report(Status::Error {
                    message: err.to_string(),
                });
                return ERROR_BACKOFF;
            }
        };

        self.consume(capture, &variant.url).await;
        RECONNECT_DELAY
    }

    /// Reads frames until the stream ends.
    async fn consume(&self, mut capture: Capture, stream_url: &str) {
        let mut calibrator = Calibrator::new();
        let mut bar = self.saved_bar;
        // Fed at the delivery rate, so its sample-count timings hold in wall time.
        let mut tracker = Tracker::new(self.config, self.sample_fps);
        let mut picker = SourcePicker::new();

        // Run OCR on activity, and at least this often when the bar sits still.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ocr_interval =
            ((f64::from(self.sample_fps) / f64::from(self.analyze_fps)).round() as u64).max(1);
        let mut frame_idx: u64 = 0;
        let mut last_bar: Option<f64> = None;
        let mut held_text: Option<f64> = None;
        let mut last_progress: Option<f64> = None;

        self.report(if bar.is_some() {
            Status::Watching
        } else {
            Status::Calibrating
        });

        while let Some(frame) = capture.next_frame().await {
            // Keep looking for the bar until one is found.
            if bar.is_none() {
                calibrator.observe(&frame);
                if let Some(Calibration { bar: found, .. }) = calibrator.best() {
                    bar = Some(found);
                    self.report(Status::Watching);
                }
                continue;
            }
            let Some(region) = bar else { continue };

            let bar_value = region.fill_ratio(&frame).map(|ratio| ratio * 100.0);

            // OCR is the expensive step; skip it while the bar is idle and reuse
            // the last reading, so a fast delivery rate costs little when nothing
            // is happening but reacts at once the moment a run moves.
            let moved = match (last_bar, bar_value) {
                (Some(a), Some(b)) => (a - b).abs() >= BAR_ACTIVITY_EPS,
                (None, None) => false,
                _ => true,
            };
            last_bar = bar_value;

            let text = if moved || frame_idx % ocr_interval == 0 {
                held_text = read_percentage(&frame, &region, &self.templates).map(|r| r.value);
                held_text
            } else {
                held_text
            };
            frame_idx += 1;

            // The bar is still reported separately: even when it is not the
            // source, an empty bar is what marks the start of an attempt.
            let sample = Sample::new(picker.pick(text, bar_value), bar_value);

            for event in tracker.observe(sample) {
                // Skip a progress event that repeats the last value; it would
                // only cost an IPC round trip and a widget re-render.
                match event {
                    run::Event::Progressed { value }
                        if last_progress.is_some_and(|p| (p - value).abs() < PROGRESS_EPS) =>
                    {
                        continue;
                    }
                    run::Event::Progressed { value } => last_progress = Some(value),
                    run::Event::Started { .. } | run::Event::Ended { .. } => last_progress = None,
                    run::Event::Alert { .. } => {}
                }
                self.emit(event, stream_url);
            }
        }

        capture.shutdown().await;
        self.report(Status::Offline);
    }

    fn emit(&self, event: run::Event, stream_url: &str) {
        let update = match event {
            run::Event::Progressed { value } => Update::Progress {
                login: self.login.clone(),
                value,
                from_start: true,
            },
            run::Event::Alert { value } => Update::Alert {
                login: self.login.clone(),
                value,
                stream_url: Some(stream_url.to_owned()),
            },
            run::Event::Ended { peak, from_start } => Update::RunEnded {
                login: self.login.clone(),
                peak,
                from_start,
            },
            run::Event::Started { .. } => return,
        };
        let _ = self.updates.send(update);
    }

    fn report(&self, status: Status) {
        let _ = self.updates.send(Update::Status {
            login: self.login.clone(),
            status,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Status, Supervisor, Update};
    use crate::config::{Channel, Settings};

    #[tokio::test]
    async fn watching_and_stopping_tracks_channels() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut supervisor = Supervisor::new(tx);
        let settings = Settings::default();

        supervisor.watch(&Channel::new("zoinkgd"), &settings);
        assert!(supervisor.is_watching("zoinkgd"));
        assert_eq!(supervisor.watched(), vec!["zoinkgd".to_owned()]);

        supervisor.stop("zoinkgd");
        assert!(!supervisor.is_watching("zoinkgd"));
    }

    #[tokio::test]
    async fn watching_twice_does_not_duplicate_the_task() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut supervisor = Supervisor::new(tx);
        let settings = Settings::default();

        supervisor.watch(&Channel::new("zoinkgd"), &settings);
        supervisor.watch(&Channel::new("zoinkgd"), &settings);
        assert_eq!(supervisor.watched().len(), 1);
    }

    #[tokio::test]
    async fn stop_all_clears_everything() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut supervisor = Supervisor::new(tx);
        let settings = Settings::default();

        supervisor.watch(&Channel::new("a"), &settings);
        supervisor.watch(&Channel::new("b"), &settings);
        supervisor.stop_all();
        assert!(supervisor.watched().is_empty());
    }

    #[test]
    fn status_serialises_with_a_tag_the_ui_can_switch_on() {
        let json = serde_json::to_string(&Status::Offline).expect("json");
        assert_eq!(json, r#"{"kind":"offline"}"#);

        let json = serde_json::to_string(&Status::Error {
            message: "boom".to_owned(),
        })
        .expect("json");
        assert!(json.contains(r#""kind":"error""#), "{json}");
    }

    #[test]
    fn updates_serialise_with_a_type_tag() {
        let json = serde_json::to_string(&Update::Alert {
            login: "zoinkgd".to_owned(),
            value: 74.5,
            stream_url: None,
        })
        .expect("json");
        assert!(json.contains(r#""type":"alert""#), "{json}");
        assert!(json.contains("74.5"), "{json}");
    }
}
