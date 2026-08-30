//! Normalised glyph bitmaps and template matching.
//!
//! The alphabet is only `0`-`9` and `%`, so scaling each glyph to a fixed grid
//! and comparing bits beats general OCR on both speed and accuracy. The decimal
//! point is recognised by size instead: it is the only glyph a few pixels tall.

use crate::capture::Frame;

use super::text::GlyphBox;

/// Grid every glyph is scaled to before comparison.
pub const GLYPH_W: u32 = 12;
pub const GLYPH_H: u32 = 16;

/// Ink threshold, matching the segmenter.
const INK_LUMA: u8 = 200;

/// A glyph this small is the decimal point.
const DOT_MAX_WIDTH: u32 = 6;
const DOT_MAX_HEIGHT: u32 = 6;

/// Fraction of cells that must agree for a match to count.
const MIN_SIMILARITY: f64 = 0.84;

/// A glyph scaled to [`GLYPH_W`] x [`GLYPH_H`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    cells: Vec<bool>,
}

impl GlyphBitmap {
    /// Scales the glyph onto the fixed grid, preserving aspect ratio.
    ///
    /// Fitting by height and centring horizontally keeps width meaningful, so a
    /// narrow `1` cannot be stretched into something resembling a `0`.
    #[must_use]
    pub fn from_frame(frame: &Frame, glyph: &GlyphBox) -> Self {
        let src_w = glyph.width();
        let src_h = glyph.height();
        let scaled_w = (src_w * GLYPH_H / src_h.max(1)).clamp(1, GLYPH_W);
        let x_offset = (GLYPH_W - scaled_w) / 2;

        let mut cells = vec![false; (GLYPH_W * GLYPH_H) as usize];

        for cy in 0..GLYPH_H {
            let y_start = glyph.y0 + cy * src_h / GLYPH_H;
            let y_end = (glyph.y0 + (cy + 1) * src_h / GLYPH_H).max(y_start + 1);

            for cx in 0..scaled_w {
                let x_start = glyph.x0 + cx * src_w / scaled_w;
                let x_end = (glyph.x0 + (cx + 1) * src_w / scaled_w).max(x_start + 1);

                let mut ink = 0u32;
                let mut total = 0u32;
                for y in y_start..y_end {
                    for x in x_start..x_end {
                        total += 1;
                        if frame.pixel(x, y) >= INK_LUMA {
                            ink += 1;
                        }
                    }
                }
                if total > 0 && ink * 2 >= total {
                    cells[(cy * GLYPH_W + x_offset + cx) as usize] = true;
                }
            }
        }

        Self { cells }
    }

    #[must_use]
    pub fn from_cells(cells: Vec<bool>) -> Option<Self> {
        (cells.len() == (GLYPH_W * GLYPH_H) as usize).then_some(Self { cells })
    }

    #[must_use]
    pub fn cells(&self) -> &[bool] {
        &self.cells
    }

    /// Share of cells that agree, 0.0 to 1.0.
    #[must_use]
    pub fn similarity(&self, other: &Self) -> f64 {
        let agree = self
            .cells
            .iter()
            .zip(&other.cells)
            .filter(|(a, b)| a == b)
            .count();
        let total = u32::try_from(self.cells.len()).unwrap_or(1).max(1);
        f64::from(u32::try_from(agree).unwrap_or(0)) / f64::from(total)
    }

    /// Packs to a bit string, for embedding a built template set in source.
    #[must_use]
    pub fn to_bits(&self) -> String {
        self.cells
            .iter()
            .map(|&c| if c { '1' } else { '0' })
            .collect()
    }

    #[must_use]
    pub fn from_bits(bits: &str) -> Option<Self> {
        Self::from_cells(bits.chars().map(|c| c == '1').collect())
    }
}

/// Whether a box is small enough to be the decimal point.
#[must_use]
pub fn is_decimal_point(glyph: &GlyphBox) -> bool {
    glyph.width() <= DOT_MAX_WIDTH && glyph.height() <= DOT_MAX_HEIGHT
}

/// Labelled reference glyphs.
#[derive(Debug, Clone, Default)]
pub struct TemplateSet {
    entries: Vec<(char, GlyphBitmap)>,
}

impl TemplateSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, label: char, bitmap: GlyphBitmap) {
        self.entries.push((label, bitmap));
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn labels(&self) -> Vec<char> {
        let mut seen: Vec<char> = self.entries.iter().map(|(label, _)| *label).collect();
        seen.sort_unstable();
        seen.dedup();
        seen
    }

    /// Closest label and its similarity, if anything clears the threshold.
    #[must_use]
    pub fn best_match(&self, glyph: &GlyphBitmap) -> Option<(char, f64)> {
        self.entries
            .iter()
            .map(|(label, bitmap)| (*label, glyph.similarity(bitmap)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|&(_, score)| score >= MIN_SIMILARITY)
    }
}

#[cfg(test)]
mod tests {
    use super::{is_decimal_point, GlyphBitmap, TemplateSet, GLYPH_H, GLYPH_W};
    use crate::capture::Frame;
    use crate::detect::GlyphBox;

    fn frame_with_block(x0: u32, x1: u32, y0: u32, y1: u32) -> Frame {
        let (w, h) = (100, 60);
        let mut gray = vec![0u8; (w * h) as usize];
        for y in y0..=y1 {
            for x in x0..=x1 {
                gray[(y * w + x) as usize] = 255;
            }
        }
        Frame::new(w, h, gray)
    }

    #[test]
    fn a_solid_block_normalises_to_all_ink() {
        let frame = frame_with_block(10, 21, 10, 25);
        let glyph = GlyphBox {
            x0: 10,
            x1: 21,
            y0: 10,
            y1: 25,
        };
        let bitmap = GlyphBitmap::from_frame(&frame, &glyph);
        assert_eq!(bitmap.cells().len(), (GLYPH_W * GLYPH_H) as usize);
        assert!(bitmap.cells().iter().all(|&c| c));
    }

    #[test]
    fn scaling_is_size_independent() {
        // The same shape at two sizes should normalise to the same bitmap.
        let small = GlyphBitmap::from_frame(
            &frame_with_block(10, 21, 10, 25),
            &GlyphBox {
                x0: 10,
                x1: 21,
                y0: 10,
                y1: 25,
            },
        );
        let large = GlyphBitmap::from_frame(
            &frame_with_block(10, 33, 10, 41),
            &GlyphBox {
                x0: 10,
                x1: 33,
                y0: 10,
                y1: 41,
            },
        );
        assert_eq!(small, large);
    }

    #[test]
    fn identical_bitmaps_are_fully_similar() {
        let a = GlyphBitmap::from_bits(&"1".repeat((GLYPH_W * GLYPH_H) as usize)).expect("bits");
        assert!((a.similarity(&a) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn inverted_bitmaps_share_nothing() {
        let size = (GLYPH_W * GLYPH_H) as usize;
        let a = GlyphBitmap::from_bits(&"1".repeat(size)).expect("bits");
        let b = GlyphBitmap::from_bits(&"0".repeat(size)).expect("bits");
        assert!(a.similarity(&b) < f64::EPSILON);
    }

    #[test]
    fn bits_round_trip() {
        let size = (GLYPH_W * GLYPH_H) as usize;
        let bits: String = (0..size)
            .map(|i| if i % 3 == 0 { '1' } else { '0' })
            .collect();
        let bitmap = GlyphBitmap::from_bits(&bits).expect("parse");
        assert_eq!(bitmap.to_bits(), bits);
    }

    #[test]
    fn rejects_a_wrong_sized_bitmap() {
        assert!(GlyphBitmap::from_bits("101").is_none());
    }

    #[test]
    fn a_poor_match_is_rejected() {
        let size = (GLYPH_W * GLYPH_H) as usize;
        let mut set = TemplateSet::new();
        set.insert(
            '8',
            GlyphBitmap::from_bits(&"1".repeat(size)).expect("bits"),
        );
        let nothing = GlyphBitmap::from_bits(&"0".repeat(size)).expect("bits");
        assert!(set.best_match(&nothing).is_none());
    }

    #[test]
    fn only_a_tiny_box_is_the_decimal_point() {
        assert!(is_decimal_point(&GlyphBox {
            x0: 625,
            x1: 627,
            y0: 18,
            y1: 20
        }));
        assert!(!is_decimal_point(&GlyphBox {
            x0: 593,
            x1: 604,
            y0: 6,
            y1: 20
        }));
    }
}
