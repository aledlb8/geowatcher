//! Twitch HLS master playlist parsing. Each `#EXT-X-STREAM-INF` line is
//! followed by its variant URL.

/// Tall enough for the percentage glyphs, cheap enough to decode continuously.
const PREFERRED_HEIGHT: u32 = 480;

/// Below this the percentage stops being legible.
const MIN_USABLE_HEIGHT: u32 = 360;

/// A quality option, e.g. `480p30` or `chunked` for source.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub url: String,
    /// Absent on audio-only variants.
    pub resolution: Option<(u32, u32)>,
    pub frame_rate: Option<f32>,
}

impl Variant {
    #[must_use]
    pub fn height(&self) -> Option<u32> {
        self.resolution.map(|(_, h)| h)
    }
}

#[derive(Debug, Clone)]
pub struct MasterPlaylist {
    /// In the order Twitch listed them.
    pub variants: Vec<Variant>,
}

impl MasterPlaylist {
    #[must_use]
    pub fn parse(body: &str) -> Self {
        let mut variants = Vec::new();
        let mut lines = body.lines().peekable();

        while let Some(line) = lines.next() {
            let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") else {
                continue;
            };

            // The URL is the next line that is neither blank nor a comment.
            let url = loop {
                match lines.peek() {
                    Some(next) if next.trim().is_empty() || next.starts_with('#') => {
                        lines.next();
                    }
                    Some(next) => break Some((*next).trim().to_owned()),
                    None => break None,
                }
            };
            let Some(url) = url else { continue };
            lines.next();

            variants.push(Variant {
                name: attribute(attrs, "VIDEO").unwrap_or_else(|| "unknown".to_owned()),
                url,
                resolution: attribute(attrs, "RESOLUTION").and_then(|r| parse_resolution(&r)),
                frame_rate: attribute(attrs, "FRAME-RATE").and_then(|f| f.parse().ok()),
            });
        }

        Self { variants }
    }

    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.name == name)
    }

    /// Cheapest variant that still renders the percentage legibly, falling back
    /// to the tallest available, then to anything at all.
    #[must_use]
    pub fn best_for_analysis(&self) -> Option<&Variant> {
        self.variants
            .iter()
            .filter(|v| v.height().is_some_and(|h| h >= MIN_USABLE_HEIGHT))
            .min_by_key(|v| v.height().unwrap_or_default().abs_diff(PREFERRED_HEIGHT))
            .or_else(|| {
                self.variants
                    .iter()
                    .max_by_key(|v| v.height().unwrap_or_default())
            })
            .or_else(|| self.variants.first())
    }
}

/// Reads one `KEY=value` or `KEY="value"` attribute.
fn attribute(attrs: &str, key: &str) -> Option<String> {
    split_attributes(attrs).into_iter().find_map(|pair| {
        let (found, value) = pair.split_once('=')?;
        if found.trim() != key {
            return None;
        }
        let value = value.trim();
        Some(
            value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value)
                .to_owned(),
        )
    })
}

/// Splits on commas outside quotes; `CODECS="a,b"` contains one.
fn split_attributes(attrs: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;

    for (index, ch) in attrs.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(&attrs[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&attrs[start..]);
    parts
}

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::{attribute, MasterPlaylist};

    /// Trimmed from a real usher response, with the signed URLs replaced.
    const REAL_SHAPE: &str = concat!(
        "#EXTM3U\n",
        "#EXT-X-TWITCH-INFO:NODE=\"x.hls.ttvnw.net\",SUPPRESS=\"true\"\n",
        "#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID=\"chunked\",NAME=\"1080p60 (source)\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=6891597,RESOLUTION=1920x1080,",
        "CODECS=\"avc1.64002A,mp4a.40.2\",VIDEO=\"chunked\",FRAME-RATE=60.000\n",
        "https://example.invalid/chunked.m3u8\n",
        "#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID=\"720p60\",NAME=\"720p60\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=3422999,RESOLUTION=1280x720,",
        "CODECS=\"avc1.4D401F,mp4a.40.2\",VIDEO=\"720p60\",FRAME-RATE=60.000\n",
        "https://example.invalid/720p60.m3u8\n",
        "#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID=\"480p30\",NAME=\"480p\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=1427999,RESOLUTION=852x480,",
        "CODECS=\"avc1.4D401F,mp4a.40.2\",VIDEO=\"480p30\",FRAME-RATE=30.000\n",
        "https://example.invalid/480p30.m3u8\n",
        "#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID=\"360p30\",NAME=\"360p\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=627999,RESOLUTION=640x360,",
        "CODECS=\"avc1.4D401E,mp4a.40.2\",VIDEO=\"360p30\",FRAME-RATE=30.000\n",
        "https://example.invalid/360p30.m3u8\n",
        "#EXT-X-MEDIA:TYPE=VIDEO,GROUP-ID=\"audio_only\",NAME=\"Audio Only\"\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=128000,CODECS=\"mp4a.40.2\",VIDEO=\"audio_only\"\n",
        "https://example.invalid/audio.m3u8\n",
    );

    #[test]
    fn parses_every_variant_in_order() {
        let playlist = MasterPlaylist::parse(REAL_SHAPE);
        let names: Vec<_> = playlist.variants.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            ["chunked", "720p60", "480p30", "360p30", "audio_only"]
        );
    }

    #[test]
    fn quoted_codecs_comma_does_not_split_attributes() {
        // CODECS="avc1.4D401F,mp4a.40.2" would break a naive comma split and
        // take VIDEO and FRAME-RATE down with it.
        let playlist = MasterPlaylist::parse(REAL_SHAPE);
        let v = playlist.by_name("480p30").expect("480p30 variant");
        assert_eq!(v.resolution, Some((852, 480)));
        assert_eq!(v.frame_rate, Some(30.0));
        assert_eq!(v.url, "https://example.invalid/480p30.m3u8");
    }

    #[test]
    fn audio_only_variant_has_no_resolution() {
        let playlist = MasterPlaylist::parse(REAL_SHAPE);
        let v = playlist.by_name("audio_only").expect("audio_only variant");
        assert_eq!(v.resolution, None);
        assert_eq!(v.height(), None);
    }

    #[test]
    fn prefers_480p_for_analysis() {
        let playlist = MasterPlaylist::parse(REAL_SHAPE);
        assert_eq!(
            playlist.best_for_analysis().map(|v| v.name.as_str()),
            Some("480p30")
        );
    }

    #[test]
    fn falls_back_to_tallest_when_nothing_meets_the_floor() {
        let body = concat!(
            "#EXT-X-STREAM-INF:RESOLUTION=284x160,VIDEO=\"160p30\"\n",
            "https://example.invalid/160.m3u8\n",
            "#EXT-X-STREAM-INF:RESOLUTION=256x144,VIDEO=\"144p30\"\n",
            "https://example.invalid/144.m3u8\n",
        );
        let playlist = MasterPlaylist::parse(body);
        assert_eq!(
            playlist.best_for_analysis().map(|v| v.name.as_str()),
            Some("160p30")
        );
    }

    #[test]
    fn picks_720_when_480_is_absent() {
        let body = concat!(
            "#EXT-X-STREAM-INF:RESOLUTION=1920x1080,VIDEO=\"chunked\"\n",
            "https://example.invalid/src.m3u8\n",
            "#EXT-X-STREAM-INF:RESOLUTION=1280x720,VIDEO=\"720p60\"\n",
            "https://example.invalid/720.m3u8\n",
        );
        let playlist = MasterPlaylist::parse(body);
        assert_eq!(
            playlist.best_for_analysis().map(|v| v.name.as_str()),
            Some("720p60")
        );
    }

    #[test]
    fn empty_playlist_yields_nothing() {
        let playlist = MasterPlaylist::parse("#EXTM3U\n");
        assert!(playlist.variants.is_empty());
        assert!(playlist.best_for_analysis().is_none());
    }

    #[test]
    fn attribute_does_not_match_a_key_suffix() {
        let attrs = "FRAME-RATE=60.000,VIDEO=\"chunked\"";
        assert_eq!(attribute(attrs, "FRAME-RATE"), Some("60.000".to_owned()));
        assert_eq!(attribute(attrs, "RATE"), None);
    }
}
