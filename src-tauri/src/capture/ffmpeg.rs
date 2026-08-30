//! Decodes an HLS stream into sampled grayscale frames via an ffmpeg child process.

use std::path::PathBuf;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use super::frame::Frame;
use super::locate;
use super::{ANALYSIS_HEIGHT, ANALYSIS_WIDTH};

/// Frames held before the reader starts dropping. Small on purpose: a backlog
/// means analysing stale frames, and staying near the live edge matters more.
const CHANNEL_DEPTH: usize = 2;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not start ffmpeg at `{path}`: {source}")]
    Spawn {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("ffmpeg produced no usable output")]
    NoOutput,
}

#[derive(Debug, Clone)]
pub struct CaptureOptions {
    /// Media playlist URL from [`crate::twitch::Variant`].
    pub url: String,
    /// Frames per second handed to the detector.
    pub fps: f32,
    /// ffmpeg executable: the bundled sidecar, or whatever `PATH` provides.
    pub ffmpeg: PathBuf,
}

impl CaptureOptions {
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            fps: 3.0,
            ffmpeg: locate::ffmpeg(""),
        }
    }

    #[must_use]
    pub fn fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    #[must_use]
    pub fn ffmpeg(mut self, path: impl Into<PathBuf>) -> Self {
        self.ffmpeg = path.into();
        self
    }
}

/// A running ffmpeg process yielding frames. Killing it on drop is deliberate:
/// an orphaned ffmpeg keeps pulling the stream forever.
#[derive(Debug)]
pub struct Capture {
    child: Child,
    frames: mpsc::Receiver<Frame>,
}

impl Capture {
    pub fn start(options: &CaptureOptions) -> Result<Self, Error> {
        let mut command = Command::new(&options.ffmpeg);
        command
            .args(["-hide_banner", "-loglevel", "error"])
            // No `-fflags nobuffer`: it breaks HLS demuxing and every frame
            // comes out identical. Latency is handled by fast_bread upstream.
            .args(["-i", &options.url])
            .arg("-an")
            .args([
                "-vf",
                &format!(
                    "fps={},scale={ANALYSIS_WIDTH}:{ANALYSIS_HEIGHT}",
                    options.fps
                ),
            ])
            .args(["-pix_fmt", "gray", "-f", "rawvideo", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Without this a console window flashes up on every capture.
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|source| Error::Spawn {
            path: options.ffmpeg.clone(),
            source,
        })?;

        let stdout = child.stdout.take().ok_or(Error::NoOutput)?;
        let (tx, frames) = mpsc::channel(CHANNEL_DEPTH);
        tokio::spawn(read_frames(stdout, tx));

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(log_stderr(stderr));
        }

        Ok(Self { child, frames })
    }

    /// Next frame, or `None` once ffmpeg exits.
    pub async fn next_frame(&mut self) -> Option<Frame> {
        self.frames.recv().await
    }

    pub async fn shutdown(mut self) {
        let _ = self.child.kill().await;
    }
}

/// Reads fixed-size frames until the stream ends. Drops rather than blocks when
/// the consumer is behind, so ffmpeg never stalls waiting on us.
async fn read_frames(mut stdout: tokio::process::ChildStdout, tx: mpsc::Sender<Frame>) {
    let frame_bytes = (ANALYSIS_WIDTH as usize) * (ANALYSIS_HEIGHT as usize);
    let mut buffer = vec![0u8; frame_bytes];

    loop {
        // A short read here means ffmpeg exited mid-frame.
        if stdout.read_exact(&mut buffer).await.is_err() {
            break;
        }
        let frame = Frame::new(ANALYSIS_WIDTH, ANALYSIS_HEIGHT, buffer.clone());
        match tx.try_send(frame) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::trace!("detector behind, dropping frame");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => break,
        }
    }
}

async fn log_stderr(stderr: tokio::process::ChildStderr) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        tracing::warn!(target: "ffmpeg", "{line}");
    }
}
