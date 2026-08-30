//! Replays captured frames through the run tracker.
//!
//! ```text
//! cargo run --example runs -- ./frames 70
//! ```

use std::path::Path;

use geowatcher_lib::capture::Frame;
use geowatcher_lib::detect::{builtin, candidates, read_percentage, BarRegion, Calibrator};
use geowatcher_lib::run::{Config, Event, Sample, Tracker};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let alert_at: f64 = std::env::args()
        .nth(2)
        .and_then(|n| n.parse().ok())
        .unwrap_or(70.0);

    let frames = load_dir(&dir);
    if frames.is_empty() {
        eprintln!("no frames in {dir}");
        std::process::exit(1);
    }

    let Some(bar) = locate_bar(&frames) else {
        eprintln!("no bar found");
        std::process::exit(1);
    };

    let templates = builtin();
    let mut tracker = Tracker::new(Config {
        alert_at,
        ..Config::default()
    });

    let (mut attempts, mut real, mut alerts) = (0, 0, 0);

    for (index, frame) in frames.iter().enumerate() {
        let sample = Sample::new(
            read_percentage(frame, &bar, &templates).map(|r| r.value),
            bar.fill_ratio(frame).map(|r| r * 100.0),
        );
        for event in tracker.observe(sample) {
            match event {
                Event::Started { at, from_start } => {
                    attempts += 1;
                    if from_start {
                        real += 1;
                    }
                    println!(
                        "[{index:>3}] start {at:>6.2}%  {}",
                        if from_start { "real" } else { "START POS" }
                    );
                }
                Event::Alert { value } => {
                    alerts += 1;
                    println!("[{index:>3}] ALERT {value:>6.2}%");
                }
                Event::Ended { peak, from_start } => {
                    println!(
                        "[{index:>3}] end   {peak:>6.2}%  {}",
                        if from_start { "real" } else { "start pos" }
                    );
                }
                Event::Progressed { .. } => {}
            }
        }
    }

    println!("\n{attempts} attempts ({real} from the start), {alerts} alerts");
    println!("best real attempt: {:.2}%", tracker.best());
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

fn load_dir(dir: &str) -> Vec<Frame> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pgm"))
        .collect();
    // Numeric order, so frame_9 comes before frame_10.
    paths.sort_by_key(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.rsplit('_').next())
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    });
    paths.iter().filter_map(|p| load_pgm(p)).collect()
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
