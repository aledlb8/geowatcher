//! Captures analysis frames from a live channel and writes them as PGM.
//!
//! ```text
//! cargo run --example capture -- zoinkgd 5
//! ```

use std::io::Write;
use std::time::Instant;

use geowatcher_lib::capture::{Capture, CaptureOptions};
use geowatcher_lib::twitch::TwitchClient;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let channel = args.next().unwrap_or_else(|| "zoinkgd".to_owned());
    let wanted: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(5);

    let client = TwitchClient::new().expect("build client");
    let playlist = match client.master_playlist(&channel, true).await {
        Ok(playlist) => playlist,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let Some(variant) = playlist.best_for_analysis() else {
        eprintln!("no usable variant");
        std::process::exit(1);
    };
    println!("{channel}: capturing {} at 3 fps", variant.name);

    let options = CaptureOptions::new(variant.url.clone()).fps(3.0);
    let mut capture = match Capture::start(&options) {
        Ok(capture) => capture,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let started = Instant::now();
    let mut count = 0;

    while count < wanted {
        let Some(frame) = capture.next_frame().await else {
            eprintln!("stream ended after {count} frames");
            break;
        };
        count += 1;

        let path = format!("frame_{count:02}.pgm");
        let mut file = std::fs::File::create(&path).expect("create pgm");
        write!(file, "P5\n{} {}\n255\n", frame.width, frame.height).expect("write header");
        file.write_all(&frame.gray).expect("write pixels");

        println!(
            "  {path}  {}x{}  mean {:.1}  at {:.1}s",
            frame.width,
            frame.height,
            frame.mean_luminance(),
            started.elapsed().as_secs_f32(),
        );
    }

    capture.shutdown().await;
}
