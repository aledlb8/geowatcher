//! Finding the ffmpeg executable.
//!
//! Release builds ship ffmpeg as a Tauri sidecar next to the app binary, but
//! `cargo run` and `tauri dev` have no bundle, so `PATH` stays a fallback.

use std::path::{Path, PathBuf};

/// Target triple this build was compiled for, set by `build.rs`.
const TRIPLE: &str = env!("TARGET_TRIPLE");

const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

/// Locates ffmpeg, preferring an explicit setting, then a bundled sidecar.
///
/// Falls back to the bare name so the OS resolves it on `PATH`; that is what
/// makes a development checkout work with no bundling step.
#[must_use]
pub fn ffmpeg(configured: &str) -> PathBuf {
    let configured = configured.trim();
    if !configured.is_empty() {
        return PathBuf::from(configured);
    }
    sidecar().unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// Looks for a bundled ffmpeg beside the running executable, and in the
/// source tree so `cargo run` picks up a fetched binary too.
fn sidecar() -> Option<PathBuf> {
    let names = [
        format!("ffmpeg{EXE_SUFFIX}"),
        format!("ffmpeg-{TRIPLE}{EXE_SUFFIX}"),
    ];

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.to_path_buf());
        }
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"));

    roots
        .iter()
        .flat_map(|root| names.iter().map(move |name| root.join(name)))
        .find(|path| is_executable(path))
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::ffmpeg;

    #[test]
    fn an_explicit_setting_wins() {
        let path = if cfg!(windows) {
            r"C:\tools\ffmpeg.exe"
        } else {
            "/opt/ffmpeg"
        };
        assert_eq!(ffmpeg(path).to_string_lossy(), path);
    }

    #[test]
    fn whitespace_is_not_a_path() {
        assert_ne!(ffmpeg("   ").to_string_lossy(), "   ");
    }

    #[test]
    fn falls_back_to_a_name_path_can_resolve() {
        // With no sidecar present this must stay usable rather than empty.
        let resolved = ffmpeg("");
        assert!(resolved.to_string_lossy().contains("ffmpeg"));
    }
}
