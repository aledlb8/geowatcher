//! Cross-checks the OCR against the progress bar over captured frames.
//!
//! The two measure the same number by completely different means, so where they
//! agree both are almost certainly right, and where they disagree one of them
//! needs looking at.
//!
//! ```text
//! cargo run --example validate -- dir_a dir_b
//! ```

use std::path::Path;

use geowatcher_lib::capture::Frame;
use geowatcher_lib::detect::{builtin, candidates, read_percentage, BarRegion, Calibrator};

/// Bar and text this far apart are treated as disagreeing.
const TOLERANCE: f64 = 1.0;

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        eprintln!("usage: validate <dir>...");
        std::process::exit(1);
    }

    let templates = builtin();
    println!(
        "templates: {} entries covering {:?}\n",
        templates.len(),
        templates.labels()
    );

    let (mut agree, mut disagree, mut unread, mut nobar) = (0, 0, 0, 0);
    let mut worst: Vec<(f64, String, String, f64, f64)> = Vec::new();

    for dir in &dirs {
        let (paths, frames) = load_dir(dir);
        let Some(bar) = locate_bar(&frames) else {
            println!("{dir}: no bar");
            continue;
        };

        for (path, frame) in paths.iter().zip(&frames) {
            let Some(ratio) = bar.fill_ratio(frame) else {
                nobar += 1;
                continue;
            };
            let from_bar = ratio * 100.0;

            match read_percentage(frame, &bar, &templates) {
                Some(reading) => {
                    let delta = (reading.value - from_bar).abs();
                    if delta <= TOLERANCE {
                        agree += 1;
                    } else {
                        disagree += 1;
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        worst.push((
                            delta,
                            format!("{dir}/{name}"),
                            reading.text.clone(),
                            reading.value,
                            from_bar,
                        ));
                    }
                }
                None => unread += 1,
            }
        }

        // A run should climb smoothly; sudden jumps mean a misread.
        let series: Vec<f64> = frames
            .iter()
            .filter_map(|f| read_percentage(f, &bar, &templates).map(|r| r.value))
            .collect();
        let steps: Vec<f64> = series.windows(2).map(|w| w[1] - w[0]).collect();
        let smooth = steps.iter().filter(|&&d| (0.0..3.0).contains(&d)).count();
        let resets = steps.iter().filter(|&&d| d < -5.0).count();
        println!(
            "{dir}: {} readings, {smooth} smooth steps, {resets} resets, {} jumps",
            series.len(),
            steps.len() - smooth - resets
        );
    }

    let total = agree + disagree;
    println!("agree      {agree}");
    println!("disagree   {disagree}");
    println!("unreadable {unread}   (no text, or a glyph below threshold)");
    println!("no bar     {nobar}");
    if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        let rate = f64::from(agree) / f64::from(total) * 100.0;
        println!("\nagreement on readable frames: {rate:.1}%");
    }

    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !worst.is_empty() {
        println!("\nlargest disagreements:");
        for (delta, path, text, value, from_bar) in worst.iter().take(12) {
            println!("  {path}: read {text:?} = {value:.2}, bar {from_bar:.2} (off by {delta:.2})");
        }
    }
}

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

fn load_dir(dir: &str) -> (Vec<std::path::PathBuf>, Vec<Frame>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pgm"))
        .collect();
    paths.sort();

    let mut kept = Vec::new();
    let mut frames = Vec::new();
    for path in paths {
        if let Some(frame) = load_pgm(&path) {
            kept.push(path);
            frames.push(frame);
        }
    }
    (kept, frames)
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
