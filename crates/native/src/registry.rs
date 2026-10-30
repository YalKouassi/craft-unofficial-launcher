//! The apps the user added: reading each one's `craft-app.json` from its latest release, checking
//! it, pinning its identity, and keeping the list on disk.
//!
//! Adding `owner/repo` fetches that repo's latest release, its `SHA256SUMS.txt`, its descriptor and
//! its icon, checks the descriptor and icon against the checksums, runs the release contract
//! (`craft_app_descriptor::check_release`) and only then lists the app. Nothing from the release is
//! ever run: the launcher reads JSON and decodes a PNG.
//!
//! The identity a descriptor declares when the app is added (id, bundle id, Windows publisher and
//! Apple team) is pinned. A later descriptor that changes any of them is refused and the app keeps
//! its pinned entry until the user removes it and adds it again, so a taken-over repo can't quietly
//! start claiming someone else's installs.

use std::path::{Path, PathBuf};

use artcraft_launcher_core::CraftApp;
use artcraft_launcher_core::release::Release;
use craft_app_descriptor::{DESCRIPTOR_FILE, Descriptor, MAX_DESCRIPTOR_BYTES, MAX_ICON_BYTES, SUMS_FILE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How many apps the user may add.
pub const MAX_APPS: usize = 100;
const MAX_SUMS_BYTES: usize = 1024 * 1024;
const MAX_LIST_BYTES: usize = 4 * 1024 * 1024;

/// What the registry needs from the network. The backend answers with GitHub; tests with a fake.
pub trait Fetch {
    /// `repo`'s latest stable release, with its assets' download links checked to be under `repo`.
    fn latest(&self, repo: &str) -> Result<Release, String>;
    /// A small file. `Ok(None)`: not found.
    fn bytes(&self, url: &str, max: usize) -> Result<Option<Vec<u8>>, String>;
}

/// The identity pinned when an app was added.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,
    pub bundle_id: String,
    pub msi_publisher: Option<String>,
    pub apple_team: Option<String>,
}

impl Pin {
    fn of(d: &Descriptor) -> Self {
        Self { id: d.id.clone(), bundle_id: d.bundle_id.clone(), msi_publisher: d.msi_publisher.clone(), apple_team: d.apple_team.clone() }
    }

    /// What changed between the pinned identity and `d`'s, in words. Empty: nothing.
    fn drift(&self, d: &Descriptor) -> Vec<String> {
        let now = Self::of(d);
        let mut out = Vec::new();
        let mut cmp = |what: &str, a: Option<&str>, b: Option<&str>| {
            if a != b {
                out.push(format!("{what} changed from {} to {}", a.unwrap_or("none"), b.unwrap_or("none")));
            }
        };
        cmp("id", Some(&self.id), Some(&now.id));
        cmp("bundle_id", Some(&self.bundle_id), Some(&now.bundle_id));
        cmp("msi_publisher", self.msi_publisher.as_deref(), now.msi_publisher.as_deref());
        cmp("apple_team", self.apple_team.as_deref(), now.apple_team.as_deref());
        out
    }
}

/// One added app, as saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stored {
    /// The repo the user entered, as GitHub spells it in the descriptor.
    pub repo: String,
    /// The release the descriptor was read from.
    pub tag: String,
    pub descriptor: Descriptor,
    pub pin: Pin,
    /// Why the last refresh was refused, if it was (shown on the app's card).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub problems: Vec<String>,
    /// It came from the app index rather than the user's Add app. Removing it remembers the repo
    /// so the index doesn't bring it back.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_index: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    apps: Vec<Stored>,
    /// Index repos the user removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dismissed: Vec<String>,
}

/// A descriptor fetched and checked, with its icon.
#[derive(Clone, Debug)]
pub struct Fetched {
    pub tag: String,
    pub descriptor: Descriptor,
    pub icon: Option<Vec<u8>>,
    pub warnings: Vec<String>,
}

/// The saved list, in the order the user added the apps.
#[derive(Debug)]
pub struct Registry {
    dir: PathBuf,
    apps: Vec<Stored>,
    dismissed: Vec<String>,
}

impl Registry {
    /// Reads the saved list. A missing or unreadable file is an empty list; an entry that no longer
    /// passes the checks (a stricter launcher) is dropped with a log line.
    pub fn load(dir: &Path) -> Self {
        let file: File = std::fs::read(dir.join("apps.json"))
            .ok()
            .filter(|b| b.len() <= MAX_LIST_BYTES)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let apps = file
            .apps
            .into_iter()
            .filter(|s| match s.descriptor.check(&s.repo) {
                Ok(()) => true,
                Err(p) => {
                    log::warn!("dropping {} from the saved list: {p}", s.repo);
                    false
                }
            })
            .take(MAX_APPS)
            .collect();
        Self { dir: dir.to_path_buf(), apps, dismissed: file.dismissed.into_iter().take(MAX_APPS * 4).collect() }
    }

    pub fn apps(&self) -> &[Stored] {
        &self.apps
    }

    pub fn repos(&self) -> Vec<String> {
        self.apps.iter().map(|s| s.repo.clone()).collect()
    }

    fn icon_path(&self, id: &str) -> PathBuf {
        self.dir.join("icons").join(format!("{id}.png"))
    }

    /// The catalog entries, for `catalog::set_apps`.
    pub fn entries(&self) -> Vec<CraftApp> {
        self.apps
            .iter()
            .map(|s| {
                let icon = std::fs::read(self.icon_path(&s.descriptor.id)).ok().filter(|b| craft_app_descriptor::check_icon(b).is_ok());
                CraftApp::from_descriptor(&s.descriptor, icon.as_deref())
            })
            .collect()
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&File { apps: self.apps.clone(), dismissed: self.dismissed.clone() }).map_err(|e| e.to_string())?;
        crate::paths::write_atomic(&self.dir.join("apps.json"), &json)
    }

    fn save_icon(&self, id: &str, icon: Option<&[u8]>) {
        let path = self.icon_path(id);
        match icon {
            Some(png) => {
                if let Err(e) = crate::paths::write_atomic(&path, png) {
                    log::warn!("couldn't save {id}'s icon: {e}");
                }
            }
            None => {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    /// Adds `repo` because the user asked (Add app). Returns the new app's id.
    pub fn add(&mut self, fetch: &dyn Fetch, repo: &str) -> Result<String, Vec<String>> {
        let id = self.insert(fetch, repo, false)?;
        if let Some(added) = self.apps.iter().find(|s| s.descriptor.id == id).map(|s| s.repo.clone()) {
            self.dismissed.retain(|d| !d.eq_ignore_ascii_case(&added));
            let _ = self.save();
        }
        Ok(id)
    }

    /// Brings the list in step with the app index: each listed repo that isn't in the list (and
    /// that the user didn't remove) is checked like a hand-added one and appears if it passes; one
    /// that fails is skipped quietly and tried again next time. Index apps whose repo left the index
    /// are dropped. Returns whether the list changed.
    pub fn sync_index(&mut self, fetch: &dyn Fetch, index: &[craft_app_descriptor::IndexEntry]) -> bool {
        let listed = |repo: &str| index.iter().any(|e| e.repo.eq_ignore_ascii_case(repo));
        let before = self.apps.len();
        let gone: Vec<String> = self.apps.iter().filter(|s| s.from_index && !listed(&s.repo)).map(|s| s.descriptor.id.clone()).collect();
        for id in &gone {
            self.apps.retain(|s| &s.descriptor.id != id);
            self.save_icon(id, None);
        }
        let mut changed = self.apps.len() != before;
        if changed && let Err(e) = self.save() {
            log::warn!("couldn't save the list: {e}");
        }
        for entry in index {
            let known = self.apps.iter().any(|s| s.repo.eq_ignore_ascii_case(&entry.repo));
            let dismissed = self.dismissed.iter().any(|d| d.eq_ignore_ascii_case(&entry.repo));
            if known || dismissed {
                continue;
            }
            match self.insert(fetch, &entry.repo, true) {
                Ok(id) => {
                    log::info!("{}: listed from the app index as {id}", entry.repo);
                    changed = true;
                }
                Err(p) => log::info!("{}: in the app index but not listed: {}", entry.repo, p.join("; ")),
            }
        }
        changed
    }

    fn insert(&mut self, fetch: &dyn Fetch, repo: &str, from_index: bool) -> Result<String, Vec<String>> {
        let repo =
            normalise_repo(repo).ok_or_else(|| vec![format!("{repo:?} isn't a GitHub repository (owner/name, or its https://github.com/ link)")])?;
        if craft_app_descriptor::reserved::repo(&repo) {
            return Err(vec!["official ArtCraft apps belong in ArtCraft Launcher; this launcher only lists unofficial apps".into()]);
        }
        if repo.eq_ignore_ascii_case(craft_app_descriptor::reserved::LAUNCHER_REPO) {
            return Err(vec!["that's this launcher's own repository; it updates itself and isn't listed as an app".into()]);
        }
        if self.apps.iter().any(|s| s.repo.eq_ignore_ascii_case(&repo)) {
            return Err(vec![format!("{repo} is already in the list")]);
        }
        if self.apps.len() >= MAX_APPS {
            return Err(vec![format!("the list is full ({MAX_APPS} apps)")]);
        }
        let got = fetch_descriptor(fetch, &repo)?;
        let d = &got.descriptor;
        if let Some(other) = self.apps.iter().find(|s| s.descriptor.id == d.id || s.descriptor.bundle_id.eq_ignore_ascii_case(&d.bundle_id)) {
            return Err(vec![format!("{} uses the same id or bundle id as {}, which is already in the list", d.name, other.descriptor.name)]);
        }
        self.save_icon(&d.id, got.icon.as_deref());
        let id = d.id.clone();
        self.apps.push(Stored { repo: d.repo.clone(), tag: got.tag, pin: Pin::of(d), descriptor: got.descriptor, problems: Vec::new(), from_index });
        self.save().map_err(|e| vec![format!("couldn't save the list: {e}")])?;
        Ok(id)
    }

    /// Removes the app `id` from the list (it stays installed). An app from the index is remembered
    /// as removed so the index doesn't bring it back. Whether it was there.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(at) = self.apps.iter().position(|s| s.descriptor.id == id) else { return false };
        let gone = self.apps.remove(at);
        if gone.from_index && !self.dismissed.iter().any(|d| d.eq_ignore_ascii_case(&gone.repo)) {
            self.dismissed.push(gone.repo);
        }
        self.save_icon(id, None);
        if let Err(e) = self.save() {
            log::warn!("couldn't save the list: {e}");
        }
        true
    }

    /// Reads every app's latest descriptor again. A descriptor that fails its checks or changes the
    /// pinned identity is refused and the old one kept, with the reasons recorded. A network
    /// failure changes nothing. Returns whether anything changed.
    pub fn refresh(&mut self, fetch: &dyn Fetch) -> bool {
        let mut changed = false;
        for i in 0..self.apps.len() {
            let Some(s) = self.apps.get(i).cloned() else { continue };
            let problems = match fetch_descriptor(fetch, &s.repo) {
                Ok(got) => {
                    let drift = s.pin.drift(&got.descriptor);
                    if drift.is_empty() {
                        if got.descriptor != s.descriptor || got.tag != s.tag {
                            self.save_icon(&got.descriptor.id, got.icon.as_deref());
                        }
                        if let Some(slot) = self.apps.get_mut(i)
                            && (slot.descriptor != got.descriptor || slot.tag != got.tag || !slot.problems.is_empty())
                        {
                            slot.descriptor = got.descriptor;
                            slot.tag = got.tag;
                            slot.problems.clear();
                            changed = true;
                        }
                        continue;
                    }
                    let mut p = vec![format!("{}'s identity changed in {}, so the launcher kept the one you added:", s.descriptor.name, got.tag)];
                    p.extend(drift);
                    p.push("remove the app and add it again to accept the new identity".into());
                    p
                }
                Err(p) if p.iter().any(|m| m.starts_with(OFFLINE)) => continue,
                Err(p) => p,
            };
            if let Some(slot) = self.apps.get_mut(i)
                && slot.problems != problems
            {
                log::warn!("{}: refresh refused: {}", s.repo, problems.join("; "));
                slot.problems = problems;
                changed = true;
            }
        }
        if changed && let Err(e) = self.save() {
            log::warn!("couldn't save the list: {e}");
        }
        changed
    }
}

/// Problems that start with this are network trouble, not the app's fault.
const OFFLINE: &str = "couldn't reach";

/// `owner/name`, from either that or a `https://github.com/owner/name[/…]` link.
pub fn normalise_repo(input: &str) -> Option<String> {
    craft_app_descriptor::index::repo_from_url(input)
}

fn sha256_hex(bytes: &[u8]) -> String {
    crate::http::hex(&Sha256::digest(bytes))
}

/// Fetches and checks `repo`'s latest descriptor: the release contract, the checksums and the icon.
pub fn fetch_descriptor(fetch: &dyn Fetch, repo: &str) -> Result<Fetched, Vec<String>> {
    let release = fetch.latest(repo).map_err(|e| vec![e])?;
    let asset = |name: &str| release.assets.iter().find(|a| a.name == name);
    let sums_asset =
        asset(SUMS_FILE).ok_or_else(|| vec![format!("{repo} {}: {SUMS_FILE} isn't attached, so nothing in it can be checked", release.tag)])?;
    let sums_bytes = fetch
        .bytes(&sums_asset.url, MAX_SUMS_BYTES)
        .map_err(|e| vec![e])?
        .ok_or_else(|| vec![format!("{SUMS_FILE} of {} wasn't found", release.tag)])?;
    let sums_text = String::from_utf8(sums_bytes).map_err(|_| vec![format!("{SUMS_FILE} isn't text")])?;
    let sums = craft_app_descriptor::parse_sha256sums(&sums_text);
    let verified = |name: &str, max: usize| -> Result<Vec<u8>, Vec<String>> {
        let a = asset(name).ok_or_else(|| vec![format!("{name} isn't attached to {}", release.tag)])?;
        let bytes = fetch.bytes(&a.url, max).map_err(|e| vec![e])?.ok_or_else(|| vec![format!("{name} wasn't found")])?;
        let want = sums.iter().find(|(_, n)| n == name).map(|(h, _)| h.as_str());
        match want {
            Some(h) if *h == sha256_hex(&bytes) => Ok(bytes),
            Some(_) => Err(vec![format!("{name} doesn't match its checksum in {SUMS_FILE}")]),
            None => Err(vec![format!("{SUMS_FILE} doesn't list {name}")]),
        }
    };
    let descriptor = Descriptor::parse(&verified(DESCRIPTOR_FILE, MAX_DESCRIPTOR_BYTES + 1)?, repo).map_err(|p| p.0)?;
    let names: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
    let report = craft_app_descriptor::check_release(&descriptor, &release.tag, &names, Some(&sums_text));
    if !report.ok() {
        let mut p = vec![format!("{repo} {} doesn't keep the release contract:", release.tag)];
        p.extend(report.problems);
        return Err(p);
    }
    let icon = match &descriptor.icon {
        Some(name) => {
            let png = verified(name, MAX_ICON_BYTES + 1)?;
            craft_app_descriptor::check_icon(&png).map_err(|e| vec![e])?;
            Some(png)
        }
        None => None,
    };
    Ok(Fetched { tag: release.tag.clone(), descriptor, icon, warnings: report.warnings })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use artcraft_launcher_core::Asset;

    use super::*;

    const REPO: &str = "someone/notecraft";

    fn png() -> Vec<u8> {
        include_bytes!("../../core/tests/fixtures/sample-icon.png").to_vec()
    }

    fn descriptor(bundle: &str) -> String {
        format!(
            r##"{{"schema":1,"id":"notecraft","name":"NoteCraft","tagline":"Notes","repo":"{REPO}","publisher":"Someone",
                "bundle_id":"{bundle}","msi_publisher":"Someone","color":"#336699","icon":"notecraft.png"}}"##
        )
    }

    /// A release served from memory. `files` maps asset names to contents.
    struct Fake {
        tag: String,
        files: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl Fake {
        fn new(tag: &str, descriptor: &str) -> Self {
            let mut files: HashMap<String, Vec<u8>> = HashMap::new();
            files.insert(DESCRIPTOR_FILE.into(), descriptor.as_bytes().to_vec());
            files.insert("notecraft.png".into(), png());
            files.insert(format!("notecraft-{}-windows-x64.msi", tag.trim_start_matches('v')), b"msi".to_vec());
            let fake = Self { tag: tag.into(), files: Mutex::new(files) };
            fake.write_sums();
            fake
        }

        fn write_sums(&self) {
            let mut files = self.files.lock().unwrap();
            files.remove(SUMS_FILE);
            let mut names: Vec<_> = files.keys().cloned().collect();
            names.sort();
            let sums: String = names.iter().map(|n| format!("{}  {n}\n", sha256_hex(&files[n]))).collect();
            files.insert(SUMS_FILE.into(), sums.into_bytes());
        }
    }

    impl Fetch for Fake {
        fn latest(&self, repo: &str) -> Result<Release, String> {
            let files = self.files.lock().unwrap();
            let assets = files
                .keys()
                .map(|n| Asset {
                    name: n.clone(),
                    size: 1,
                    url: format!("https://github.com/{repo}/releases/download/{}/{n}", self.tag),
                    sha256: None,
                })
                .collect();
            Ok(Release { tag: self.tag.clone(), assets, ..Release::default() })
        }

        fn bytes(&self, url: &str, max: usize) -> Result<Option<Vec<u8>>, String> {
            let name = url.rsplit('/').next().unwrap_or_default();
            let b = self.files.lock().unwrap().get(name).cloned();
            match b {
                Some(b) if b.len() > max => Err("too big".into()),
                b => Ok(b),
            }
        }
    }

    #[test]
    fn adding_checks_everything_and_saves() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        let fake = Fake::new("v1.0.0", &descriptor("org.someone.notecraft"));
        assert_eq!(reg.add(&fake, "https://github.com/someone/notecraft/"), Ok("notecraft".into()));
        assert!(reg.add(&fake, REPO).unwrap_err()[0].contains("already"));
        let again = Registry::load(dir.path());
        assert_eq!(again.repos(), [REPO]);
        let entries = again.entries();
        assert_eq!(entries[0].bundle_id(), "org.someone.notecraft");
        assert!(entries[0].icon_png.is_some(), "the icon was saved and read back");
    }

    #[test]
    fn a_tampered_descriptor_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Fake::new("v1.0.0", &descriptor("org.someone.notecraft"));
        fake.files.lock().unwrap().insert(DESCRIPTOR_FILE.into(), descriptor("org.evil.notecraft").into_bytes());
        let err = Registry::load(dir.path()).add(&fake, REPO).unwrap_err();
        assert!(err[0].contains("checksum"), "{err:?}");
    }

    #[test]
    fn official_repos_and_artcraft_identities_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        assert!(reg.add(&Fake::new("v1.0.0", &descriptor("x.y")), "storytold/photocraft").is_err());
        let err = reg.add(&Fake::new("v1.0.0", &descriptor("ai.storyteller.notecraft")), REPO).unwrap_err();
        assert!(err.iter().any(|p| p.contains("ArtCraft")), "{err:?}");
    }

    #[test]
    fn the_launcher_never_lists_itself() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        let err = reg.add(&Fake::new("v1.0.0", &descriptor("x.y")), "https://github.com/YalKouassi/craft-unofficial-launcher").unwrap_err();
        assert!(err[0].contains("own repository"), "{err:?}");
    }

    #[test]
    fn a_release_that_breaks_the_contract_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Fake::new("v2.0.0", &descriptor("org.someone.notecraft"));
        fake.files.lock().unwrap().insert("notecraft-1.0.0-linux-x86_64.AppImage".into(), b"old".to_vec());
        fake.write_sums();
        let err = Registry::load(dir.path()).add(&fake, REPO).unwrap_err();
        assert!(err.iter().any(|p| p.contains("tag says 2.0.0")), "{err:?}");
    }

    #[test]
    fn identity_drift_keeps_the_pinned_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        reg.add(&Fake::new("v1.0.0", &descriptor("org.someone.notecraft")), REPO).unwrap();
        // A newer release that only changes the tagline is taken.
        let newer = Fake::new("v1.1.0", &descriptor("org.someone.notecraft").replace("\"Notes\"", "\"Better notes\""));
        assert!(reg.refresh(&newer));
        assert_eq!(reg.apps()[0].descriptor.tagline, "Better notes");
        // One that changes the bundle id is not.
        let hijacked = Fake::new("v1.2.0", &descriptor("org.other.notecraft"));
        assert!(reg.refresh(&hijacked));
        let s = &reg.apps()[0];
        assert_eq!(s.descriptor.bundle_id, "org.someone.notecraft");
        assert_eq!(s.tag, "v1.1.0");
        assert!(s.problems.iter().any(|p| p.contains("bundle_id changed")), "{:?}", s.problems);
        assert!(Registry::load(dir.path()).apps()[0].problems.len() > 1, "the refusal is saved");
    }

    #[test]
    fn removing_forgets_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        reg.add(&Fake::new("v1.0.0", &descriptor("org.someone.notecraft")), REPO).unwrap();
        assert!(reg.remove("notecraft"));
        assert!(!reg.remove("notecraft"));
        assert!(Registry::load(dir.path()).apps().is_empty());
    }

    #[test]
    fn repos_are_read_from_links_too() {
        assert_eq!(normalise_repo(" https://github.com/a/b.git ").as_deref(), Some("a/b"));
        assert_eq!(normalise_repo("github.com/a/b/releases").as_deref(), Some("a/b"));
        assert_eq!(normalise_repo("a/b").as_deref(), Some("a/b"));
        assert_eq!(normalise_repo("nonsense"), None);
    }

    #[test]
    fn index_apps_appear_when_they_pass_and_stay_gone_once_removed() {
        use craft_app_descriptor::IndexEntry;
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        let entry = |repo: &str| IndexEntry { repo: repo.into(), name: "x".into() };
        let good = Fake::new("v1.0.0", &descriptor("org.someone.notecraft"));
        // A repo that doesn't keep the contract is skipped quietly.
        let broken = Fake::new("v1.0.0", &descriptor("org.someone.notecraft"));
        broken.files.lock().unwrap().remove(DESCRIPTOR_FILE);
        broken.write_sums();
        assert!(!reg.sync_index(&broken, &[entry(REPO)]));
        assert!(reg.apps().is_empty());
        // Once it does, it appears.
        assert!(reg.sync_index(&good, &[entry(REPO)]));
        assert!(reg.apps()[0].from_index);
        assert!(!reg.sync_index(&good, &[entry(REPO)]), "nothing new the second time");
        // Removed by the user: the index doesn't bring it back, across restarts too.
        assert!(reg.remove("notecraft"));
        assert!(!reg.sync_index(&good, &[entry(REPO)]));
        let mut again = Registry::load(dir.path());
        assert!(!again.sync_index(&good, &[entry(REPO)]));
        // Adding it by hand undoes the removal.
        again.add(&good, REPO).unwrap();
        assert!(!again.apps()[0].from_index);
        // An index app whose repo leaves the index goes; a hand-added one stays.
        assert!(!again.sync_index(&good, &[]));
        assert_eq!(again.apps().len(), 1);
    }

    #[test]
    fn an_index_app_leaves_with_its_entry() {
        use craft_app_descriptor::IndexEntry;
        let dir = tempfile::tempdir().unwrap();
        let mut reg = Registry::load(dir.path());
        let good = Fake::new("v1.0.0", &descriptor("org.someone.notecraft"));
        assert!(reg.sync_index(&good, &[IndexEntry { repo: REPO.into(), name: "x".into() }]));
        assert!(reg.sync_index(&good, &[]));
        assert!(reg.apps().is_empty());
    }
}
