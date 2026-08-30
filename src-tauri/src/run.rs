//! Turning a stream of readings into attempts.
//!
//! 74% only matters if the player got there from the start rather than dropping
//! in from a Start Position. Continuity is what separates the two, and unlike
//! the `TESTMODE` watermark a mod cannot hide it.
//!
//! OCR cannot read the opening seconds of a run reliably, so continuity alone
//! would misjudge real attempts as Start Pos. An empty progress bar covers that
//! gap: its absolute value is untrustworthy, but "near empty" is a dependable
//! sign that an attempt just began.

/// Settings for one channel.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Alert at or above this percentage.
    pub alert_at: f64,
    /// A run starting at or below this counts as a real attempt.
    pub start_ceiling: f64,
    /// Larger forward jumps between samples are misreads, not progress.
    pub max_step: f64,
    /// A drop this large means the attempt ended and another began.
    pub reset_drop: f64,
    /// Consecutive readings above the threshold before alerting. The OCR
    /// misreads occasionally and one bad glyph should not raise an alert.
    pub confirm_samples: u32,
    /// Bar fill at or below this means an attempt is at its very beginning.
    pub empty_bar: f64,
    /// Consecutive empty-bar samples before believing it. The bar glitches.
    pub reset_confirm: u32,
    /// Samples after a reset during which a new run still counts as from-start.
    pub reset_grace: u32,
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
            empty_bar: 2.0,
            reset_confirm: 2,
            reset_grace: 15,
            end_after_idle: 9,
        }
    }
}

/// One frame's worth of measurements.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    /// Percentage read from the text, when legible.
    pub percent: Option<f64>,
    /// Bar fill as a percentage, when a bar was found.
    pub bar: Option<f64>,
}

impl Sample {
    #[must_use]
    pub fn new(percent: Option<f64>, bar: Option<f64>) -> Self {
        Self { percent, bar }
    }
}

/// An attempt in progress.
#[derive(Debug, Clone, Copy)]
pub struct Run {
    pub started_at: f64,
    pub current: f64,
    pub peak: f64,
    pub samples: u32,
    /// Whether this attempt was watched from the beginning.
    pub from_start: bool,
    alerted: bool,
    above: u32,
}

impl Run {
    /// Progress actually earned, which is all a Start Pos run achieved.
    #[must_use]
    pub fn earned(&self) -> f64 {
        self.peak - self.started_at
    }
}

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
    /// Consecutive empty-bar samples seen.
    empty_streak: u32,
    /// Samples since a confirmed reset, if one has happened.
    since_reset: Option<u32>,
}

impl Tracker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            run: None,
            idle: 0,
            best: 0.0,
            empty_streak: 0,
            since_reset: None,
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

    /// Feeds one frame's measurements.
    pub fn observe(&mut self, sample: Sample) -> Vec<Event> {
        let mut events = self.track_reset(sample.bar);

        let Some(value) = sample.percent else {
            events.extend(self.observe_gap());
            return events;
        };
        self.idle = 0;

        let Some(run) = self.run else {
            events.push(self.begin(value));
            return events;
        };

        let delta = value - run.current;
        if delta <= -self.config.reset_drop {
            events.push(self.finish());
            events.push(self.begin(value));
            return events;
        }
        if delta > self.config.max_step {
            return events;
        }

        events.extend(self.advance(value));
        events
    }

    /// Watches the bar for the start of a fresh attempt.
    fn track_reset(&mut self, bar: Option<f64>) -> Vec<Event> {
        if let Some(count) = self.since_reset {
            self.since_reset = Some(count.saturating_add(1));
        }

        let Some(bar) = bar else {
            self.empty_streak = 0;
            return Vec::new();
        };

        if bar > self.config.empty_bar {
            self.empty_streak = 0;
            return Vec::new();
        }

        self.empty_streak += 1;
        if self.empty_streak < self.config.reset_confirm {
            return Vec::new();
        }

        // Confirmed: the bar is empty, so any attempt in flight is over.
        self.since_reset = Some(0);
        let ended = self
            .run
            .is_some_and(|run| run.peak > self.config.start_ceiling);

        if ended {
            vec![self.finish()]
        } else {
            Vec::new()
        }
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
        // Either the reading itself is near zero, or the bar was seen empty
        // recently enough that this must be the same attempt continuing.
        let recent_reset = self
            .since_reset
            .is_some_and(|count| count <= self.config.reset_grace);
        let from_start = value <= self.config.start_ceiling || recent_reset;

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
    use super::{Config, Event, Sample, Tracker};

    fn tracker() -> Tracker {
        Tracker::new(Config::default())
    }

    /// Feeds percentages with no bar information.
    fn feed(tracker: &mut Tracker, values: &[f64]) -> Vec<Event> {
        values
            .iter()
            .flat_map(|&v| tracker.observe(Sample::new(Some(v), None)))
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

    /// Feeds an empty bar with no readable text, as at the start of a run.
    fn empty_bar(tracker: &mut Tracker, times: u32) -> Vec<Event> {
        (0..times)
            .flat_map(|_| tracker.observe(Sample::new(None, Some(0.4))))
            .collect()
    }

    #[test]
    fn a_run_from_zero_alerts_once_past_the_threshold() {
        let mut t = tracker();
        assert_eq!(alerts(&feed(&mut t, &climb(0.5, 76.0))).len(), 1);
    }

    #[test]
    fn alerting_waits_for_confirmation() {
        let mut t = tracker();
        assert!(alerts(&feed(&mut t, &climb(0.5, 70.5))).is_empty());
        assert_eq!(alerts(&t.observe(Sample::new(Some(72.0), None))).len(), 1);
    }

    #[test]
    fn a_start_pos_run_never_alerts() {
        let mut t = tracker();
        let events = feed(&mut t, &climb(49.0, 85.0));
        assert!(alerts(&events).is_empty(), "{events:?}");
        assert!(!t.run().expect("run").from_start);
    }

    #[test]
    fn an_empty_bar_rescues_a_run_ocr_joined_late() {
        // The real failure: the bar empties, OCR cannot read the first 20%,
        // and the run is only picked up mid-flight. It is still a real attempt.
        let mut t = tracker();
        empty_bar(&mut t, 3);
        let events = feed(&mut t, &climb(21.0, 76.0));

        assert!(t.run().expect("run").from_start);
        assert_eq!(alerts(&events).len(), 1, "{events:?}");
    }

    #[test]
    fn a_stale_reset_does_not_rescue_a_start_pos_run() {
        let mut t = tracker();
        empty_bar(&mut t, 3);
        // Long enough after the reset that this cannot be the same attempt.
        for _ in 0..(Config::default().reset_grace + 2) {
            t.observe(Sample::default());
        }
        feed(&mut t, &climb(49.0, 60.0));
        assert!(!t.run().expect("run").from_start);
    }

    #[test]
    fn one_bar_glitch_is_not_a_reset() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 40.0));
        // A single empty reading mid-run is a glitch, not the end of the run.
        let events = t.observe(Sample::new(None, Some(0.1)));
        assert!(events.is_empty(), "{events:?}");
        assert!(t.run().is_some());
    }

    #[test]
    fn a_confirmed_empty_bar_ends_the_attempt() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 40.0));
        let events = empty_bar(&mut t, 2);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Ended { peak, .. } if (*peak - 40.0).abs() < 1e-9)),
            "{events:?}"
        );
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
        assert!((t.run().expect("run").peak - 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_second_attempt_can_alert_again() {
        let mut t = tracker();
        let mut values = climb(0.5, 74.0);
        values.extend(climb(0.5, 74.0));
        assert_eq!(alerts(&feed(&mut t, &values)).len(), 2);
    }

    #[test]
    fn an_implausible_jump_is_ignored() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 10.0));
        assert!(t.observe(Sample::new(Some(73.0), None)).is_empty());
        assert!((t.run().expect("run").current - 10.0).abs() < 1e-9);
    }

    #[test]
    fn one_bad_reading_cannot_alert_alone() {
        let mut t = tracker();
        let mut values = climb(0.5, 66.0);
        values.push(71.0);
        values.push(68.0);
        assert!(alerts(&feed(&mut t, &values)).is_empty());
    }

    #[test]
    fn a_gap_closes_the_attempt() {
        let mut t = tracker();
        feed(&mut t, &climb(0.5, 30.0));
        let mut ended = None;
        for _ in 0..Config::default().end_after_idle {
            if let Some(event) = t.observe(Sample::default()).first().copied() {
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
            assert!(t.observe(Sample::default()).is_empty());
        }
        assert!(t.run().is_some());
    }

    #[test]
    fn best_ignores_start_pos_runs() {
        let mut t = tracker();
        feed(&mut t, &climb(49.0, 90.0));
        assert!((t.best() - 0.0).abs() < 1e-9, "best was {}", t.best());

        for _ in 0..Config::default().end_after_idle {
            t.observe(Sample::default());
        }
        feed(&mut t, &climb(0.5, 40.0));
        assert!((t.best() - 40.0).abs() < 1e-9, "best was {}", t.best());
    }
}
