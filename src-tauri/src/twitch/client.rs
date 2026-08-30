//! Twitch's web-player endpoints. Undocumented and subject to change:
//! `CLIENT_ID` and `PLAYBACK_TOKEN_QUERY_HASH` are what break first.

use serde::Deserialize;

use super::error::{Error, Result};
use super::playlist::MasterPlaylist;

/// Public client id used by Twitch's own web player.
pub const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

/// Persisted-query hash for the `PlaybackAccessToken` operation.
pub const PLAYBACK_TOKEN_QUERY_HASH: &str =
    "0828119ded1c13477966434e15800ff57ddacf13ba1911c129dc2200705b0712";

const GQL_ENDPOINT: &str = "https://gql.twitch.tv/gql";
const USHER_ENDPOINT: &str = "https://usher.ttvnw.net/api/channel/hls";

const DIRECTORY_QUERY: &str = "query($name: String!, $limit: Int!) { \
     game(name: $name) { streams(first: $limit) { \
     edges { node { viewersCount broadcaster { login } } } } } }";

/// Signed token authorising playlist access for one channel.
#[derive(Debug, Clone, Deserialize)]
pub struct PlaybackToken {
    pub value: String,
    pub signature: String,
}

/// A channel live in a game directory.
#[derive(Debug, Clone)]
pub struct LiveChannel {
    pub login: String,
    pub viewers: u32,
}

#[derive(Debug, Clone)]
pub struct TwitchClient {
    http: reqwest::Client,
}

impl TwitchClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/125.0 Safari/537.36",
            )
            .build()?;
        Ok(Self { http })
    }

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
                "persistedQuery": { "version": 1, "sha256Hash": PLAYBACK_TOKEN_QUERY_HASH }
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

    /// `low_latency` sets `fast_bread`, Streamlink's `--twitch-low-latency`.
    pub async fn master_playlist(
        &self,
        channel: &str,
        low_latency: bool,
    ) -> Result<MasterPlaylist> {
        let token = self.playback_token(channel).await?;

        let response = self
            .http
            .get(format!("{USHER_ENDPOINT}/{channel}.m3u8"))
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

        // 404 covers both offline and nonexistent.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Offline(channel.to_owned()));
        }

        let playlist = MasterPlaylist::parse(&response.error_for_status()?.text().await?);
        if playlist.variants.is_empty() {
            return Err(Error::NoVariants(channel.to_owned()));
        }
        Ok(playlist)
    }

    /// Busiest first. One request covers the whole directory, so this is a much
    /// cheaper liveness probe than `master_playlist` per channel.
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
