//! Turning a stream of percentage readings into attempts.
//!
//! A reading on its own says nothing about whether it is worth interrupting
//! someone for. 74% only matters if the player got there from the start rather
//! than dropping in from a Start Position. Watching a run grow continuously
//! from zero is what separates the two, and unlike the `TESTMODE` watermark it
//! cannot be hidden by a mod.

/// Settings for one channel.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Alert at or above this percentage.
    pub alert_at: f64,
    /// A run must begin at or below this to count as a real attempt.
    pub start_ceiling: f64,
    /// Larger forward jumps between samples are misreads, not progress.
    pub max_step: f64,
    /// A drop this large means the attempt ended and another began.
    pub reset_drop: f64,
    /// Consecutive readings above the threshold before alerting. The OCR
    /// misreads occasionally, and one bad glyph should not wake anyone up.
    pub confirm_samples: u32,
    /// Readings missed before the current attempt is closed.
    pub end_after_idle: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            alert_at: 70.0,
            start_ceiling: 3.0,
            max_step: 12.0,
            reset_drop: 5.0,
            confirm_samples: 2,
            end_after_idle: 9,
        }
    }
}

/// An attempt in progress.
#[derive(Debug, Clone, Copy)]
pub struct Run {
    pub started_at: f64,
    pub current: f64,
    pub peak: f64,
    pub samples: u32,
    /// Whether this attempt was watched from the very beginning.
    pub from_start: bool,
    alerted: bool,
    above: u32,
}

impl Run {
    /// Progress actually earned, which is all a Start Pos run really achieved.
    #[must_use]
    pub fn earned(&self) -> f64 {
        self.peak - self.started_at
    }
}

/// Something worth telling the rest of the app about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Started {
        at: f64,
        from_start: bool,
    },
    Progressed {
        value: f64,
    },
    /// A real attempt crossed the threshold. Fires once per attempt.
    Alert {
        value: f64,
    },
    Ended {
        peak: f64,
        from_start: bool,
    },
}

/// Follows one channel's attempts.
#[derive(Debug, Clone)]
pub struct Tracker {
    config: Config,
    run: Option<Run>,
    idle: u32,
    best: f64,
}

impl Tracker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            run: None,
            idle: 0,
            best: 0.0,
        }
    }

    #[must_use]
    pub fn run(&self) -> Option<&Run> {
        self.run.as_ref()
    }

    /// Highest percentage seen on a real attempt.
    #[must_use]
    pub fn best(&self) -> f64 {
        self.best
    }

    /// Feeds one sample. `None` means the frame had no readable percentage,
    /// which happens constantly during menus and death screens.
    pub fn observe(&mut self, reading: Option<f64>) -> Vec<Event> {
        let Some(value) = reading else {
            return self.observe_gap();
        };
        self.idle = 0;

        let Some(run) = self.run else {
            return vec![self.begin(value)];
        };

        let delta = value - run.current;

        if delta <= -self.config.reset_drop {
            let mut events = vec![self.finish()];
            events.push(self.begin(value));
            return events;
        }
        if delta > self.config.max_step {
            return Vec::new();
        }

        self.advance(value)
    }

    /// Handles a frame with no reading, closing the attempt once enough pass.
    fn observe_gap(&mut self) -> Vec<Event> {
        if self.run.is_none() {
            return Vec::new();
        }
        self.idle += 1;
        if self.idle < self.config.end_after_idle {
            return Vec::new();
        }
        vec![self.finish()]
    }

    fn begin(&mut self, value: f64) -> Event {
        let from_start = value <= self.config.start_ceiling;
        self.run = Some(Run {
            started_at: value,
            current: value,
            peak: value,
            samples: 1,
            from_start,
            alerted: false,
            above: 0,
        });
        self.idle = 0;
        Event::Started {
            at: value,
            from_start,
        }
    }

    fn advance(&mut self, value: f64) -> Vec<Event> {
        let config = self.config;
        let mut alert = false;
        let mut peak = 0.0;
        let mut from_start = false;

        if let Some(run) = self.run.as_mut() {
            run.current = value;
            run.samples += 1;
            run.peak = run.peak.max(value);

            run.above = if value >= config.alert_at {
                run.above + 1
            } else {
                0
            };

            if !run.alerted && run.from_start && run.above >= config.confirm_samples {
                run.alerted = true;
                alert = true;
            }
            peak = run.peak;
            from_start = run.from_start;
        }

        if from_start {
            self.best = self.best.max(peak);
        }

        let mut events = vec![Event::Progressed { value }];
        if alert {
            events.push(Event::Alert { value });
        }
        events
    }

    fn finish(&mut self) -> Event {
        let run = self.run.take().expect("finish called with a run");
        self.idle = 0;
        Event::Ended {
            peak: run.peak,
            from_start: run.from_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Event, Tracker};

    fn tracker() -> Tracker {
        Tracker::new(Config::default())
    }

    /// Feeds a sequence and collects every event produced.
    fn feed(tracker: &mut Tracker, values: &[f64]) -> Vec<Event> {
        values
            .iter()
            .flat_map(|&v| tracker.observe(Some(v)))
            .collect()
    }

    /// A plausible run: about 2% per sample at 3 fps.
    fn climb(from: f64, to: f64) -> Vec<f64> {
        let mut values = Vec::new();
        let mut v = from;
        while v < to {
            values.push(v);
            v += 2.0;
        }
        values.push(to);
        values
    }

    fn alerts(events: &[Event]) -> Vec<f64> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Alert { value } => Some(*value),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_run_from_zero_alerts_once_past_the_threshold() {
        let mut t = tracker();
        let events = feed(&mut t, &climb(0.5, 76.0));
        assert_eq!(alerts(&events).len(), 1, "{events:?}");
    }

    #[test]
    fn alerting_waits_for_confirmation() {
        let mut t = tracker();
        // Reaches the threshold on the last sample only: not yet confirmed.
        let events = feed(&mut t, &climb(0.5, 70.5));
        assert!(alerts(&events).is_empty(), "{events:?}");

        let more = t.observe(Some(72.0));
        assert_eq!(alerts(&more).len(), 1, "{more:?}");
    }

    #[test]
    fn a_start_pos_run_never_alerts() {
        let mut t = tracker();
        let events = feed(&mut t, &climb(49.0, 85.0));
        assert!(alerts(&events).is_empty(), "{events:?}");
        assert!(!t.run().expect("run").from_start);
    }

    #[test]
    fn earned_progress_discounts_the_start_offset() {
        let mut t = tracker();
        feed(&mut t, &climb(49.0, 62.0));
        assert!((t.run().expect("run").earned() - 13.0).abs() < 1e-9);
    }

    #[test]
    fn dying_and_retrying_starts_a_new_attempt() {
        let mut t = tracker();
        let mut values = climb(0.5, 60.0);
        values.extend(climb(0.5, 10.0));
        let events = feed(&mut t, &values);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Ended { peak, .. } if (*peak - 60.0).abs() < 1e-9)),
            "{events:?}"
        );
        let run = t.run().expect("new run");
        assert!(run.from_start);
        assert!((run.peak - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_second_attempt_can_alert_again() {
        let mut t = tracker();
        let mut values = climb(0.5, 74.0);
        values.extend(climb(0.5, 74.0));
        let events = feed(&mut t, &values);
        assert_eq!(alerts(&events).len(), 2, "{events:?}");
    }

    #[test]
    fn an_implausible_jump_is_ignored() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 10.0));
        let events = t.observe(Some(73.0));
        assert!(events.is_empty(), "{events:?}");
        assert!((t.run().expect("run").current - 10.0).abs() < 1e-9);
    }

    #[test]
    fn one_bad_reading_cannot_alert_alone() {
        let mut t = tracker();
        // Climb to just under the threshold, then a single spike within
        // max_step, then back down. A lone spike must not fire.
        let mut values = climb(0.5, 66.0);
        values.push(71.0);
        values.push(68.0);
        let events = feed(&mut t, &values);
        assert!(alerts(&events).is_empty(), "{events:?}");
    }

    #[test]
    fn a_gap_closes_the_attempt() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 30.0));
        let mut ended = None;
        for _ in 0..Config::default().end_after_idle {
            if let Some(event) = t.observe(None).first().copied() {
                ended = Some(event);
            }
        }
        assert!(matches!(ended, Some(Event::Ended { .. })), "{ended:?}");
        assert!(t.run().is_none());
    }

    #[test]
    fn a_short_gap_does_not_end_the_attempt() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 30.0));
        for _ in 0..(Config::default().end_after_idle - 1) {
            assert!(t.observe(None).is_empty());
        }
        assert!(t.run().is_some());
    }

    #[test]
    fn best_ignores_start_pos_runs() {
        let mut t = tracker();
        feed(&mut t, &climb(49.0, 90.0));
        assert!((t.best() - 0.0).abs() < 1e-9, "best was {}", t.best());

        for _ in 0..Config::default().end_after_idle {
            t.observe(None);
        }
        feed(&mut t, &climb(0.5, 40.0));
        assert!((t.best() - 40.0).abs() < 1e-9, "best was {}", t.best());
    }

    #[test]
    fn a_reading_with_no_run_starts_one() {
        let mut t = tracker();
        assert_eq!(
            t.observe(Some(12.0)),
            vec![Event::Started {
                at: 12.0,
                from_start: false
            }]
        );
    }
}
