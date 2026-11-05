//! What kind of app an index entry is, for grouping and filtering the launcher's list.
//!
//! A category is the index maintainer's, never the app's: it lives on an entry of
//! `index/apps.json` (`{"repo": …, "app": …, "category": "audio-video"}`), not in `craft-app.json`,
//! so app authors can't file themselves under a heading and older launchers, which read only
//! `repo` and `app`, ignore it. The set is fixed so the UI can translate every heading; an
//! unknown key reads as no category.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Category {
    #[serde(rename = "graphics")]
    Graphics,
    #[serde(rename = "audio-video")]
    AudioVideo,
    #[serde(rename = "3d")]
    ThreeD,
    #[serde(rename = "maps")]
    Maps,
    #[serde(rename = "productivity")]
    Productivity,
    #[serde(rename = "developer")]
    Developer,
    #[serde(rename = "utilities")]
    Utilities,
}

impl Category {
    /// Every category, in the order the launcher shows them.
    pub const ALL: [Self; 7] = [Self::Graphics, Self::AudioVideo, Self::ThreeD, Self::Maps, Self::Productivity, Self::Developer, Self::Utilities];

    /// The key the index writes.
    pub fn key(self) -> &'static str {
        match self {
            Self::Graphics => "graphics",
            Self::AudioVideo => "audio-video",
            Self::ThreeD => "3d",
            Self::Maps => "maps",
            Self::Productivity => "productivity",
            Self::Developer => "developer",
            Self::Utilities => "utilities",
        }
    }

    /// The category an index key names, if any (case-insensitive).
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key().eq_ignore_ascii_case(key.trim()))
    }

    /// The English heading; the UI's translation key.
    pub fn label(self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::AudioVideo => "Audio & video",
            Self::ThreeD => "3D & CAD",
            Self::Maps => "Maps & GIS",
            Self::Productivity => "Productivity",
            Self::Developer => "Developer tools",
            Self::Utilities => "Utilities",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_unknown_keys_are_none() {
        for c in Category::ALL {
            assert_eq!(Category::from_key(c.key()), Some(c));
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(json, format!("\"{}\"", c.key()));
            assert_eq!(serde_json::from_str::<Category>(&json).unwrap(), c);
        }
        assert_eq!(Category::from_key(" Audio-Video "), Some(Category::AudioVideo));
        assert_eq!(Category::from_key("games"), None);
        assert_eq!(Category::from_key(""), None);
    }
}
