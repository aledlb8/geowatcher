//! Finds a channel's progress bar by watching where one keeps appearing.
//!
//! A single frame is not enough: level geometry throws up bright horizontal
//! shapes, and the level editor has its own scroll slider that looks a lot like
//! a progress bar. Recurrence across frames plus a preference for the topmost
//! candidate separates the real one.

use std::collections::HashMap;

use crate::capture::Frame;

use super::bar::{candidates, BarRegion};

/// Frames before a result is offered.
const MIN_FRAMES: u32 = 8;

/// Below this share of frames the match is too weak to trust.
const MIN_CONFIDENCE: f64 = 0.35;

/// Coordinate quantisation, absorbing a few pixels of jitter.
const X_BUCKET: u32 = 4;
const Y_BUCKET: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    pub bar: BarRegion,
    /// Share of observed frames this bar appeared in.
    pub confidence: f64,
}

impl Calibration {
    /// Whether this should be used without asking the user to confirm.
    #[must_use]
    pub fn is_confident(&self) -> bool {
        self.confidence >= 0.6
    }
}

#[derive(Debug, Default)]
pub struct Calibrator {
    frames: u32,
    seen: HashMap<(u32, u32, u32, u32), (u32, BarRegion)>,
}

impl Calibrator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn frames_seen(&self) -> u32 {
        self.frames
    }

    /// Records the best candidate in this frame, if any.
    pub fn observe(&mut self, frame: &Frame) {
        self.frames += 1;

        // Only the best-centred candidate counts, so a busy level cannot stuff
        // the tally with geometry that happens to be bright.
        if let Some(region) = candidates(frame).into_iter().next() {
            let key = (
                region.x0 / X_BUCKET,
                region.x1 / X_BUCKET,
                region.y_top / Y_BUCKET,
                region.y_bottom / Y_BUCKET,
            );
            let entry = self.seen.entry(key).or_insert((0, region));
            entry.0 += 1;
        }
    }

    /// Most consistent bar seen so far, preferring the topmost on a tie.
    #[must_use]
    pub fn best(&self) -> Option<Calibration> {
        if self.frames < MIN_FRAMES {
            return None;
        }

        let (count, bar) =
            self.seen
                .values()
                .copied()
                .max_by(|(a_count, a_bar), (b_count, b_bar)| {
                    a_count
                        .cmp(b_count)
                        .then_with(|| b_bar.y_top.cmp(&a_bar.y_top))
                })?;

        let confidence = f64::from(count) / f64::from(self.frames);
        if confidence < MIN_CONFIDENCE {
            return None;
        }
        Some(Calibration { bar, confidence })
    }
}

#[cfg(test)]
mod tests {
    use super::Calibrator;
    use crate::capture::Frame;

    const W: u32 = 852;
    const H: u32 = 480;

    fn bar_frame(x0: u32, x1: u32, y_top: u32, y_bottom: u32) -> Frame {
        let mut gray = vec![20u8; (W * H) as usize];
        for x in x0..=x1 {
            gray[(y_top * W + x) as usize] = 255;
            gray[(y_bottom * W + x) as usize] = 255;
        }
        Frame::new(W, H, gray)
    }

    fn blank() -> Frame {
        Frame::new(W, H, vec![0u8; (W * H) as usize])
    }

    #[test]
    fn withholds_a_result_until_enough_frames() {
        let mut c = Calibrator::new();
        for _ in 0..4 {
            c.observe(&bar_frame(272, 578, 5, 17));
        }
        assert!(c.best().is_none());
    }

    #[test]
    fn locks_on_to_a_recurring_bar() {
        let mut c = Calibrator::new();
        for _ in 0..12 {
            c.observe(&bar_frame(272, 578, 5, 17));
        }
        let result = c.best().expect("calibration");
        assert_eq!((result.bar.y_top, result.bar.y_bottom), (5, 17));
        assert!(result.is_confident(), "confidence {}", result.confidence);
    }

    #[test]
    fn stays_unconvinced_when_the_bar_is_rare() {
        let mut c = Calibrator::new();
        for _ in 0..2 {
            c.observe(&bar_frame(272, 578, 5, 17));
        }
        for _ in 0..14 {
            c.observe(&blank());
        }
        assert!(c.best().is_none());
    }

    #[test]
    fn tolerates_a_few_pixels_of_jitter() {
        let mut c = Calibrator::new();
        for i in 0..12 {
            let nudge = i % 3;
            c.observe(&bar_frame(272 + nudge, 578 + nudge, 5, 17));
        }
        let result = c.best().expect("calibration");
        assert!(result.confidence > 0.5, "confidence {}", result.confidence);
    }
}
