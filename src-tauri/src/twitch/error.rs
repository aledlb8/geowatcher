//! Errors produced while talking to Twitch.

/// A failure while resolving a channel's stream.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The channel exists but is not currently broadcasting.
    ///
    /// Usher answers `404` with `{"error":"Can not find channel"}` both for an
    /// offline channel and for one that does not exist, so the two are not
    /// distinguishable here. The supervisor treats this as "retry later".
    #[error("channel `{0}` is offline")]
    Offline(String),

    /// Twitch accepted the request but returned no playback token, which
    /// generally means the persisted query hash is stale.
    #[error("no playback token in response for `{0}`")]
    NoPlaybackToken(String),

    /// The master playlist parsed but advertised no usable video variant.
    #[error("no video variants advertised for `{0}`")]
    NoVariants(String),

    /// The request itself failed, or returned an unexpected status.
    #[error("twitch request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// A response did not have the shape we expect.
    #[error("unexpected twitch response: {0}")]
    Malformed(String),
}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;
