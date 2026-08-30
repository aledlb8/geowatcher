//! Turning a stream URL into sampled grayscale frames.

mod ffmpeg;
mod frame;

pub use ffmpeg::{Capture, CaptureOptions, Error};
pub use frame::Frame;

/// Every frame is normalised to this size so calibration coordinates are
/// comparable across channels. Matches Twitch's 480p30 tier exactly.
pub const ANALYSIS_WIDTH: u32 = 852;
pub const ANALYSIS_HEIGHT: u32 = 480;
