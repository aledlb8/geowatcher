//! Everything about how the corner widget looks and where it sits.
//!
//! Kept separate from watcher settings because it changes constantly while
//! someone is styling the thing, and none of it should restart a watcher.

use serde::{Deserialize, Serialize};

/// Which screen corner the widget docks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    #[must_use]
    pub fn is_right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    #[must_use]
    pub fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }
}

/// The progress ramp. Percentage is encoded in hue as well as length, so how
/// close a run is reads from across the room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ramp {
    pub low: String,
    pub mid: String,
    pub high: String,
    pub peak: String,
}

impl Default for Ramp {
    fn default() -> Self {
        Self {
            low: "#89B4FA".to_owned(),
            mid: "#A6E3A1".to_owned(),
            high: "#FAB387".to_owned(),
            peak: "#F38BA8".to_owned(),
        }
    }
}

/// Widget appearance and placement.
///
/// The toggles are flat on purpose: this is the JSON people hand-edit, and
/// nesting them into sub-structs to satisfy a lint would only make it worse.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    pub corner: Corner,
    /// Gap from the screen edge, in logical pixels.
    pub margin_x: u32,
    pub margin_y: u32,
    pub width: u32,

    pub opacity: f64,
    pub radius: u32,
    pub padding: u32,
    /// Space between channel rows.
    pub gap: u32,

    pub font_family: String,
    pub numeral_family: String,
    pub font_size: u32,

    pub background: String,
    pub panel: String,
    pub line: String,
    pub text: String,
    pub dim: String,
    pub ramp: Ramp,

    pub show_bar: bool,
    pub bar_height: u32,
    /// One tick per attempt, height by peak reached.
    pub show_history: bool,
    pub history_length: u32,
    pub history_height: u32,
    /// List offline channels rather than hiding them.
    pub show_offline: bool,

    /// Shrink to a pill when nothing is running.
    pub collapse_when_idle: bool,
    /// Ignore the mouse until it hovers the widget.
    pub click_through: bool,
    pub always_on_top: bool,
    /// Show the widget at all.
    pub enabled: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            corner: Corner::TopRight,
            margin_x: 20,
            margin_y: 20,
            width: 320,

            opacity: 0.9,
            radius: 16,
            padding: 14,
            gap: 12,

            font_family: "Archivo".to_owned(),
            numeral_family: "Martian Mono".to_owned(),
            font_size: 12,

            background: "#1E1E2E".to_owned(),
            panel: "#181825".to_owned(),
            line: "#313244".to_owned(),
            text: "#CDD6F4".to_owned(),
            dim: "#6C7086".to_owned(),
            ramp: Ramp::default(),

            show_bar: true,
            bar_height: 5,
            show_history: true,
            history_length: 32,
            history_height: 18,
            show_offline: false,

            collapse_when_idle: true,
            click_through: true,
            always_on_top: true,
            enabled: true,
        }
    }
}

impl Appearance {
    /// Clamps anything a hand-edited settings file could put out of range.
    pub fn sanitise(&mut self) {
        self.width = self.width.clamp(160, 900);
        self.opacity = self.opacity.clamp(0.15, 1.0);
        self.radius = self.radius.min(48);
        self.padding = self.padding.clamp(0, 64);
        self.gap = self.gap.min(48);
        self.font_size = self.font_size.clamp(8, 32);
        self.bar_height = self.bar_height.clamp(1, 24);
        self.history_length = self.history_length.clamp(4, 120);
        self.history_height = self.history_height.clamp(4, 64);
        self.margin_x = self.margin_x.min(600);
        self.margin_y = self.margin_y.min(600);
    }

    /// Widget height estimate, used to place it against a bottom corner.
    #[must_use]
    pub fn estimated_height(&self, rows: u32) -> u32 {
        if rows == 0 {
            return self.padding * 2 + 22;
        }
        let row =
            20 + if self.show_bar {
                self.bar_height + 6
            } else {
                0
            } + if self.show_history {
                self.history_height + 6
            } else {
                0
            };
        self.padding * 2 + rows * row + rows.saturating_sub(1) * self.gap
    }
}

#[cfg(test)]
mod tests {
    use super::{Appearance, Corner};

    #[test]
    fn corners_know_their_edges() {
        assert!(Corner::TopRight.is_right());
        assert!(!Corner::TopRight.is_bottom());
        assert!(Corner::BottomLeft.is_bottom());
        assert!(!Corner::BottomLeft.is_right());
    }

    #[test]
    fn sanitise_pulls_absurd_values_back() {
        let mut a = Appearance {
            width: 5000,
            opacity: 4.0,
            font_size: 900,
            history_length: 0,
            ..Appearance::default()
        };
        a.sanitise();
        assert_eq!(a.width, 900);
        assert!((a.opacity - 1.0).abs() < f64::EPSILON);
        assert_eq!(a.font_size, 32);
        assert_eq!(a.history_length, 4);
    }

    #[test]
    fn sanitise_leaves_sensible_values_alone() {
        let original = Appearance::default();
        let mut copy = original.clone();
        copy.sanitise();
        assert_eq!(original, copy);
    }

    #[test]
    fn height_grows_with_rows() {
        let a = Appearance::default();
        assert!(a.estimated_height(3) > a.estimated_height(1));
        assert!(a.estimated_height(1) > a.estimated_height(0));
    }

    #[test]
    fn hiding_parts_shrinks_the_estimate() {
        let full = Appearance::default();
        let bare = Appearance {
            show_bar: false,
            show_history: false,
            ..Appearance::default()
        };
        assert!(bare.estimated_height(3) < full.estimated_height(3));
    }

    #[test]
    fn appearance_round_trips_as_camel_case() {
        let json = serde_json::to_string(&Appearance::default()).expect("json");
        assert!(json.contains("clickThrough"), "{json}");
        assert!(json.contains("\"corner\":\"topRight\""), "{json}");

        let back: Appearance = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, Appearance::default());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // A settings file written by an older build must still load.
        let back: Appearance = serde_json::from_str("{}").expect("parse");
        assert_eq!(back, Appearance::default());
    }
}
