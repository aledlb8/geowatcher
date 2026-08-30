//! Opening a channel for viewing.
//!
//! mpv gets a direct HLS URL, which stays far nearer the live edge than the
//! Twitch web player. When mpv is not installed the browser is the fallback.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// How a channel ended up being opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    Mpv,
    Browser,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not open a browser for `{channel}`: {source}")]
    Browser {
        channel: String,
        source: std::io::Error,
    },
}

/// Resolves the mpv executable, honouring a configured path.
#[must_use]
pub fn find_mpv(configured: &str) -> Option<PathBuf> {
    let candidate = if configured.trim().is_empty() {
        PathBuf::from("mpv")
    } else {
        PathBuf::from(configured.trim())
    };

    // An absolute path is only usable if it is actually there; a bare name has
    // to be probed, since PATH lookup is the OS's job.
    if candidate.is_absolute() {
        return candidate.is_file().then_some(candidate);
    }
    probe(&candidate).then_some(candidate)
}

/// Runs `--version` to see whether the executable resolves at all.
fn probe(program: &PathBuf) -> bool {
    let mut command = Command::new(program);
    command
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    no_console(&mut command);
    command.status().is_ok()
}

/// Opens `stream_url` in mpv, or the channel page in a browser.
///
/// `stream_url` should be a media playlist URL; passing `None` skips mpv, which
/// is what happens when the channel could not be resolved.
pub fn open(channel: &str, stream_url: Option<&str>, mpv_path: &str) -> Result<Opened, Error> {
    if let (Some(url), Some(mpv)) = (stream_url, find_mpv(mpv_path)) {
        let mut command = Command::new(mpv);
        command
            .args([
                // Keep playback at the live edge rather than buffering ahead.
                "--profile=low-latency",
                "--cache=no",
                &format!("--force-media-title={channel}"),
                url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        no_console(&mut command);

        if command.spawn().is_ok() {
            return Ok(Opened::Mpv);
        }
        tracing::warn!("mpv failed to start, falling back to the browser");
    }

    open_browser(channel)
}

fn open_browser(channel: &str) -> Result<Opened, Error> {
    let url = format!("https://twitch.tv/{channel}");
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|err| Error::Browser {
        channel: channel.to_owned(),
        source: std::io::Error::other(err.to_string()),
    })?;
    Ok(Opened::Browser)
}

/// Stops a console window flashing up on Windows.
fn no_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::find_mpv;

    #[test]
    fn a_missing_absolute_path_resolves_to_nothing() {
        let missing = if cfg!(windows) {
            r"C:\definitely\not\here\mpv.exe"
        } else {
            "/definitely/not/here/mpv"
        };
        assert!(find_mpv(missing).is_none());
    }

    #[test]
    fn a_bare_name_that_does_not_exist_resolves_to_nothing() {
        assert!(find_mpv("mpv-that-is-not-installed-xyz").is_none());
    }

    #[test]
    fn whitespace_is_treated_as_unconfigured() {
        // Falls back to probing plain `mpv`, which may or may not be present;
        // either way it must not panic or treat "   " as a path.
        let _ = find_mpv("   ");
    }
}
