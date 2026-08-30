//! Segments the percentage text out of captured frames and renders each glyph
//! as ASCII, for building and checking templates.
//!
//! ```text
//! cargo run --example glyphs -- ./frames
//! ```

use std::path::Path;

use geowatcher_lib::capture::Frame;
use geowatcher_lib::detect::{candidates, segment, BarRegion, Calibrator, TextRegion};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let limit: usize = std::env::args()
        .nth(2)
        .and_then(|n| n.parse().ok())
        .unwrap_or(2);

    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pgm"))
        .collect();
    paths.sort();

    let frames: Vec<Frame> = paths.iter().filter_map(|p| load_pgm(p)).collect();
    if frames.is_empty() {
        eprintln!("no frames in {dir}");
        std::process::exit(1);
    }

    let Some(bar) = locate_bar(&frames) else {
        eprintln!("no bar found; cannot place the text region");
        std::process::exit(1);
    };
    println!(
        "bar x={}..{} y={}..{}",
        bar.x0, bar.x1, bar.y_top, bar.y_bottom
    );

    for (path, frame) in paths.iter().zip(&frames).take(limit) {
        let region = TextRegion::right_of_bar(&bar, frame);
        let glyphs = segment(frame, &region);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        println!("\n{name}: {} glyphs", glyphs.len());

        for glyph in glyphs {
            println!(
                "  x={}..{} y={}..{}  {}x{}",
                glyph.x0,
                glyph.x1,
                glyph.y0,
                glyph.y1,
                glyph.width(),
                glyph.height()
            );
            for y in glyph.y0..=glyph.y1 {
                let row: String = (glyph.x0..=glyph.x1)
                    .map(|x| if frame.pixel(x, y) >= 200 { '#' } else { '.' })
                    .collect();
                println!("    {row}");
            }
        }
    }
}

/// Prefers a calibrated bar; falls back to the first frame that shows one.
fn locate_bar(frames: &[Frame]) -> Option<BarRegion> {
    let mut calibrator = Calibrator::new();
    for frame in frames {
        calibrator.observe(frame);
    }
    calibrator
        .best()
        .map(|c| c.bar)
        .or_else(|| frames.iter().find_map(|f| candidates(f).into_iter().next()))
}

fn load_pgm(path: &Path) -> Option<Frame> {
    let data = std::fs::read(path).ok()?;
    let mut parts = data.splitn(4, |&b| b == b'\n');
    if parts.next()? != b"P5" {
        return None;
    }
    let dims = std::str::from_utf8(parts.next()?).ok()?;
    let (w, h) = dims.trim().split_once(' ')?;
    let (width, height) = (w.parse().ok()?, h.parse().ok()?);
    parts.next()?;
    Some(Frame::new(width, height, parts.next()?.to_vec()))
}
