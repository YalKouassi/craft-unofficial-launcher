//! Where an app's releases come from: its own GitHub repo, or one app among several in a shared
//! release repo.
//!
//! A source is written as a **key**: `owner/name` for an app that has the repo to itself, or
//! `owner/name#<app id>` for an app in a shared repo. The launcher uses the key wherever it used to
//! use the repo as an app's identity (its saved list, caches, the index); only the code that talks
//! to GitHub splits it back into the repo and the app.
//!
//! In a shared repo every release belongs to one app, named by its tag: `<app id>-v<version>`
//! (`mailcraft-v0.10.0`). A release whose tag doesn't start with the app's id belongs to another
//! app and is never read as this one's.

use crate::descriptor::{valid_id, valid_repo};

/// An app's release source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source<'a> {
    /// GitHub `owner/name`.
    pub repo: &'a str,
    /// The app's id when the repo is shared.
    pub app: Option<&'a str>,
}

impl<'a> Source<'a> {
    /// Splits a key. `None` when the repo or the app id isn't valid.
    pub fn parse(key: &'a str) -> Option<Self> {
        let (repo, app) = match key.split_once('#') {
            Some((repo, app)) => (repo, Some(app)),
            None => (key, None),
        };
        (valid_repo(repo) && app.is_none_or(valid_id)).then_some(Self { repo, app })
    }

    /// The key: `owner/name` or `owner/name#app`.
    pub fn key(&self) -> String {
        match self.app {
            Some(app) => format!("{}#{app}", self.repo),
            None => self.repo.to_string(),
        }
    }

    /// The part of `tag` that names the version (`v1.2.3`), or `None` when the release belongs to
    /// another app of a shared repo. Every tag is this app's when the repo isn't shared.
    pub fn version_tag<'t>(&self, tag: &'t str) -> Option<&'t str> {
        let Some(app) = self.app else { return Some(tag) };
        let rest = tag.strip_prefix(app)?.strip_prefix('-')?;
        // `picasso-suite-v1.0.0` isn't a release of an app called `picasso`.
        let digits = rest.strip_prefix(['v', 'V']).unwrap_or(rest);
        digits.starts_with(|c: char| c.is_ascii_digit()).then_some(rest)
    }

    /// The tag this app's release `version` gets: `v1.2.3`, or `<app>-v1.2.3` in a shared repo.
    pub fn tag_for(&self, version: &str) -> String {
        match self.app {
            Some(app) => format!("{app}-v{version}"),
            None => format!("v{version}"),
        }
    }
}

/// The repo part of a key (the whole key when it names no app).
pub fn repo_of(key: &str) -> &str {
    key.split_once('#').map_or(key, |(repo, _)| repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        let own = Source::parse("org/examplecraft").unwrap();
        assert_eq!((own.repo, own.app), ("org/examplecraft", None));
        assert_eq!(own.key(), "org/examplecraft");
        let shared = Source::parse("org/bridges#mailcraft").unwrap();
        assert_eq!((shared.repo, shared.app), ("org/bridges", Some("mailcraft")));
        assert_eq!(shared.key(), "org/bridges#mailcraft");
        assert_eq!(repo_of("org/bridges#mailcraft"), "org/bridges");
        assert_eq!(repo_of("org/examplecraft"), "org/examplecraft");
    }

    #[test]
    fn bad_keys_are_refused() {
        for bad in ["", "org", "org/bridges#", "org/bridges#Mail", "org/bridges#a#b", "org/bridges#../x", "org/a/b#x", "#mailcraft"] {
            assert_eq!(Source::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_shared_repo_reads_only_its_own_tags() {
        let s = Source::parse("org/bridges#picasso").unwrap();
        assert_eq!(s.version_tag("picasso-v1.2.0"), Some("v1.2.0"));
        assert_eq!(s.version_tag("picasso-1.2.0"), Some("1.2.0"));
        assert_eq!(s.version_tag("picasso-suite-v1.2.0"), None, "another app whose id starts the same");
        assert_eq!(s.version_tag("mailcraft-v1.2.0"), None);
        assert_eq!(s.version_tag("v1.2.0"), None);
        assert_eq!(s.version_tag("picasso-"), None);
        assert_eq!(s.tag_for("1.2.0"), "picasso-v1.2.0");
        let own = Source::parse("org/picasso").unwrap();
        assert_eq!(own.version_tag("v1.2.0"), Some("v1.2.0"));
        assert_eq!(own.tag_for("1.2.0"), "v1.2.0");
    }
}
