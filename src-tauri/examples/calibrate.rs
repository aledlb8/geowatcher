//! Runs bar calibration over a directory of PGM frames written by the capture
//! example, then reports the fill each frame would read.
//!
//! ```text
//! cargo run --example calibrate -- ./frames
//! ```

use std::path::Path;

use geowatcher_lib::capture::Frame;
use geowatcher_lib::detect::Calibrator;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {dir}: {e}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pgm"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        eprintln!("no .pgm files in {dir}");
        std::process::exit(1);
    }

    let frames: Vec<Frame> = paths.iter().filter_map(|p| load_pgm(p)).collect();
    let mut calibrator = Calibrator::new();
    for frame in &frames {
        calibrator.observe(frame);
    }

    let Some(calibration) = calibrator.best() else {
        println!(
            "{dir}: no bar found across {} frames",
            calibrator.frames_seen()
        );
        return;
    };

    let bar = calibration.bar;
    println!(
        "{dir}: bar x={}..{} (w={}) y={}..{}  confidence {:.0}%{}",
        bar.x0,
        bar.x1,
        bar.width(),
        bar.y_top,
        bar.y_bottom,
        calibration.confidence * 100.0,
        if calibration.is_confident() {
            ""
        } else {
            "  [low - would prompt]"
        },
    );

    for (path, frame) in paths.iter().zip(&frames) {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match bar.fill_ratio(frame) {
            Some(ratio) => println!("   {name}  {:6.2}%", ratio * 100.0),
            None => println!("   {name}   --"),
        }
    }
}

/// Minimal binary PGM reader for the capture example's output.
fn load_pgm(path: &Path) -> Option<Frame> {
    let data = std::fs::read(path).ok()?;
    let mut parts = data.splitn(4, |&b| b == b'\n');

    if parts.next()? != b"P5" {
        return None;
    }
    let dims = std::str::from_utf8(parts.next()?).ok()?;
    let (w, h) = dims.trim().split_once(' ')?;
    let (width, height) = (w.parse().ok()?, h.parse().ok()?);
    parts.next()?; // maxval
    let pixels = parts.next()?.to_vec();

    Some(Frame::new(width, height, pixels))
}
