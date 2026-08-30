//! Finding and splitting the percentage text.
//!
//! Geometry Dash draws the percentage immediately right of the progress bar, so
//! a calibrated bar tells us where to look. The glyphs are bright with a dark
//! outline, and always separated by at least a column or two of background,
//! which makes a column projection enough to split them.

use crate::capture::Frame;

use super::bar::BarRegion;

/// Glyph pixels are near-white.
const TEXT_LUMA: u8 = 200;

/// Blank columns needed to end a glyph.
const MIN_GAP: u32 = 2;

/// Clear of the bar's rounded cap.
const GAP_AFTER_BAR: u32 = 8;

/// Enough for `100.00%`.
const TEXT_WIDTH: u32 = 170;

/// The text sits a little below the bar's lower border.
const BELOW_BAR: u32 = 20;

/// A merged blob wider than this is not a single glyph.
const MAX_GLYPH_WIDTH: u32 = 34;

/// Where the percentage should be, derived from the bar.
#[derive(Debug, Clone, Copy)]
pub struct TextRegion {
    pub x0: u32,
    pub x1: u32,
    pub y0: u32,
    pub y1: u32,
}

impl TextRegion {
    #[must_use]
    pub fn right_of_bar(bar: &BarRegion, frame: &Frame) -> Self {
        Self {
            x0: (bar.x1 + GAP_AFTER_BAR).min(frame.width.saturating_sub(1)),
            x1: (bar.x1 + GAP_AFTER_BAR + TEXT_WIDTH).min(frame.width.saturating_sub(1)),
            y0: 0,
            y1: (bar.y_bottom + BELOW_BAR).min(frame.height.saturating_sub(1)),
        }
    }
}

/// One glyph's bounding box in frame coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphBox {
    pub x0: u32,
    pub x1: u32,
    pub y0: u32,
    pub y1: u32,
}

impl GlyphBox {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.x1 - self.x0 + 1
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.y1 - self.y0 + 1
    }
}

/// Splits the region into glyph boxes, left to right.
#[must_use]
pub fn segment(frame: &Frame, region: &TextRegion) -> Vec<GlyphBox> {
    let occupied: Vec<bool> = (region.x0..=region.x1)
        .map(|x| (region.y0..=region.y1).any(|y| frame.pixel(x, y) >= TEXT_LUMA))
        .collect();

    let mut boxes = Vec::new();
    let mut start: Option<u32> = None;
    let mut gap = 0;

    for (index, &ink) in occupied.iter().enumerate() {
        let x = region.x0 + u32::try_from(index).unwrap_or(0);
        if ink {
            if start.is_none() {
                start = Some(x);
            }
            gap = 0;
        } else if let Some(begin) = start {
            gap += 1;
            if gap >= MIN_GAP {
                push_glyph(&mut boxes, frame, region, begin, x - gap);
                start = None;
            }
        }
    }
    if let Some(begin) = start {
        push_glyph(&mut boxes, frame, region, begin, region.x1);
    }

    boxes
}

/// Trims a column span to its vertical ink extent and keeps it if plausible.
fn push_glyph(out: &mut Vec<GlyphBox>, frame: &Frame, region: &TextRegion, x0: u32, x1: u32) {
    if x1 < x0 || x1 - x0 + 1 > MAX_GLYPH_WIDTH {
        return;
    }

    let rows: Vec<u32> = (region.y0..=region.y1)
        .filter(|&y| (x0..=x1).any(|x| frame.pixel(x, y) >= TEXT_LUMA))
        .collect();

    let (Some(&y0), Some(&y1)) = (rows.first(), rows.last()) else {
        return;
    };

    out.push(GlyphBox { x0, x1, y0, y1 });
}

#[cfg(test)]
mod tests {
    use super::{segment, GlyphBox, TextRegion};
    use crate::capture::Frame;
    use crate::detect::BarRegion;

    const W: u32 = 852;
    const H: u32 = 480;

    /// Paints solid blocks at the given column spans to stand in for glyphs.
    fn frame_with_glyphs(spans: &[(u32, u32)]) -> Frame {
        let mut gray = vec![10u8; (W * H) as usize];
        for &(x0, x1) in spans {
            for x in x0..=x1 {
                for y in 8..=28 {
                    gray[(y * W + x) as usize] = 255;
                }
            }
        }
        Frame::new(W, H, gray)
    }

    fn bar() -> BarRegion {
        BarRegion {
            x0: 272,
            x1: 578,
            y_top: 5,
            y_bottom: 17,
        }
    }

    #[test]
    fn region_sits_just_right_of_the_bar() {
        let frame = frame_with_glyphs(&[]);
        let region = TextRegion::right_of_bar(&bar(), &frame);
        assert_eq!(region.x0, 586);
        assert!(region.x1 > 700);
    }

    #[test]
    fn splits_glyphs_on_column_gaps() {
        // The real spans measured from a 54.52% frame.
        let spans = [
            (593, 604),
            (607, 622),
            (625, 627),
            (631, 642),
            (646, 658),
            (661, 678),
        ];
        let frame = frame_with_glyphs(&spans);
        let found = segment(&frame, &TextRegion::right_of_bar(&bar(), &frame));

        assert_eq!(found.len(), 6, "got {found:?}");
        for (got, &(x0, x1)) in found.iter().zip(&spans) {
            assert_eq!((got.x0, got.x1), (x0, x1));
        }
    }

    #[test]
    fn reports_glyph_extents() {
        let frame = frame_with_glyphs(&[(600, 611)]);
        let found = segment(&frame, &TextRegion::right_of_bar(&bar(), &frame));
        assert_eq!(
            found,
            vec![GlyphBox {
                x0: 600,
                x1: 611,
                y0: 8,
                y1: 28
            }]
        );
    }

    #[test]
    fn ignores_an_overwide_blob() {
        let frame = frame_with_glyphs(&[(600, 660)]);
        assert!(segment(&frame, &TextRegion::right_of_bar(&bar(), &frame)).is_empty());
    }

    #[test]
    fn empty_region_yields_nothing() {
        let frame = frame_with_glyphs(&[]);
        assert!(segment(&frame, &TextRegion::right_of_bar(&bar(), &frame)).is_empty());
    }
}
