//! The `craft-app.json` schema (version 1) and its checks.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{MAX_DESCRIPTOR_BYTES, SCHEMA, reserved};

/// An unofficial app's description of itself, once checked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// Always [`SCHEMA`] for now.
    pub schema: u32,
    /// Machine name: binary, package and asset stem (`notecraft`). `^[a-z0-9]+(-[a-z0-9]+)*$`.
    pub id: String,
    /// Display name (`NoteCraft`).
    pub name: String,
    /// What it's for, in a few words.
    pub tagline: String,
    /// GitHub `owner/repo` its releases are published under. Must be the repo it was read from.
    pub repo: String,
    /// Who makes it, as the card's byline shows it.
    pub publisher: String,
    /// The project this one packages, when it's a bridge repo (`owner/repo`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    /// Reverse-DNS id: macOS bundle id, Flatpak id and Linux desktop-file id.
    pub bundle_id: String,
    /// The Publisher its MSI registers. Required when a release ships an `.msi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msi_publisher: Option<String>,
    /// The Apple Developer ID team that signs its `.app`. Without one the launcher never installs
    /// its DMG, it only opens a copy that is already there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apple_team: Option<String>,
    /// Its website (`https://` only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// Brand colour, `#rrggbb`.
    pub color: String,
    /// A PNG release asset (at most 512 px on each side, 256 KB; square is best) used as its icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Older asset stems its releases may carry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub former_ids: Vec<String>,
    /// Older display names its installers registered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub former_names: Vec<String>,
    /// Older bundle ids it shipped under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub former_bundles: Vec<String>,
}

/// Every reason a descriptor was refused, so a maintainer can fix them all in one go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Problems(pub Vec<String>);

impl fmt::Display for Problems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("; "))
    }
}

impl std::error::Error for Problems {}

const MAX_LIST: usize = 8;

impl Descriptor {
    /// Reads and checks a descriptor. `repo` is the GitHub `owner/repo` it was fetched from (or
    /// the repo the app is built in); the descriptor must name that same repo.
    pub fn parse(bytes: &[u8], repo: &str) -> Result<Self, Problems> {
        if bytes.len() > MAX_DESCRIPTOR_BYTES {
            return Err(Problems(vec![format!("{} is {} bytes; the limit is {MAX_DESCRIPTOR_BYTES}", crate::DESCRIPTOR_FILE, bytes.len())]));
        }
        let d: Descriptor = serde_json::from_slice(bytes).map_err(|e| Problems(vec![format!("{} isn't valid: {e}", crate::DESCRIPTOR_FILE)]))?;
        d.check(repo)?;
        Ok(d)
    }

    /// Runs every check [`Descriptor::parse`] runs.
    pub fn check(&self, repo: &str) -> Result<(), Problems> {
        let mut p = Vec::new();
        let mut bad = |ok: bool, msg: String| {
            if !ok {
                p.push(msg);
            }
        };
        bad(self.schema == SCHEMA, format!("schema {} isn't supported (this launcher reads schema {SCHEMA})", self.schema));
        bad(valid_id(&self.id), format!("id {:?} must be 1–40 lowercase letters, digits and single hyphens", self.id));
        bad(!reserved::id_for_repo(&self.id, &self.repo), format!("id {:?} belongs to an official app", self.id));
        bad(valid_name(&self.name), format!("name {:?} must be 1–40 characters of plain text, usable as a folder name (no / \\ : * ? \" < > |, not starting or ending with a dot)", self.name));
        bad(!reserved::id(&self.name), format!("name {:?} belongs to an official app", self.name));
        bad(valid_text(&self.tagline, 80), "tagline must be 1–80 characters of plain text".into());
        bad(valid_repo(&self.repo), format!("repo {:?} must be a GitHub owner/name", self.repo));
        bad(self.repo.eq_ignore_ascii_case(repo), format!("repo {:?} doesn't match the repository it came from ({repo})", self.repo));
        bad(!reserved::repo(&self.repo), "an unofficial app can't live under an official GitHub account".into());
        bad(valid_text(&self.publisher, 60), "publisher must be 1–60 characters of plain text".into());
        bad(!reserved::publisher(&self.publisher), format!("publisher {:?} belongs to ArtCraft", self.publisher));
        if let Some(up) = &self.upstream {
            bad(valid_repo(up), format!("upstream {up:?} must be a GitHub owner/name"));
        }
        bad(valid_bundle(&self.bundle_id), format!("bundle_id {:?} must be reverse-DNS, like org.example.app", self.bundle_id));
        bad(!reserved::bundle(&self.bundle_id), format!("bundle_id {:?} is in ArtCraft's namespace", self.bundle_id));
        if let Some(m) = &self.msi_publisher {
            bad(valid_text(m, 100), "msi_publisher must be 1–100 characters of plain text".into());
            bad(!reserved::msi_publisher(m), format!("msi_publisher {m:?} belongs to ArtCraft"));
        }
        if let Some(t) = &self.apple_team {
            bad(
                t.len() == 10 && t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
                format!("apple_team {t:?} must be a 10-character Team ID"),
            );
            bad(!reserved::apple_team(t), "apple_team is ArtCraft's Developer ID".into());
        }
        if let Some(w) = &self.website {
            bad(valid_https(w), format!("website {w:?} must be an https:// link"));
        }
        bad(parse_color(&self.color).is_some(), format!("color {:?} must be #rrggbb", self.color));
        if let Some(i) = &self.icon {
            bad(
                crate::valid_file_name(i) && i.to_ascii_lowercase().ends_with(".png"),
                format!("icon {i:?} must be the file name of a .png release asset"),
            );
        }
        bad(
            self.former_ids.len() <= MAX_LIST && self.former_names.len() <= MAX_LIST && self.former_bundles.len() <= MAX_LIST,
            format!("at most {MAX_LIST} former ids, names and bundles each"),
        );
        for f in &self.former_ids {
            bad(valid_id(f) && !reserved::id(f), format!("former id {f:?} isn't a valid unofficial id"));
        }
        for f in &self.former_names {
            bad(valid_name(f) && !reserved::id(f), format!("former name {f:?} isn't a valid unofficial name"));
        }
        for f in &self.former_bundles {
            bad(valid_bundle(f) && !reserved::bundle(f), format!("former bundle {f:?} isn't a valid unofficial bundle id"));
        }
        if p.is_empty() { Ok(()) } else { Err(Problems(p)) }
    }

    /// The brand colour as sRGB.
    pub fn rgb(&self) -> [u8; 3] {
        parse_color(&self.color).unwrap_or([0x80, 0x80, 0x80])
    }

    /// Asset stems this app's releases may use, current first.
    pub fn stems(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.former_ids.iter().map(String::as_str))
    }

    /// Serialises the descriptor as a maintainer would commit it (pretty JSON, trailing newline).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).map(|s| s + "\n").unwrap_or_default()
    }
}

pub(crate) fn valid_id(id: &str) -> bool {
    (1..=40).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}

/// A display name. The launcher joins it into install paths (Windows looks for `<Program Files>\<name>`),
/// so it must be a single, ordinary folder name too.
fn valid_name(s: &str) -> bool {
    valid_text(s, 40)
        && !s.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        && !s.starts_with('.')
        && !s.ends_with('.')
}

fn valid_text(s: &str, max_chars: usize) -> bool {
    let n = s.chars().count();
    (1..=max_chars).contains(&n) && s.trim() == s && !s.chars().any(|c| c.is_control() || is_bidi_control(c))
}

/// Right-to-left overrides and friends let a name read differently from what it is.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// `owner/name` with GitHub's characters.
pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else { return false };
    let owner_ok = (1..=39).contains(&owner.len()) && owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') && !owner.starts_with('-');
    let name_ok = (1..=100).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        && name != "."
        && name != ".."
        && !name.ends_with(".git");
    owner_ok && name_ok
}

fn valid_bundle(b: &str) -> bool {
    let parts: Vec<&str> = b.split('.').collect();
    b.len() <= 255
        && parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty() && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') && !p.starts_with(|c: char| c.is_ascii_digit())
        })
}

fn valid_https(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    url.len() <= 200
        && !host.is_empty()
        && !host.contains(['@', '\\'])
        && !url.chars().any(|c| c.is_control() || c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`' | '|' | '^' | '{' | '}'))
}

fn parse_color(c: &str) -> Option<[u8; 3]> {
    let hex = c.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| hex.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const EXAMPLE: &str = r##"{
        "schema": 1,
        "id": "examplecraft",
        "name": "ExampleCraft",
        "tagline": "What it's for, in a few words",
        "repo": "example-org/examplecraft",
        "publisher": "Example Org",
        "upstream": null,
        "bundle_id": "org.example.examplecraft",
        "msi_publisher": "Example Org",
        "apple_team": null,
        "website": "https://example.org/examplecraft",
        "color": "#7a45b8",
        "icon": "examplecraft-128.png"
    }"##;

    fn with(field: &str, value: serde_json::Value) -> Vec<u8> {
        let mut v: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        v[field] = value;
        serde_json::to_vec(&v).unwrap()
    }

    #[test]
    fn the_proposals_example_reads() {
        let d = Descriptor::parse(EXAMPLE.as_bytes(), "example-org/examplecraft").unwrap();
        assert_eq!(d.id, "examplecraft");
        assert_eq!(d.rgb(), [0x7a, 0x45, 0xb8]);
        assert_eq!(Descriptor::parse(d.to_json().as_bytes(), "Example-Org/ExampleCraft").unwrap(), d);
    }

    #[test]
    fn the_repo_must_be_the_one_it_came_from() {
        let e = Descriptor::parse(EXAMPLE.as_bytes(), "someone-else/examplecraft").unwrap_err();
        assert!(e.to_string().contains("doesn't match"), "{e}");
    }

    #[test]
    fn artcraft_namespace_is_refused() {
        let repo = "example-org/examplecraft";
        for (field, value) in [
            ("id", "photocraft"),
            ("name", "WordCraft"),
            ("bundle_id", "ai.storyteller.examplecraft"),
            ("msi_publisher", "Learning Machines LLC"),
            ("apple_team", "DJ6XS33FX8"),
            ("publisher", "ArtCraft"),
        ] {
            assert!(Descriptor::parse(&with(field, value.into()), repo).is_err(), "{field} = {value} was accepted");
        }
        let official = with("repo", "storytold/examplecraft".into());
        assert!(Descriptor::parse(&official, "storytold/examplecraft").is_err());
    }

    #[test]
    fn hostile_fields_are_refused() {
        let repo = "example-org/examplecraft";
        for (field, value) in [
            ("schema", serde_json::json!(2)),
            ("id", "Example Craft".into()),
            ("id", "-x".into()),
            ("id", "a".repeat(41).into()),
            ("name", "Example\u{202e}tfarC".into()),
            ("name", "".into()),
            ("name", "..".into()),
            ("name", r"..\..\Windows".into()),
            ("name", "Note/Craft".into()),
            ("tagline", "x".repeat(81).into()),
            ("website", "http://example.org".into()),
            ("website", "https://example.org/a|b".into()),
            ("website", "javascript:alert(1)".into()),
            ("color", "red".into()),
            ("icon", "../icon.png".into()),
            ("icon", "icon.svg".into()),
            ("bundle_id", "examplecraft".into()),
            ("apple_team", "short".into()),
            ("upstream", "not a repo".into()),
        ] {
            assert!(Descriptor::parse(&with(field, value.clone()), repo).is_err(), "{field} = {value} was accepted");
        }
        assert!(Descriptor::parse(&with("surprise", true.into()), repo).is_err(), "unknown fields are refused");
        assert!(Descriptor::parse(&vec![b' '; crate::MAX_DESCRIPTOR_BYTES + 1], repo).is_err());
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let mut v: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        v["id"] = "photocraft".into();
        v["color"] = "blue".into();
        let e = Descriptor::parse(&serde_json::to_vec(&v).unwrap(), "example-org/examplecraft").unwrap_err();
        assert_eq!(e.0.len(), 2, "{e}");
    }

    #[test]
    fn repos_follow_github_rules() {
        assert!(valid_repo("tangency-labs/solidcraft"));
        assert!(valid_repo("a/b.c_d-e"));
        for bad in ["", "a", "a/b/c", "a/..", "-a/b", "a/b.git", "a b/c"] {
            assert!(!valid_repo(bad), "{bad}");
        }
    }
}
