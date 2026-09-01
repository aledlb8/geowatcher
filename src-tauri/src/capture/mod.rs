//! Turning a stream URL into sampled grayscale frames.

mod ffmpeg;
mod frame;
mod locate;

pub use ffmpeg::{Capture, CaptureOptions, Error, PooledFrame};
pub use frame::Frame;
pub use locate::ffmpeg as locate_ffmpeg;

/// Every frame is normalised to this size so calibration coordinates are
/// comparable across channels. Matches Twitch's 480p30 tier exactly.
pub const ANALYSIS_WIDTH: u32 = 852;
pub const ANALYSIS_HEIGHT: u32 = 480;
