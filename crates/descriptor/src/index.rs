//! The app index: a plain list of GitHub repository URLs.
//!
//! ```json
//! { "schema": 1, "apps": ["https://github.com/owner/name", "owner/other",
//!                         {"repo": "owner/shared", "app": "examplecraft"}] }
//! ```
//!
//! An entry with an `app` (or written `owner/shared#examplecraft`) is one app in a shared release
//! repo; see [`crate::source`]. An object entry may also name a `category` (see
//! [`crate::category`]); launchers before 0.12 ignore it.
//!
//! Every listed repo is checked exactly like one a user adds by hand: its latest release's
//! `craft-app.json`, checksums and release contract. A repo that passes appears in the launcher on
//! its own (not installed); one that doesn't (no descriptor yet, a broken release) is skipped
//! quietly until it does. Being listed vouches for nothing.

use serde::{Deserialize, Serialize};

use crate::descriptor::valid_repo;

/// The index file's name in the launcher's repo and on its `main` branch.
pub const INDEX_FILE: &str = "index/apps.json";
/// An index larger than this is refused unread.
pub const MAX_INDEX_BYTES: usize = 64 * 1024;
const MAX_ENTRIES: usize = 200;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub schema: u32,
    pub apps: Vec<IndexEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// The source key: GitHub `owner/repo`, or `owner/repo#app` for one app of a shared repo.
    pub repo: String,
    /// A name for logs and messages before the descriptor is read: the app id or the repo's name.
    pub name: String,
    /// The heading the maintainer files it under. `None`: none, or a key this launcher doesn't know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<crate::Category>,
}

impl IndexEntry {
    pub fn new(repo: String) -> Self {
        let name = repo.rsplit(['/', '#']).next().unwrap_or_default().to_string();
        Self { repo, name, category: None }
    }
}

/// `owner/name` from `owner/name` or a `https://github.com/owner/name[/…][.git]` link.
pub fn repo_from_url(input: &str) -> Option<String> {
    let s = input.trim().trim_end_matches('/');
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    let s = s.strip_prefix("www.").unwrap_or(s);
    let s = s.strip_prefix("github.com/").unwrap_or(s);
    let mut parts = s.split('/');
    let repo = format!("{}/{}", parts.next()?, parts.next()?.trim_end_matches(".git"));
    valid_repo(&repo).then_some(repo)
}

/// A source key from what a user or the index wrote: a repo (as [`repo_from_url`] reads it), or
/// `owner/name#app` (also `https://github.com/owner/name#app`) for one app of a shared repo.
pub fn source_from_input(input: &str) -> Option<String> {
    let (repo, app) = match input.trim().split_once('#') {
        Some((repo, app)) => (repo, Some(app)),
        None => (input, None),
    };
    let key = match app {
        Some(app) => format!("{}#{app}", repo_from_url(repo)?),
        None => repo_from_url(repo)?,
    };
    crate::source::Source::parse(&key).is_some().then_some(key)
}

impl Index {
    /// Reads an index leniently: a bad entry is dropped, never the whole list. Entries are URL,
    /// `owner/name` or `owner/name#app` strings, or `{"repo": …, "app": …}` objects; duplicates and
    /// official repos are dropped.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(format!("the index is {} bytes; the limit is {MAX_INDEX_BYTES}", bytes.len()));
        }
        let raw: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| format!("the index isn't JSON: {e}"))?;
        let schema = raw.get("schema").and_then(serde_json::Value::as_u64).unwrap_or(0);
        if schema != u64::from(crate::SCHEMA) {
            return Err(format!("index schema {schema} isn't supported"));
        }
        let mut apps: Vec<IndexEntry> = Vec::new();
        for e in raw.get("apps").and_then(serde_json::Value::as_array).into_iter().flatten() {
            let text = match (e.as_str(), e.get("repo").and_then(serde_json::Value::as_str)) {
                (Some(s), _) => Some(s.to_string()),
                (None, Some(repo)) => match e.get("app") {
                    None => Some(repo.to_string()),
                    Some(app) => app.as_str().map(|app| format!("{repo}#{app}")),
                },
                (None, None) => None,
            };
            let Some(repo) = text.as_deref().and_then(source_from_input) else { continue };
            if crate::reserved::repo(crate::source::repo_of(&repo)) || apps.iter().any(|a| a.repo.eq_ignore_ascii_case(&repo)) {
                continue;
            }
            let category = e.get("category").and_then(serde_json::Value::as_str).and_then(crate::Category::from_key);
            apps.push(IndexEntry { category, ..IndexEntry::new(repo) });
            if apps.len() == MAX_ENTRIES {
                break;
            }
        }
        Ok(Self { schema: crate::SCHEMA, apps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_repo_names_are_read_and_bad_entries_drop_out() {
        let json = br#"{"schema":1,"apps":[
            "https://github.com/AlienTechSoftware/notecraft",
            "https://github.com/UCS-Care-Group/mailcraft/releases",
            "github.com/someone/thing.git",
            {"repo":"other/legacy","name":"Legacy"},
            "alientechsoftware/NoteCraft",
            "https://github.com/storytold/photocraft",
            "../etc",
            42
        ]}"#;
        let i = Index::parse(json).unwrap();
        let repos: Vec<&str> = i.apps.iter().map(|a| a.repo.as_str()).collect();
        assert_eq!(repos, ["AlienTechSoftware/notecraft", "UCS-Care-Group/mailcraft", "someone/thing", "other/legacy"]);
        assert_eq!(i.apps[1].name, "mailcraft");
        assert!(Index::parse(br#"{"schema":9,"apps":[]}"#).is_err());
    }

    #[test]
    fn links_become_repo_names() {
        assert_eq!(repo_from_url(" https://github.com/a/b.git ").as_deref(), Some("a/b"));
        assert_eq!(repo_from_url("github.com/a/b/releases").as_deref(), Some("a/b"));
        assert_eq!(repo_from_url("a/b").as_deref(), Some("a/b"));
        assert_eq!(repo_from_url("nonsense"), None);
    }

    #[test]
    fn apps_of_a_shared_repo_are_separate_entries() {
        let json = br#"{"schema":1,"apps":[
            {"repo":"YalKouassi/craft-bridges","app":"mailcraft"},
            {"repo":"https://github.com/YalKouassi/craft-bridges","app":"notecraft"},
            "YalKouassi/craft-bridges#procraft",
            "https://github.com/YalKouassi/craft-bridges#transit",
            {"repo":"YalKouassi/craft-bridges","app":"mailcraft"},
            {"repo":"YalKouassi/craft-bridges","app":"Bad App"},
            {"repo":"YalKouassi/craft-bridges","app":7},
            {"repo":"storytold/apps","app":"photocraft"},
            "YalKouassi/craft-bridges"
        ]}"#;
        let i = Index::parse(json).unwrap();
        let repos: Vec<&str> = i.apps.iter().map(|a| a.repo.as_str()).collect();
        assert_eq!(
            repos,
            [
                "YalKouassi/craft-bridges#mailcraft",
                "YalKouassi/craft-bridges#notecraft",
                "YalKouassi/craft-bridges#procraft",
                "YalKouassi/craft-bridges#transit",
                "YalKouassi/craft-bridges"
            ]
        );
        assert_eq!(i.apps[0].name, "mailcraft");
        assert_eq!(source_from_input("github.com/o/shared#x-craft").as_deref(), Some("o/shared#x-craft"));
        assert_eq!(source_from_input("o/shared#"), None);
    }
    #[test]
    fn categories_are_read_and_unknown_ones_dropped() {
        let json = br#"{"schema":1,"apps":[
            {"repo":"o/shared","app":"a","category":"audio-video"},
            {"repo":"o/shared","app":"b","category":"games"},
            {"repo":"o/shared","app":"c","category":7},
            {"repo":"o/other","category":"Maps"},
            "o/plain"
        ]}"#;
        let i = Index::parse(json).unwrap();
        let cats: Vec<Option<crate::Category>> = i.apps.iter().map(|a| a.category).collect();
        assert_eq!(cats, [Some(crate::Category::AudioVideo), None, None, Some(crate::Category::Maps), None]);
    }

    #[test]
    fn the_shipped_index_parses_with_a_known_category_on_every_object_entry() {
        let bytes = include_bytes!("../../../index/apps.json");
        let raw: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let objects = raw["apps"].as_array().unwrap().iter().filter(|e| e.is_object()).count();
        let i = Index::parse(bytes).unwrap();
        assert_eq!(i.apps.len(), raw["apps"].as_array().unwrap().len(), "every entry in index/apps.json is read");
        for e in raw["apps"].as_array().unwrap().iter().filter_map(|e| e.get("category")) {
            assert!(e.as_str().and_then(crate::Category::from_key).is_some(), "unknown category {e} in index/apps.json");
        }
        assert!(objects > 0);
    }
}
