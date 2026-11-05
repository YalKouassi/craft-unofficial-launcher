//! Where release information comes from.
//!
//! [`GitHubSource`] asks the GitHub REST API (one request per app, revalidated with ETags) and,
//! when the API is out of quota, falls back to github.com's release pages: the
//! `releases/latest` redirect names the tag and `SHA256SUMS.txt` lists every file with its
//! checksum, which costs no API quota at all.
//!
//! Every app of a shared release repo (`owner/name#app`) reads the same release list, so it is
//! fetched once and shared for a minute. Its quota-free route is the repo's `latest.json`
//! (`{"apps": {"<app>": "<app>-v1.2.3"}}`, kept by the repo's own workflow) read from
//! raw.githubusercontent.com, then the release's `SHA256SUMS.txt` as above.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use artcraft_launcher_core::protocol::{LatestRelease, Source};
use artcraft_launcher_core::release::{self, Release};
use serde::{Deserialize, Serialize};

use crate::http::{Fetched, Http};

/// One answer from a source.
#[derive(Clone, Debug)]
pub struct Answer {
    pub latest: LatestRelease,
    pub source: Source,
}

/// Looks up latest releases. Called from worker threads.
pub trait ReleaseSource: Send + Sync {
    fn latest(&self, repo: &str, prereleases: bool) -> Answer;
    /// `repo`'s published releases, newest first, for choosing a version.
    fn versions(&self, _repo: &str) -> Result<Vec<Release>, String> {
        Err("version list isn't available here".into())
    }
    /// The release `track` follows: the latest stable one, or the chosen nightly or exact version
    /// from [`versions`](Self::versions), with no fallback to another.
    fn chosen(&self, repo: &str, track: &artcraft_launcher_core::protocol::VersionTrack) -> Answer {
        if *track == artcraft_launcher_core::protocol::VersionTrack::Stable {
            return self.latest(repo, false);
        }
        let latest = match self.versions(repo) {
            Ok(list) => release::choose_version(&list, track).map_or(LatestRelease::None, LatestRelease::Found),
            Err(why) => LatestRelease::Failed(why),
        };
        Answer { latest, source: Source::GitHubApi }
    }
    /// The last answer saved for `repo`, for an instant first paint.
    fn cached(&self, repo: &str, prereleases: bool) -> Option<LatestRelease>;
    /// The tag of `repo`'s newest stable release, by a route that costs no API quota (GitHub's
    /// `releases/latest` redirect), or `None` when it can't be told. Quick: a check pressed by the
    /// user asks this for every app to see whether the feed has fallen behind.
    fn newest_tag(&self, repo: &str) -> Option<String> {
        let _ = repo;
        None
    }
    /// API refused until this time (Unix seconds).
    fn rate_limited_until(&self) -> Option<i64>;
    /// Persist anything worth keeping.
    fn save(&self);
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Entry {
    etag: Option<String>,
    /// `None`: the repo has no releases.
    release: Option<Release>,
    prereleases: bool,
    fetched_at: i64,
}

#[derive(Default, Serialize, Deserialize)]
struct CacheFile {
    entries: BTreeMap<String, Entry>,
}

struct SharedList {
    fetched_at: i64,
    etag: Option<String>,
    text: String,
}

/// How long one fetch of a shared repo's release list serves all its apps.
const SHARED_LIST_FRESH_SECS: i64 = 60;

pub struct GitHubSource {
    http: Http,
    cache: Mutex<CacheFile>,
    versions_cache: Mutex<BTreeMap<String, (i64, Vec<Release>)>>,
    /// A shared repo's release list by API URL.
    lists: Mutex<BTreeMap<String, SharedList>>,
    cache_path: std::path::PathBuf,
    limited_until: Mutex<Option<i64>>,
}

impl GitHubSource {
    pub fn new(http: Http, cache_path: std::path::PathBuf) -> Self {
        let cache = std::fs::read(&cache_path).ok().and_then(|b| serde_json::from_slice::<CacheFile>(&b).ok()).unwrap_or_default();
        Self {
            http,
            cache: Mutex::new(cache),
            versions_cache: Mutex::new(BTreeMap::new()),
            lists: Mutex::new(BTreeMap::new()),
            cache_path,
            limited_until: Mutex::new(None),
        }
    }

    fn entry(&self, repo: &str, prereleases: bool) -> Option<Entry> {
        let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.entries.get(repo).filter(|e| e.prereleases == prereleases).cloned()
    }

    fn store(&self, repo: &str, entry: Entry) {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner).entries.insert(repo.to_string(), entry);
    }

    fn limited(&self) -> bool {
        let until = *self.limited_until.lock().unwrap_or_else(PoisonError::into_inner);
        until.is_some_and(|u| u > crate::now())
    }

    /// `repo` here and below is a source key: `owner/name`, or `owner/name#app` in a shared
    /// release repo, whose app reads the repo's release list and never its `releases/latest`.
    fn ask_api(&self, repo: &str, prereleases: bool) -> Result<Option<Answer>, String> {
        let (url, list) = release::latest_request(repo, prereleases);
        if repo.contains('#') {
            return self.ask_shared_list(repo, prereleases, &url);
        }
        let saved = self.entry(repo, prereleases);
        let found =
            |release: Option<Release>| Answer { latest: release.map_or(LatestRelease::None, LatestRelease::Found), source: Source::GitHubApi };
        match self.http.get_api(&url, saved.as_ref().and_then(|e| e.etag.as_deref()))? {
            Fetched::Body { text, etag } => {
                let release = if list {
                    release::parse_github_release_list(&text, repo, prereleases)?
                } else {
                    Some(release::parse_github_release(&text, repo)?)
                };
                self.store(repo, Entry { etag, release: release.clone(), prereleases, fetched_at: crate::now() });
                Ok(Some(found(release)))
            }
            Fetched::NotModified => match saved {
                Some(e) => {
                    self.store(repo, Entry { fetched_at: crate::now(), ..e.clone() });
                    Ok(Some(found(e.release)))
                }
                None => Err("GitHub said nothing changed, but there's no saved answer".into()),
            },
            Fetched::NotFound => {
                self.store(repo, Entry { etag: None, release: None, prereleases, fetched_at: crate::now() });
                Ok(Some(found(None)))
            }
            Fetched::RateLimited { reset } => {
                *self.limited_until.lock().unwrap_or_else(PoisonError::into_inner) = Some(reset.unwrap_or_else(|| crate::now() + 15 * 60));
                Ok(None)
            }
        }
    }

    /// One app of a shared repo, from the repo's release list. The list is fetched once for all
    /// its apps: the lock is held while fetching, so apps checked at the same time wait for that
    /// one request instead of each making their own.
    fn ask_shared_list(&self, key: &str, prereleases: bool, url: &str) -> Result<Option<Answer>, String> {
        let found =
            |release: Option<Release>| Answer { latest: release.map_or(LatestRelease::None, LatestRelease::Found), source: Source::GitHubApi };
        let text = {
            let mut lists = self.lists.lock().unwrap_or_else(PoisonError::into_inner);
            match lists.get(url) {
                Some(l) if crate::now().saturating_sub(l.fetched_at) < SHARED_LIST_FRESH_SECS => l.text.clone(),
                saved => {
                    let etag = saved.and_then(|l| l.etag.clone());
                    match self.http.get_api(url, etag.as_deref())? {
                        Fetched::Body { text, etag } => {
                            lists.insert(url.to_string(), SharedList { fetched_at: crate::now(), etag, text: text.clone() });
                            text
                        }
                        Fetched::NotModified => match lists.get_mut(url) {
                            Some(l) => {
                                l.fetched_at = crate::now();
                                l.text.clone()
                            }
                            None => return Err("GitHub said nothing changed, but there's no saved answer".into()),
                        },
                        Fetched::NotFound => {
                            drop(lists);
                            self.store(key, Entry { etag: None, release: None, prereleases, fetched_at: crate::now() });
                            return Ok(Some(found(None)));
                        }
                        Fetched::RateLimited { reset } => {
                            *self.limited_until.lock().unwrap_or_else(PoisonError::into_inner) =
                                Some(reset.unwrap_or_else(|| crate::now() + 15 * 60));
                            return Ok(None);
                        }
                    }
                }
            }
        };
        let release = release::parse_github_release_list(&text, key, prereleases)?;
        self.store(key, Entry { etag: None, release: release.clone(), prereleases, fetched_at: crate::now() });
        Ok(Some(found(release)))
    }

    /// The tag a shared repo's `latest.json` names for `key`'s app, or `None` when it names none.
    fn hub_file_tag(&self, key: &str) -> Result<Option<String>, String> {
        let source = craft_app_descriptor::Source::parse(key).ok_or("not a valid source")?;
        let app = source.app.ok_or("not an app of a shared repo")?;
        let url = format!("https://raw.githubusercontent.com/{}/HEAD/latest.json", source.repo);
        let Some(text) = self.http.get_text(&url)? else { return Ok(None) };
        let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("unreadable latest.json ({e})"))?;
        Ok(v["apps"][app]
            .as_str()
            .filter(|t| t.len() <= 64 && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b)))
            .filter(|t| source.version_tag(t).is_some())
            .map(str::to_string))
    }

    /// The quota-free route for an app of a shared repo (stable releases only).
    fn ask_hub_file(&self, key: &str) -> Result<Answer, String> {
        match self.hub_file_tag(key)? {
            Some(tag) => self.release_at_tag(key, tag),
            None => Ok(Answer { latest: LatestRelease::None, source: Source::GitHubPages }),
        }
    }

    /// The quota-free route through github.com (stable releases only).
    fn ask_pages(&self, repo: &str) -> Result<Answer, String> {
        let location = self.http.redirect_target(&format!("https://github.com/{repo}/releases/latest"))?;
        let Some(tag) = location.as_deref().and_then(|l| release::tag_from_redirect(l, repo)) else {
            // No published release: github.com sends us to the release list instead.
            return Ok(Answer { latest: LatestRelease::None, source: Source::GitHubPages });
        };
        self.release_at_tag(repo, tag)
    }

    /// A release known by its tag, completed from its `SHA256SUMS.txt` (or the saved answer for
    /// that tag).
    fn release_at_tag(&self, repo: &str, tag: String) -> Result<Answer, String> {
        if let Some(e) = self.entry(repo, false)
            && let Some(r) = e.release.as_ref().filter(|r| r.tag == tag)
        {
            return Ok(Answer { latest: LatestRelease::Found(r.clone()), source: Source::GitHubPages });
        }
        let sums_url = format!("{}{tag}/SHA256SUMS.txt", release::download_prefix(repo));
        let sums = self.http.get_text(&sums_url)?.map(|t| release::parse_sha256sums(&t)).unwrap_or_default();
        let r = release::release_from_sums(repo, &tag, &sums);
        self.store(repo, Entry { etag: None, release: Some(r.clone()), prereleases: false, fetched_at: crate::now() });
        Ok(Answer { latest: LatestRelease::Found(r), source: Source::GitHubPages })
    }
}

impl ReleaseSource for GitHubSource {
    fn versions(&self, repo: &str) -> Result<Vec<Release>, String> {
        if let Some((at, list)) = self.versions_cache.lock().unwrap_or_else(PoisonError::into_inner).get(repo)
            && crate::now().saturating_sub(*at) < 15 * 60
        {
            return Ok(list.clone());
        }
        if self.limited() {
            return Err("GitHub's request limit was reached. Try again later.".into());
        }
        let url = format!("https://api.github.com/repos/{}/releases?per_page=100", craft_app_descriptor::source::repo_of(repo));
        let list = match self.http.get_api(&url, None)? {
            Fetched::Body { text, .. } => release::parse_github_versions(&text, repo)?,
            Fetched::NotFound => Vec::new(),
            Fetched::RateLimited { reset } => {
                *self.limited_until.lock().unwrap_or_else(PoisonError::into_inner) = Some(reset.unwrap_or_else(|| crate::now() + 15 * 60));
                return Err("GitHub's request limit was reached. Try again later.".into());
            }
            Fetched::NotModified => return Err("GitHub returned no version list. Try again.".into()),
        };
        self.versions_cache.lock().unwrap_or_else(PoisonError::into_inner).insert(repo.to_string(), (crate::now(), list.clone()));
        Ok(list)
    }

    fn latest(&self, repo: &str, prereleases: bool) -> Answer {
        let api_error = if self.limited() {
            None
        } else {
            match self.ask_api(repo, prereleases) {
                Ok(Some(answer)) => return answer,
                Ok(None) => None,
                Err(e) => Some(e),
            }
        };
        // The release pages know only one latest release per repo, so a shared repo's app reads
        // the repo's latest.json instead.
        if !prereleases && api_error.is_none() {
            let answer = if repo.contains('#') { self.ask_hub_file(repo) } else { self.ask_pages(repo) };
            match answer {
                Ok(answer) => return answer,
                Err(e) => log::warn!("{repo}: release pages: {e}"),
            }
        }
        let why = api_error.unwrap_or_else(|| "GitHub's request limit was reached".into());
        match self.cached(repo, prereleases) {
            Some(latest) => Answer { latest, source: Source::Cache },
            None => Answer { latest: LatestRelease::Failed(why), source: Source::Cache },
        }
    }

    fn cached(&self, repo: &str, prereleases: bool) -> Option<LatestRelease> {
        self.entry(repo, prereleases).map(|e| e.release.map_or(LatestRelease::None, LatestRelease::Found))
    }

    fn newest_tag(&self, repo: &str) -> Option<String> {
        if repo.contains('#') {
            return self.hub_file_tag(repo).unwrap_or_else(|e| {
                log::debug!("{repo}: couldn't read latest.json: {e}");
                None
            });
        }
        match self.http.redirect_target(&format!("https://github.com/{repo}/releases/latest")) {
            Ok(location) => location.as_deref().and_then(|l| release::tag_from_redirect(l, repo)),
            Err(e) => {
                log::debug!("{repo}: couldn't read the latest-release redirect: {e}");
                None
            }
        }
    }

    fn rate_limited_until(&self) -> Option<i64> {
        let until = *self.limited_until.lock().unwrap_or_else(PoisonError::into_inner);
        until.filter(|u| *u > crate::now())
    }

    fn save(&self) {
        let json = {
            let cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            serde_json::to_vec(&*cache)
        };
        match json {
            Ok(bytes) => {
                if let Err(e) = crate::paths::write_atomic(&self.cache_path, &bytes) {
                    log::warn!("release cache: {e}");
                }
            }
            Err(e) => log::warn!("release cache: {e}"),
        }
    }
}
