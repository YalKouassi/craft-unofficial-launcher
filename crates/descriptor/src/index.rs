//! The app index: a plain list of GitHub repository URLs.
//!
//! ```json
//! { "schema": 1, "apps": ["https://github.com/owner/name", "owner/other"] }
//! ```
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
    /// GitHub `owner/repo`.
    pub repo: String,
    /// A name for logs and messages before the descriptor is read: the repo's name.
    pub name: String,
}

impl IndexEntry {
    fn new(repo: String) -> Self {
        let name = repo.rsplit('/').next().unwrap_or_default().to_string();
        Self { repo, name }
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

impl Index {
    /// Reads an index leniently: a bad entry is dropped, never the whole list. Entries are URL or
    /// `owner/name` strings (`{"repo": …}` objects are read too); duplicates and official repos are
    /// dropped.
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
            let text = e.as_str().or_else(|| e.get("repo").and_then(serde_json::Value::as_str));
            let Some(repo) = text.and_then(repo_from_url) else { continue };
            if crate::reserved::repo(&repo) || apps.iter().any(|a| a.repo.eq_ignore_ascii_case(&repo)) {
                continue;
            }
            apps.push(IndexEntry::new(repo));
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
}
