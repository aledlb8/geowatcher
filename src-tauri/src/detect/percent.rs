//! Reading the percentage as a number.

use crate::capture::Frame;

use super::bar::BarRegion;
use super::glyph::{is_decimal_point, GlyphBitmap, TemplateSet};
use super::text::{segment, TextRegion};

/// Percentages above this are impossible and mean a misread.
const MAX_VALUE: f64 = 100.0;

/// A percentage recovered from the frame.
#[derive(Debug, Clone)]
pub struct Reading {
    pub value: f64,
    /// Similarity of the worst-matching glyph.
    pub confidence: f64,
    /// Raw recognised characters, useful when a read looks wrong.
    pub text: String,
}

/// Reads the percentage next to `bar`, or `None` if it cannot be trusted.
#[must_use]
pub fn read_percentage(frame: &Frame, bar: &BarRegion, templates: &TemplateSet) -> Option<Reading> {
    if templates.is_empty() {
        return None;
    }

    let glyphs = segment(frame, &TextRegion::right_of_bar(bar, frame));
    if glyphs.is_empty() {
        return None;
    }

    let mut text = String::new();
    let mut confidence = 1.0_f64;

    for glyph in &glyphs {
        if is_decimal_point(glyph) {
            text.push('.');
            continue;
        }
        let (label, score) = templates.best_match(&GlyphBitmap::from_frame(frame, glyph))?;
        text.push(label);
        confidence = confidence.min(score);
    }

    let value = parse(&text)?;
    Some(Reading {
        value,
        confidence,
        text,
    })
}

/// Accepts `54.52%` and `8%`; anything else is a misread.
fn parse(text: &str) -> Option<f64> {
    let digits = text.strip_suffix('%')?;
    if digits.is_empty() {
        return None;
    }

    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (digits, None),
    };

    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }

    let value: f64 = digits.parse().ok()?;
    (value <= MAX_VALUE).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_a_normal_reading() {
        assert_eq!(parse("54.52%"), Some(54.52));
    }

    #[test]
    fn parses_a_whole_number() {
        assert_eq!(parse("8%"), Some(8.0));
        assert_eq!(parse("100%"), Some(100.0));
    }

    #[test]
    fn rejects_impossible_values() {
        assert_eq!(parse("154.52%"), None);
        assert_eq!(parse("100.01%"), None);
    }

    #[test]
    fn rejects_malformed_text() {
        for bad in ["", "%", "54.52", ".52%", "54..52%", "54.%", "5a.2%"] {
            assert_eq!(parse(bad), None, "accepted {bad:?}");
        }
    }
}
