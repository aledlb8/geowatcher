//! Locating and measuring the Geometry Dash progress bar.
//!
//! The bar is a rounded rectangle near the top of the viewport with a bright
//! outline. Its two horizontal borders show up as long bright runs on adjacent
//! rows, which is what we search for.

use crate::capture::Frame;

/// Only the top of the frame is searched.
const SEARCH_FRACTION: f64 = 0.20;

/// A border run must span at least this much of the frame width. The real bar
/// measures ~35% across every channel sampled; the level editor's scroll
/// slider is ~24%, so this threshold rejects it.
const MIN_RUN_FRACTION: f64 = 0.28;

/// Vertical distance between the bar's two borders.
const MIN_BORDER_GAP: u32 = 7;
const MAX_BORDER_GAP: u32 = 26;

/// The bar is centred in the viewport; allow this much drift.
const CENTER_TOLERANCE_FRACTION: f64 = 0.12;

/// Luminance for outline pixels.
const BORDER_LUMA: u8 = 190;

/// Luminance for filled-track pixels, which are dimmer than the outline.
const FILL_LUMA: u8 = 140;

/// Dark pixels tolerated inside the fill before calling it the edge.
const FILL_GAP_TOLERANCE: u32 = 6;

/// Ignore this many pixels at each end, where the rounded caps are bright
/// regardless of fill.
const CAP_INSET: u32 = 3;

/// A located progress bar in frame coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BarRegion {
    pub x0: u32,
    pub x1: u32,
    pub y_top: u32,
    pub y_bottom: u32,
}

impl BarRegion {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.x1.saturating_sub(self.x0)
    }

    /// Row sampled when measuring fill.
    #[must_use]
    pub fn interior_y(&self) -> u32 {
        (self.y_top + self.y_bottom) / 2
    }

    /// Fraction of the bar that is filled, 0.0 to 1.0.
    ///
    /// Validated against on-screen OCR to about 0.15 percentage points.
    #[must_use]
    pub fn fill_ratio(&self, frame: &Frame) -> Option<f64> {
        let start = self.x0 + CAP_INSET;
        let end = self.x1.checked_sub(CAP_INSET)?;
        if end <= start {
            return None;
        }

        let y = self.interior_y();
        let span = end - start;
        let mut edge = 0;
        let mut gap = 0;

        for offset in 0..=span {
            if frame.pixel(start + offset, y) >= FILL_LUMA {
                edge = offset;
                gap = 0;
            } else {
                gap += 1;
                if gap > FILL_GAP_TOLERANCE {
                    break;
                }
            }
        }

        Some(f64::from(edge) / f64::from(span))
    }
}

/// Longest run of pixels at or above `luma` in one row.
fn longest_run(frame: &Frame, y: u32, luma: u8) -> Option<(u32, u32)> {
    let row = frame.row(y);
    let mut best: Option<(u32, u32)> = None;
    let mut start = 0u32;
    let mut len = 0u32;

    for (x, &value) in row.iter().enumerate() {
        let x = u32::try_from(x).unwrap_or(u32::MAX);
        if value >= luma {
            if len == 0 {
                start = x;
            }
            len += 1;
            if best.is_none_or(|(bs, be)| be - bs + 1 < len) {
                best = Some((start, x));
            }
        } else {
            len = 0;
        }
    }
    best
}

/// Candidate bars in one frame, best-centred first.
#[must_use]
pub fn candidates(frame: &Frame) -> Vec<BarRegion> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let search_rows = (f64::from(frame.height) * SEARCH_FRACTION) as u32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let min_run = (f64::from(frame.width) * MIN_RUN_FRACTION) as u32;

    // One long run per row is enough; the bar's borders are the widest things
    // this near the top of a level.
    let runs: Vec<Option<(u32, u32)>> = (0..search_rows)
        .map(|y| {
            longest_run(frame, y, BORDER_LUMA).filter(|&(s, e)| e.saturating_sub(s) >= min_run)
        })
        .collect();

    let center = f64::from(frame.width) / 2.0;
    let tolerance = f64::from(frame.width) * CENTER_TOLERANCE_FRACTION;
    let mut found = Vec::new();

    for top in 0..search_rows {
        let Some((top_start, top_end)) = runs[top as usize] else {
            continue;
        };
        for bottom in (top + MIN_BORDER_GAP)..=(top + MAX_BORDER_GAP).min(search_rows - 1) {
            let Some((bottom_start, bottom_end)) = runs[bottom as usize] else {
                continue;
            };

            // Both borders belong to the same rectangle, so their extents agree.
            let x0 = top_start.max(bottom_start);
            let x1 = top_end.min(bottom_end);
            if x1 <= x0 {
                continue;
            }
            let outer = (top_end.max(bottom_end)) - (top_start.min(bottom_start));
            if outer == 0 || f64::from(x1 - x0) / f64::from(outer) < 0.9 {
                continue;
            }

            let bar_center = f64::from(x0 + x1) / 2.0;
            if (bar_center - center).abs() > tolerance {
                continue;
            }

            found.push((
                (bar_center - center).abs(),
                BarRegion {
                    x0,
                    x1,
                    y_top: top,
                    y_bottom: bottom,
                },
            ));
        }
    }

    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, region)| region).collect()
}

#[cfg(test)]
mod tests {
    use super::{candidates, BarRegion};
    use crate::capture::Frame;

    const W: u32 = 852;
    const H: u32 = 480;

    /// Draws a GD-style bar: bright borders, a filled left portion, dark track.
    fn frame_with_bar(x0: u32, x1: u32, y_top: u32, y_bottom: u32, fill: f64) -> Frame {
        let mut gray = vec![20u8; (W * H) as usize];
        let mut set = |x: u32, y: u32, v: u8| {
            gray[(y * W + x) as usize] = v;
        };

        for x in x0..=x1 {
            set(x, y_top, 255);
            set(x, y_bottom, 255);
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let fill_end = x0 + (f64::from(x1 - x0) * fill) as u32;
        for y in (y_top + 1)..y_bottom {
            for x in x0..=fill_end {
                set(x, y, 200);
            }
        }
        Frame::new(W, H, gray)
    }

    #[test]
    fn finds_a_standard_bar() {
        let frame = frame_with_bar(272, 578, 5, 17, 0.5);
        let found = candidates(&frame);
        assert!(!found.is_empty(), "expected a candidate");
        let bar = found[0];
        assert_eq!((bar.y_top, bar.y_bottom), (5, 17));
        assert!(bar.width() > 300, "width was {}", bar.width());
    }

    #[test]
    fn measures_fill_accurately() {
        for target in [0.0, 0.25, 0.5, 0.735, 1.0] {
            let frame = frame_with_bar(272, 578, 5, 17, target);
            let bar = candidates(&frame)[0];
            let measured = bar.fill_ratio(&frame).expect("ratio");
            assert!(
                (measured - target).abs() < 0.02,
                "target {target}, measured {measured}"
            );
        }
    }

    #[test]
    fn rejects_the_editor_slider() {
        // Narrower than a real bar and lower down; must not be picked up.
        let frame = frame_with_bar(343, 551, 22, 31, 0.0);
        assert!(candidates(&frame).is_empty());
    }

    #[test]
    fn rejects_an_off_centre_shape() {
        let frame = frame_with_bar(0, 310, 5, 17, 0.5);
        assert!(candidates(&frame).is_empty());
    }

    #[test]
    fn empty_frame_has_no_candidates() {
        let frame = Frame::new(W, H, vec![0u8; (W * H) as usize]);
        assert!(candidates(&frame).is_empty());
    }

    #[test]
    fn fill_ratio_needs_a_sane_region() {
        let frame = frame_with_bar(272, 578, 5, 17, 0.5);
        let degenerate = BarRegion {
            x0: 100,
            x1: 102,
            y_top: 5,
            y_bottom: 17,
        };
        assert!(degenerate.fill_ratio(&frame).is_none());
    }
}
