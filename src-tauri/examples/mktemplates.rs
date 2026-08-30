//! Builds glyph templates from captured frames.
//!
//! Glyphs are clustered by appearance first, then each whole cluster is named
//! by majority vote. The votes come from the progress bar, which measures the
//! same number the text spells out. Individual bar readings are sometimes
//! wrong, but a cluster collects hundreds of them, so the wrong ones lose.
//!
//! ```text
//! cargo run --example mktemplates -- dir_a dir_b > src/detect/templates.rs
//! ```

use std::collections::HashMap;
use std::path::Path;

use geowatcher_lib::capture::Frame;
use geowatcher_lib::detect::{
    candidates, is_decimal_point, segment, BarRegion, Calibrator, GlyphBitmap, TextRegion, GLYPH_H,
    GLYPH_W,
};

/// The bar is only dependable in mid-range: near empty its fill is a few pixels
/// and reads far too low, near full it saturates on death flashes.
const TRUSTED_BAR: std::ops::Range<f64> = 12.0..96.0;

/// Clear of integer boundaries, where a small bar error changes the digit.
const SAFE_FRACTION: std::ops::RangeInclusive<f64> = 0.2..=0.8;

/// Similarity needed to join an existing cluster.
const JOIN: f64 = 0.88;

/// Clusters smaller than this are noise.
const MIN_MEMBERS: usize = 8;

/// Votes needed before a cluster's label is believed.
const MIN_VOTES: usize = 3;

/// Variants kept per label. Texture packs change the font, so one bitmap per
/// digit only ever matches the channels it was built from.
const MAX_VARIANTS: usize = 6;

#[derive(Default)]
struct Cluster {
    members: Vec<GlyphBitmap>,
    votes: HashMap<char, usize>,
}

fn main() {
    let dirs: Vec<String> = std::env::args().skip(1).collect();
    if dirs.is_empty() {
        eprintln!("usage: mktemplates <dir>...");
        std::process::exit(1);
    }

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut frames_seen = 0;
    let mut frames_voted = 0;

    for dir in &dirs {
        let frames = load_dir(dir);
        let Some(bar) = locate_bar(&frames) else {
            eprintln!("{dir}: no bar, skipping");
            continue;
        };
        for frame in &frames {
            frames_seen += 1;
            if collect(frame, &bar, &mut clusters) {
                frames_voted += 1;
            }
        }
    }

    eprintln!("{frames_seen} frames, {frames_voted} contributed labels");
    eprintln!("{} clusters before pruning", clusters.len());

    let labelled = name_clusters(&clusters);
    for (label, cluster) in &labelled {
        eprintln!("  '{label}': {} samples", cluster.members.len());
    }
    emit(&labelled);
}

/// Adds a frame's glyphs to clusters, voting on labels when the bar is usable.
fn collect(frame: &Frame, bar: &BarRegion, clusters: &mut Vec<Cluster>) -> bool {
    let glyphs = segment(frame, &TextRegion::right_of_bar(bar, frame));
    if glyphs.is_empty() {
        return false;
    }

    let hints = label_hints(frame, bar, &glyphs);
    let voted = !hints.is_empty();

    for (index, glyph) in glyphs.iter().enumerate() {
        if is_decimal_point(glyph) {
            continue;
        }
        let bitmap = GlyphBitmap::from_frame(frame, glyph);
        let vote = hints.get(&index).copied();
        add_to_clusters(clusters, bitmap, vote);
    }
    voted
}

/// Position-to-character hints from the bar, when it is worth believing.
fn label_hints(
    frame: &Frame,
    bar: &BarRegion,
    glyphs: &[geowatcher_lib::detect::GlyphBox],
) -> HashMap<usize, char> {
    let Some(ratio) = bar.fill_ratio(frame) else {
        return HashMap::new();
    };
    let value = ratio * 100.0;
    if !TRUSTED_BAR.contains(&value) || !SAFE_FRACTION.contains(&value.fract()) {
        return HashMap::new();
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let digits = (value.trunc() as u32).to_string();

    // Expect <digits> . <two decimals> <percent sign>.
    if glyphs.iter().position(is_decimal_point) != Some(digits.len())
        || glyphs.len() != digits.len() + 4
    {
        return HashMap::new();
    }

    let mut hints: HashMap<usize, char> = digits.chars().enumerate().collect();
    hints.insert(glyphs.len() - 1, '%');
    hints
}

/// Places a bitmap in the closest cluster, or starts a new one.
fn add_to_clusters(clusters: &mut Vec<Cluster>, bitmap: GlyphBitmap, vote: Option<char>) {
    let best = clusters
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.members.first().map(|m| (i, bitmap.similarity(m))))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|&(_, score)| score >= JOIN)
        .map(|(i, _)| i);

    let index = if let Some(index) = best {
        index
    } else {
        clusters.push(Cluster::default());
        clusters.len() - 1
    };

    clusters[index].members.push(bitmap);
    if let Some(label) = vote {
        *clusters[index].votes.entry(label).or_insert(0) += 1;
    }
}

/// Keeps every well-supported cluster, several per label.
fn name_clusters(clusters: &[Cluster]) -> Vec<(char, &Cluster)> {
    let mut by_label: HashMap<char, Vec<(usize, &Cluster)>> = HashMap::new();

    for cluster in clusters {
        if cluster.members.len() < MIN_MEMBERS {
            continue;
        }
        let Some((&label, &votes)) = cluster.votes.iter().max_by_key(|(_, &n)| n) else {
            continue;
        };
        if votes < MIN_VOTES {
            continue;
        }
        by_label.entry(label).or_default().push((votes, cluster));
    }

    let mut out = Vec::new();
    let mut labels: Vec<char> = by_label.keys().copied().collect();
    labels.sort_unstable();

    for label in labels {
        let mut variants = by_label.remove(&label).unwrap_or_default();
        variants.sort_by_key(|(votes, cluster)| (std::cmp::Reverse(*votes), cluster.members.len()));
        for (_, cluster) in variants.into_iter().take(MAX_VARIANTS) {
            out.push((label, cluster));
        }
    }
    out
}

/// Per-cell majority across a cluster's members.
fn majority(list: &[GlyphBitmap]) -> Vec<bool> {
    (0..(GLYPH_W * GLYPH_H) as usize)
        .map(|i| list.iter().filter(|b| b.cells()[i]).count() * 2 >= list.len())
        .collect()
}

fn emit(labelled: &[(char, &Cluster)]) {
    println!("//! Generated by `cargo run --example mktemplates`. Do not edit by hand.");
    println!();
    println!("/// Label and its normalised bitmap, one bit per cell.");
    println!("pub(super) const TEMPLATES: &[(char, &str)] = &[");
    for (label, cluster) in labelled {
        let bits: String = majority(&cluster.members)
            .into_iter()
            .map(|on| if on { '1' } else { '0' })
            .collect();
        println!(
            "    ('{label}', \"{bits}\"), // {} samples",
            cluster.members.len()
        );
    }
    println!("];");
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
    paths.sort();
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
