//! The apps the launcher knows about: unofficial Craft-style apps, each described by the
//! `craft-app.json` its own releases carry.
//!
//! Upstream compiles its catalog in. Here the catalog is data: the native backend reads each
//! added app's descriptor, checks it (`craft_app_descriptor`), turns it into a [`CraftApp`] with
//! [`CraftApp::from_descriptor`] and publishes the list with [`set_apps`]. Everything else reads
//! [`apps`] and [`CraftApp::by_id`], which hand out `&'static` entries so the rest of the code can
//! keep treating an app as plain data.
//!
//! Text here is whatever the app's maintainer wrote; it is shown as-is, never translated.

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError, RwLock};

use craft_app_descriptor::Descriptor;

/// Who makes an app. Every app this launcher lists is [`AppKind::Community`] (shown as
/// "Unofficial"); [`AppKind::Official`] survives only so upstream's tests and merges keep working.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Official,
    Community,
}

/// One app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraftApp {
    /// Machine name: binary, package and asset stem (`notecraft`).
    pub id: &'static str,
    /// Display name (`NoteCraft`).
    pub name: &'static str,
    /// What it's for, in a few words, as its maintainer wrote it.
    pub tagline: &'static str,
    /// Brand colour (sRGB), for surfaces.
    pub color: [u8; 3],
    /// The brand colour for text on a light background.
    pub ink_light: [u8; 3],
    /// The brand colour for text on a dark background.
    pub ink_dark: [u8; 3],
    /// GitHub `owner/repo`, pinned when the user added the app. Releases come only from here.
    pub repo: &'static str,
    /// Older names its release assets may carry.
    pub former_ids: &'static [&'static str],
    /// Older display names its installers registered.
    pub former_names: &'static [&'static str],
    pub kind: AppKind,
    /// Who publishes it, as the card's byline shows it.
    pub publisher: &'static str,
    /// The Publisher its Windows installer registers. `None`: no MSI is ever recognised as it.
    pub msi_publisher: Option<&'static str>,
    /// Its bundle id (macOS bundle, Flatpak and desktop-file id). `None`: the bare `id`.
    pub bundle: Option<&'static str>,
    /// Older bundle ids it shipped under.
    pub former_bundles: &'static [&'static str],
    /// Its website. `None`: its repository.
    pub site: Option<&'static str>,
    /// A page where it can be tried in a web browser. `None`: it names none.
    pub web_demo: Option<&'static str>,
    /// The Apple Developer ID team that signs its `.app`. `None`: its DMG is never installed, only
    /// opened if it's already there.
    pub apple_team: Option<&'static str>,
    /// The project a bridge repo packages, when this is one.
    pub upstream: Option<&'static str>,
    /// Its icon (PNG, checked with `craft_app_descriptor::check_icon`).
    pub icon_png: Option<&'static [u8]>,
}

impl CraftApp {
    /// Its macOS bundle id, Flatpak id and Linux desktop-file id.
    pub fn bundle_id(&self) -> String {
        self.bundle.unwrap_or(self.id).to_string()
    }

    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.repo)
    }

    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }

    /// Its website, or its repository when it names none.
    pub fn website(&self) -> String {
        self.site.map_or_else(|| self.repo_url(), str::to_string)
    }

    /// Whether the launcher installs this app from a `kind` package.
    pub fn installs(&self, kind: crate::PackageKind) -> bool {
        kind != crate::PackageKind::Dmg || self.apple_team.is_some()
    }

    /// Bundle ids this app may be installed under, current first.
    pub fn bundle_ids(&self) -> impl Iterator<Item = String> + '_ {
        std::iter::once(self.bundle_id()).chain(self.former_bundles.iter().map(|b| b.to_string()))
    }

    /// Asset stems this app's releases may use, current first.
    pub fn stems(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.id).chain(self.former_ids.iter().copied())
    }

    /// Display names this app's installers may have registered, current first.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.name).chain(self.former_names.iter().copied())
    }

    /// An app in the current catalog, or the launcher itself.
    pub fn by_id(id: &str) -> Option<&'static CraftApp> {
        apps().iter().find(|a| a.id == id).or_else(|| (id == LAUNCHER_ID).then_some(&LAUNCHER))
    }

    /// The entry for a checked descriptor and its (checked) icon. The same descriptor always
    /// gives back the same entry, so refreshing the catalog doesn't grow memory.
    pub fn from_descriptor(d: &Descriptor, icon: Option<&[u8]>) -> CraftApp {
        type Key = (String, Option<Vec<u8>>);
        static INTERNED: Mutex<BTreeMap<Key, CraftApp>> = Mutex::new(BTreeMap::new());
        let key = (d.to_json(), icon.map(<[u8]>::to_vec));
        let mut interned = INTERNED.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(app) = interned.get(&key) {
            return *app;
        }
        let color = d.rgb();
        let app = CraftApp {
            id: leak(&d.id),
            name: leak(&d.name),
            tagline: leak(&d.tagline),
            color,
            ink_light: mix(color, [0, 0, 0], 0.3),
            ink_dark: mix(color, [255, 255, 255], 0.35),
            repo: leak(&d.repo),
            former_ids: leak_all(&d.former_ids),
            former_names: leak_all(&d.former_names),
            kind: AppKind::Community,
            publisher: leak(&d.publisher),
            msi_publisher: d.msi_publisher.as_deref().map(leak),
            bundle: Some(leak(&d.bundle_id)),
            former_bundles: leak_all(&d.former_bundles),
            site: d.website.as_deref().map(leak),
            web_demo: d.web_demo.as_deref().map(leak),
            apple_team: d.apple_team.as_deref().map(leak),
            upstream: d.upstream.as_deref().map(leak),
            icon_png: icon.filter(|i| craft_app_descriptor::check_icon(i).is_ok()).map(|i| &*Box::leak(i.to_vec().into_boxed_slice())),
        };
        interned.insert(key, app);
        app
    }
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

fn leak_all(v: &[String]) -> &'static [&'static str] {
    Box::leak(v.iter().map(|s| leak(s)).collect::<Vec<_>>().into_boxed_slice())
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    // Both inputs are 0..=255 and t is 0..=1, so the result stays in range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round().clamp(0.0, 255.0) as u8;
    [ch(a[0], b[0]), ch(a[1], b[1]), ch(a[2], b[2])]
}

#[cfg(not(any(test, feature = "sample-catalog")))]
const INITIAL: &[CraftApp] = &[];
#[cfg(any(test, feature = "sample-catalog"))]
const INITIAL: &[CraftApp] = &crate::sample::SUITE;

static APPS: RwLock<&'static [CraftApp]> = RwLock::new(INITIAL);

/// The apps the user has added, in display order.
pub fn apps() -> &'static [CraftApp] {
    *APPS.read().unwrap_or_else(PoisonError::into_inner)
}

/// Replaces the catalog. Entries keep living after they're replaced (the old list is leaked, a few
/// hundred bytes per change), so `&'static` references held elsewhere stay valid.
pub fn set_apps(apps: Vec<CraftApp>) {
    let mut unique: Vec<CraftApp> = Vec::with_capacity(apps.len());
    for a in apps {
        if a.id != LAUNCHER_ID && !unique.iter().any(|u| u.id == a.id) {
            unique.push(a);
        }
    }
    *APPS.write().unwrap_or_else(PoisonError::into_inner) = Box::leak(unique.into_boxed_slice());
}

/// The project's home.
pub const WEBSITE_URL: &str = "https://github.com/YalKouassi/craft-unofficial-launcher";
/// The project on GitHub.
pub const GITHUB_URL: &str = "https://github.com/YalKouassi/craft-unofficial-launcher";
/// The account an official ArtCraft app lives under: never listed here.
pub const GITHUB_OWNER: &str = "storytold";
/// Craft Unofficial Launcher itself, described like an app so it can update itself with the same
/// machinery (asset names `craft-unofficial-launcher-<ver>-…`). Not part of [`apps`].
pub const LAUNCHER: CraftApp = CraftApp {
    id: "craft-unofficial-launcher",
    name: "Craft Unofficial Launcher",
    tagline: "Installs and updates unofficial Craft-style apps",
    color: [0x0b, 0x0b, 0x0c],
    ink_light: [0x0b, 0x0b, 0x0c],
    ink_dark: [0xf2, 0xf1, 0xee],
    repo: LAUNCHER_REPO,
    former_ids: &[],
    former_names: &[],
    kind: AppKind::Community,
    publisher: "Craft Unofficial Launcher contributors",
    msi_publisher: Some("Craft Unofficial Launcher contributors"),
    bundle: Some("io.github.yalkouassi.craft-unofficial-launcher"),
    former_bundles: &[],
    site: None,
    web_demo: None,
    apple_team: None,
    upstream: None,
    icon_png: None,
};

/// The launcher's own repo, checked so it can say when a newer launcher is out.
pub const LAUNCHER_REPO: &str = "YalKouassi/craft-unofficial-launcher";
/// The launcher's own asset stem.
pub const LAUNCHER_ID: &str = "craft-unofficial-launcher";

/// An app's icon (PNG): the UI draws it, and the Linux install puts it in the desktop menu.
pub fn app_icon_png(id: &str) -> Option<&'static [u8]> {
    CraftApp::by_id(id).and_then(|a| a.icon_png)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str, website: Option<&str>) -> Descriptor {
        let mut d = Descriptor {
            schema: 1,
            id: id.into(),
            name: "NoteCraft".into(),
            tagline: "Notes".into(),
            repo: format!("someone/{id}"),
            publisher: "Someone".into(),
            upstream: None,
            bundle_id: format!("io.github.someone.{id}"),
            msi_publisher: Some("Someone".into()),
            apple_team: None,
            website: website.map(Into::into),
            web_demo: None,
            color: "#336699".into(),
            icon: None,
            former_ids: vec![],
            former_names: vec![],
            former_bundles: vec![],
        };
        d.check(&d.repo.clone()).unwrap();
        d.former_ids = vec!["oldnote".into()];
        d
    }

    #[test]
    fn a_descriptor_becomes_an_unofficial_entry() {
        let app = CraftApp::from_descriptor(&descriptor("notecraft", None), None);
        assert_eq!(app.kind, AppKind::Community);
        assert_eq!(app.bundle_id(), "io.github.someone.notecraft");
        assert_eq!(app.website(), "https://github.com/someone/notecraft");
        assert_eq!(app.stems().collect::<Vec<_>>(), ["notecraft", "oldnote"]);
        assert!(!app.installs(crate::PackageKind::Dmg), "no Apple team, no DMG install");
        assert!(app.installs(crate::PackageKind::Msi));
        assert_eq!(app.color, [0x33, 0x66, 0x99]);
        // The same descriptor hands back the very same strings.
        assert!(std::ptr::eq(app.id, CraftApp::from_descriptor(&descriptor("notecraft", None), None).id));
    }

    #[test]
    fn an_icon_that_isnt_a_png_is_dropped() {
        let app = CraftApp::from_descriptor(&descriptor("badicon", None), Some(b"<svg/>"));
        assert!(app.icon_png.is_none());
    }

    #[test]
    fn the_launcher_is_always_found_but_never_listed() {
        assert_eq!(CraftApp::by_id(LAUNCHER_ID).map(|a| a.repo), Some(LAUNCHER_REPO));
        assert!(!apps().iter().any(|a| a.id == LAUNCHER_ID));
    }

    #[test]
    fn the_sample_catalog_has_unique_ids() {
        let mut ids: Vec<_> = crate::sample::SUITE.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), crate::sample::SUITE.len());
    }
}
