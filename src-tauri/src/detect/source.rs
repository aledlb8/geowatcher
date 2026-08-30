//! Choosing between the percentage text and the progress bar.
//!
//! Text is far more accurate, so it wins whenever it is legible. Some channels
//! never give it: the percentage is switched off, or a stream overlay sits on
//! top of it. Those fall back to the bar, which is approximate but present.
//!
//! The choice is made once per stream and then held, because mixing the two
//! mid-run would look like a jump — the bar reads a couple of points off where
//! the text reads exact.

/// Frames observed before committing to a source.
const WINDOW: u32 = 60;

/// Text is considered usable if it reads at least this often.
const TEXT_HIT_RATE: f64 = 0.05;

/// The bar must be this reliable before it is worth falling back to.
const BAR_HIT_RATE: f64 = 0.5;

/// Where a percentage came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Still deciding; only text readings are passed through.
    Undecided,
    Text,
    /// Approximate, within a couple of percentage points.
    Bar,
}

/// Picks a source from what the first frames of a stream actually offered.
#[derive(Debug, Clone, Default)]
pub struct SourcePicker {
    frames: u32,
    text_hits: u32,
    bar_hits: u32,
    decided: Option<Source>,
}

impl SourcePicker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn source(&self) -> Source {
        self.decided.unwrap_or(Source::Undecided)
    }

    /// Records a frame and returns the percentage to track, if any.
    pub fn pick(&mut self, text: Option<f64>, bar: Option<f64>) -> Option<f64> {
        if self.decided.is_none() {
            self.frames += 1;
            if text.is_some() {
                self.text_hits += 1;
            }
            if bar.is_some() {
                self.bar_hits += 1;
            }
            if self.frames >= WINDOW {
                self.decided = Some(self.decide());
            }
            // Until the choice is made, only trust text.
            return text;
        }

        match self.decided {
            Some(Source::Bar) => bar,
            _ => text,
        }
    }

    fn decide(&self) -> Source {
        let frames = f64::from(self.frames.max(1));
        let text_rate = f64::from(self.text_hits) / frames;
        let bar_rate = f64::from(self.bar_hits) / frames;

        if text_rate >= TEXT_HIT_RATE || bar_rate < BAR_HIT_RATE {
            Source::Text
        } else {
            Source::Bar
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Source, SourcePicker, WINDOW};

    #[test]
    fn text_is_used_while_undecided() {
        let mut picker = SourcePicker::new();
        assert_eq!(picker.pick(Some(12.0), Some(9.0)), Some(12.0));
        assert_eq!(picker.source(), Source::Undecided);
    }

    #[test]
    fn a_channel_with_readable_text_stays_on_text() {
        let mut picker = SourcePicker::new();
        for i in 0..WINDOW {
            // Text reads about half the time, which is normal.
            let text = (i % 2 == 0).then_some(40.0);
            picker.pick(text, Some(38.0));
        }
        assert_eq!(picker.source(), Source::Text);
        assert_eq!(picker.pick(None, Some(38.0)), None);
    }

    #[test]
    fn a_channel_whose_text_never_reads_falls_back_to_the_bar() {
        let mut picker = SourcePicker::new();
        for _ in 0..WINDOW {
            picker.pick(None, Some(38.0));
        }
        assert_eq!(picker.source(), Source::Bar);
        assert_eq!(picker.pick(None, Some(41.0)), Some(41.0));
    }

    #[test]
    fn no_bar_and_no_text_stays_on_text_rather_than_inventing_numbers() {
        let mut picker = SourcePicker::new();
        for _ in 0..WINDOW {
            picker.pick(None, None);
        }
        assert_eq!(picker.source(), Source::Text);
    }

    #[test]
    fn even_rare_text_beats_the_bar() {
        let mut picker = SourcePicker::new();
        for i in 0..WINDOW {
            // Text reads on 1 frame in 12: still better than an approximate bar.
            let text = (i % 12 == 0).then_some(40.0);
            picker.pick(text, Some(38.0));
        }
        assert_eq!(picker.source(), Source::Text);
    }

    #[test]
    fn the_choice_is_held_once_made() {
        let mut picker = SourcePicker::new();
        for _ in 0..WINDOW {
            picker.pick(None, Some(38.0));
        }
        assert_eq!(picker.source(), Source::Bar);
        // Text appearing later must not flip the source mid-stream.
        picker.pick(Some(50.0), Some(48.0));
        assert_eq!(picker.source(), Source::Bar);
    }
}
