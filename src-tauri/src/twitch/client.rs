//! A minimal Twitch client covering exactly what the watcher needs.
//!
//! This deliberately reimplements the small slice of Streamlink's Twitch plugin
//! we depend on, because Streamlink is a Python application and cannot be
//! bundled into a Tauri app without shipping a Python runtime alongside it.
//!
//! Both endpoints used here are the ones Twitch's own web player uses. They are
//! not a documented public API and can change; [`CLIENT_ID`] and
//! [`PLAYBACK_TOKEN_QUERY_HASH`] are the two values most likely to need
//! updating if playback resolution starts failing.

use serde::Deserialize;

use super::error::{Error, Result};
use super::playlist::MasterPlaylist;

/// The public client id used by Twitch's own web player.
pub const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

/// Persisted-query hash for the `PlaybackAccessToken` operation.
pub const PLAYBACK_TOKEN_QUERY_HASH: &str =
    "0828119ded1c13477966434e15800ff57ddacf13ba1911c129dc2200705b0712";

const GQL_ENDPOINT: &str = "https://gql.twitch.tv/gql";
const USHER_ENDPOINT: &str = "https://usher.ttvnw.net/api/channel/hls";

/// Directory listing query. Uses GraphQL variables so channel and game names
/// are never interpolated into the query text.
const DIRECTORY_QUERY: &str = "query($name: String!, $limit: Int!) { \
     game(name: $name) { streams(first: $limit) { \
     edges { node { viewersCount broadcaster { login } } } } } }";

/// A signed token authorising playlist access for one channel.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaybackToken {
    /// The token payload.
    pub value: String,
    /// Twitch's signature over [`Self::value`].
    pub signature: String,
}

/// A channel that is live in a game directory.
#[derive(Debug, Clone)]
pub struct LiveChannel {
    /// The channel's login name, which is what stream URLs use.
    pub login: String,
    /// Current viewer count.
    pub viewers: u32,
}

/// Talks to Twitch's web endpoints.
#[derive(Debug, Clone)]
pub struct TwitchClient {
    http: reqwest::Client,
}

impl TwitchClient {
    /// Builds a client with a browser-like user agent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Http`] if the TLS backend cannot be initialised.
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/125.0 Safari/537.36",
            )
            .build()?;
        Ok(Self { http })
    }

    /// Requests a playback access token for `channel`.
    ///
    /// # Errors
    ///
    /// [`Error::NoPlaybackToken`] if Twitch returns no token, which usually
    /// means [`PLAYBACK_TOKEN_QUERY_HASH`] has gone stale; [`Error::Http`] if
    /// the request itself fails.
    pub async fn playback_token(&self, channel: &str) -> Result<PlaybackToken> {
        #[derive(Deserialize)]
        struct Response {
            data: Option<Data>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Data {
            stream_playback_access_token: Option<PlaybackToken>,
        }

        let body = serde_json::json!({
            "operationName": "PlaybackAccessToken",
            "variables": {
                "isLive": true,
                "login": channel,
                "isVod": false,
                "vodID": "",
                "playerType": "site",
            },
            "extensions": {
                "persistedQuery": {
                    "version": 1,
                    "sha256Hash": PLAYBACK_TOKEN_QUERY_HASH,
                }
            }
        });

        let response: Response = self
            .http
            .post(GQL_ENDPOINT)
            .header("Client-ID", CLIENT_ID)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response
            .data
            .and_then(|d| d.stream_playback_access_token)
            .ok_or_else(|| Error::NoPlaybackToken(channel.to_owned()))
    }

    /// Fetches and parses the master playlist for a live channel.
    ///
    /// `low_latency` sets the `fast_bread` flag, which is what Streamlink's
    /// `--twitch-low-latency` toggles: it keeps playback nearer the live edge.
    ///
    /// # Errors
    ///
    /// [`Error::Offline`] if Twitch reports no active broadcast,
    /// [`Error::NoVariants`] if the playlist advertises no video, or
    /// [`Error::Http`] if the request fails.
    pub async fn master_playlist(
        &self,
        channel: &str,
        low_latency: bool,
    ) -> Result<MasterPlaylist> {
        let token = self.playback_token(channel).await?;
        let url = format!("{USHER_ENDPOINT}/{channel}.m3u8");

        let response = self
            .http
            .get(&url)
            .query(&[
                ("client_id", CLIENT_ID),
                ("token", token.value.as_str()),
                ("sig", token.signature.as_str()),
                ("allow_source", "true"),
                ("allow_audio_only", "true"),
                ("fast_bread", if low_latency { "true" } else { "false" }),
            ])
            .send()
            .await?;

        // Usher answers 404 both for an offline channel and a nonexistent one.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Offline(channel.to_owned()));
        }

        let body = response.error_for_status()?.text().await?;
        let playlist = MasterPlaylist::parse(&body);

        if playlist.variants.is_empty() {
            return Err(Error::NoVariants(channel.to_owned()));
        }
        Ok(playlist)
    }

    /// Lists channels currently live in a game's directory, busiest first.
    ///
    /// This doubles as a cheap liveness probe: one request covers every channel
    /// in the directory, where [`Self::master_playlist`] costs two per channel.
    ///
    /// # Errors
    ///
    /// [`Error::Malformed`] if the directory response has an unexpected shape,
    /// or [`Error::Http`] if the request fails.
    pub async fn live_channels_for_game(&self, game: &str, limit: u32) -> Result<Vec<LiveChannel>> {
        #[derive(Deserialize)]
        struct Response {
            data: Option<Data>,
        }
        #[derive(Deserialize)]
        struct Data {
            game: Option<Game>,
        }
        #[derive(Deserialize)]
        struct Game {
            streams: Option<Streams>,
        }
        #[derive(Deserialize)]
        struct Streams {
            edges: Vec<Edge>,
        }
        #[derive(Deserialize)]
        struct Edge {
            node: Node,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Node {
            viewers_count: Option<u32>,
            broadcaster: Option<Broadcaster>,
        }
        #[derive(Deserialize)]
        struct Broadcaster {
            login: String,
        }

        let body = serde_json::json!({
            "query": DIRECTORY_QUERY,
            "variables": { "name": game, "limit": limit },
        });

        let response: Response = self
            .http
            .post(GQL_ENDPOINT)
            .header("Client-ID", CLIENT_ID)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let directory = response
            .data
            .and_then(|d| d.game)
            .ok_or_else(|| Error::Malformed(format!("no directory for game `{game}`")))?;

        Ok(directory
            .streams
            .map(|s| s.edges)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                Some(LiveChannel {
                    login: edge.node.broadcaster?.login,
                    viewers: edge.node.viewers_count.unwrap_or(0),
                })
            })
            .collect())
    }
}
