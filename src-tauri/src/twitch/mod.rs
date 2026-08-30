//! Resolving Twitch channels to playable HLS streams.

mod client;
mod error;
mod playlist;

pub use client::{LiveChannel, PlaybackToken, TwitchClient, CLIENT_ID, PLAYBACK_TOKEN_QUERY_HASH};
pub use error::{Error, Result};
pub use playlist::{MasterPlaylist, Variant};
