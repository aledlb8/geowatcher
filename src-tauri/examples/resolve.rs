//! Resolves a Twitch channel to an analysable stream URL.
//!
//! A development aid for checking the Twitch endpoints still behave, without
//! launching the desktop app:
//!
//! ```text
//! cargo run --example resolve -- zoinkgd
//! cargo run --example resolve            # lists live Geometry Dash channels
//! ```

use geowatcher_lib::twitch::{Error, TwitchClient};

#[tokio::main]
async fn main() {
    let client = match TwitchClient::new() {
        Ok(client) => client,
        Err(err) => {
            eprintln!("could not build client: {err}");
            std::process::exit(1);
        }
    };

    let Some(channel) = std::env::args().nth(1) else {
        list_directory(&client).await;
        return;
    };

    match client.master_playlist(&channel, true).await {
        Ok(playlist) => {
            let names: Vec<_> = playlist.variants.iter().map(|v| v.name.as_str()).collect();
            println!("{channel} is live");
            println!("  variants: {}", names.join(", "));

            if let Some(best) = playlist.best_for_analysis() {
                let (w, h) = best.resolution.unwrap_or((0, 0));
                println!(
                    "  chosen:   {} ({w}x{h} @ {:?} fps)",
                    best.name, best.frame_rate
                );
                println!("  url len:  {} bytes", best.url.len());
            }
        }
        Err(Error::Offline(who)) => println!("{who} is offline"),
        Err(err) => {
            eprintln!("failed: {err}");
            std::process::exit(1);
        }
    }
}

/// Prints whoever is currently live in the Geometry Dash directory.
async fn list_directory(client: &TwitchClient) {
    match client.live_channels_for_game("Geometry Dash", 10).await {
        Ok(channels) if channels.is_empty() => println!("nobody is live"),
        Ok(channels) => {
            println!("live Geometry Dash channels:");
            for channel in channels {
                println!("  {:<24} {:>6} viewers", channel.login, channel.viewers);
            }
        }
        Err(err) => {
            eprintln!("directory query failed: {err}");
            std::process::exit(1);
        }
    }
}
