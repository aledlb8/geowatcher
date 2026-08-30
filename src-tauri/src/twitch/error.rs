//! Errors while resolving a channel's stream.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Offline or nonexistent; usher does not distinguish them.
    #[error("channel `{0}` is offline")]
    Offline(String),

    /// Usually means the persisted query hash is stale.
    #[error("no playback token in response for `{0}`")]
    NoPlaybackToken(String),

    #[error("no video variants advertised for `{0}`")]
    NoVariants(String),

    #[error("twitch request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("unexpected twitch response: {0}")]
    Malformed(String),
}

pub type Result<T> = std::result::Result<T, Error>;
