//! Resolving Twitch channels to playable HLS streams.
//!
//! The watcher needs three things from Twitch and nothing more: whether a
//! channel is live, a playlist URL for it, and a quality tier cheap enough to
//! decode continuously. Everything here serves one of those three.

mod client;
mod error;
mod playlist;

pub use client::{LiveChannel, PlaybackToken, TwitchClient, CLIENT_ID, PLAYBACK_TOKEN_QUERY_HASH};
pub use error::{Error, Result};
pub use playlist::{MasterPlaylist, Variant};
