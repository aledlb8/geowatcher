//! Runs the supervisor against live channels and prints what it sees.
//!
//! ```text
//! cargo run --example watch -- zoinkgd 120 40
//! ```
//! Arguments: channel, seconds to watch, alert threshold.

use std::time::Duration;

use geowatcher_lib::config::Settings;
use geowatcher_lib::supervisor::{Supervisor, Update};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let login = args.next().unwrap_or_else(|| "zoinkgd".to_owned());
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(120);
    let alert_at: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(40.0);

    let mut settings = Settings {
        alert_at,
        ..Settings::default()
    };
    settings.add_channel(&login);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut supervisor = Supervisor::new(tx);
    for channel in &settings.channels {
        supervisor.watch(channel, &settings);
    }

    println!("watching {login} for {seconds}s, alerting at {alert_at}%\n");

    let deadline = tokio::time::sleep(Duration::from_secs(seconds));
    tokio::pin!(deadline);

    let mut progress = 0u32;
    loop {
        tokio::select! {
            () = &mut deadline => break,
            Some(update) = rx.recv() => match update {
                Update::Status { login, status } => {
                    println!("  [{login}] {status:?}");
                }
                Update::Progress { value, .. } => {
                    progress += 1;
                    // Progress is noisy at 3 fps; sample it.
                    if progress % 10 == 0 {
                        println!("      {value:.2}%");
                    }
                }
                Update::Alert { login, value, .. } => {
                    println!("  [{login}] ALERT {value:.2}%");
                }
                Update::RunEnded { login, peak, from_start } => {
                    println!(
                        "  [{login}] run ended at {peak:.2}% ({})",
                        if from_start { "real" } else { "start pos" }
                    );
                }
            },
        }
    }

    supervisor.stop_all();
    println!("\ndone");
}
