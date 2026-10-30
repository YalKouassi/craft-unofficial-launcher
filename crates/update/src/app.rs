//! Which app an installer or an update works on, and who makes it.
//!
//! Nothing here defaults to anyone's identity: every app names its own bundle id, MSI publisher,
//! Apple Developer ID team and website. An unofficial app normally builds its [`App`] from the
//! `craft-app.json` it ships ([`App::from_embedded`]), so the updater, the launcher and the CI
//! checker all read the same description.

use craft_app_descriptor::{Descriptor, Problems};

/// An app, as its releases and installers know it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct App {
    /// Machine name: repo, binary, package and asset stem (`examplecraft`).
    pub id: &'static str,
    /// Display name (`ExampleCraft`).
    pub name: &'static str,
    /// What it's for, in a few words (English). Written into the menu entries installers add.
    pub tagline: &'static str,
    /// GitHub `owner/repo`. Releases are only ever downloaded from this repository.
    pub repo: &'static str,
    /// Older names its release assets may carry.
    pub former_ids: &'static [&'static str],
    /// Older display names its installers registered.
    pub former_names: &'static [&'static str],
    /// Its 128 px PNG icon, for the menu entry written beside an AppImage. `None`: no icon.
    pub icon_png: Option<&'static [u8]>,
    /// Reverse-DNS id (`org.example.examplecraft`): macOS bundle id, Flatpak id and Linux
    /// desktop-file id.
    pub bundle_id: &'static str,
    /// Older bundle ids it shipped under (still recognised when detecting installs).
    pub former_bundles: &'static [&'static str],
    /// The `Publisher` its Windows Installer package registers. `None`: no MSI install is ever
    /// recognised as this app's.
    pub msi_publisher: Option<&'static str>,
    /// The Apple Developer ID team that signs its `.app`. `None`: a DMG is never installed, only
    /// a copy that's already there is opened.
    pub apple_team: Option<&'static str>,
    /// Its website. `None`: the GitHub repository is its page.
    pub website: Option<&'static str>,
    /// The `craft-app.json` this app was built from, when it was. The updater checks each release
    /// against it before downloading anything (see [`crate::contract`]).
    pub descriptor: Option<&'static Descriptor>,
}

impl App {
    /// An app with no former names, no icon, no MSI publisher, no Apple team and no website.
    pub const fn new(id: &'static str, name: &'static str, tagline: &'static str, repo: &'static str, bundle_id: &'static str) -> Self {
        Self {
            id,
            name,
            tagline,
            repo,
            former_ids: &[],
            former_names: &[],
            icon_png: None,
            bundle_id,
            former_bundles: &[],
            msi_publisher: None,
            apple_team: None,
            website: None,
            descriptor: None,
        }
    }

    /// The app a checked `craft-app.json` describes.
    ///
    /// Leaks the descriptor and its strings so the result lives for the whole process: call it
    /// once per process (typically through [`App::from_embedded`] at start-up) and keep the
    /// reference, never once per frame or per check.
    pub fn from_descriptor(d: &Descriptor) -> &'static App {
        let d: &'static Descriptor = Box::leak(Box::new(d.clone()));
        let list = |v: &'static [String]| -> &'static [&'static str] { Box::leak(v.iter().map(String::as_str).collect::<Box<[_]>>()) };
        Box::leak(Box::new(App {
            id: &d.id,
            name: &d.name,
            tagline: &d.tagline,
            repo: &d.repo,
            former_ids: list(&d.former_ids),
            former_names: list(&d.former_names),
            icon_png: None,
            bundle_id: &d.bundle_id,
            former_bundles: list(&d.former_bundles),
            msi_publisher: d.msi_publisher.as_deref(),
            apple_team: d.apple_team.as_deref(),
            website: d.website.as_deref(),
            descriptor: Some(d),
        }))
    }

    /// Reads and checks the `craft-app.json` compiled into the app, which must name `repo` (the
    /// GitHub `owner/repo` the app is published from):
    ///
    /// ```ignore
    /// let app = App::from_embedded(include_str!("../craft-app.json"), "example-org/examplecraft")?;
    /// ```
    ///
    /// Like [`App::from_descriptor`], call it once per process.
    pub fn from_embedded(json: &'static str, repo: &str) -> Result<&'static App, Problems> {
        Descriptor::parse(json.as_bytes(), repo).map(|d| App::from_descriptor(&d))
    }

    /// The descriptor the release contract is checked against: the one this app was built from,
    /// else one made from its fields (for an [`App`] written out by hand).
    pub fn contract_descriptor(&self) -> Descriptor {
        if let Some(d) = self.descriptor {
            return d.clone();
        }
        let owned = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect();
        Descriptor {
            schema: craft_app_descriptor::SCHEMA,
            id: self.id.into(),
            name: self.name.into(),
            tagline: self.tagline.into(),
            repo: self.repo.into(),
            publisher: self.msi_publisher.unwrap_or(self.name).into(),
            upstream: None,
            bundle_id: self.bundle_id.into(),
            msi_publisher: self.msi_publisher.map(Into::into),
            apple_team: self.apple_team.map(Into::into),
            website: self.website.map(Into::into),
            web_demo: None,
            color: "#000000".into(),
            icon: None,
            former_ids: owned(self.former_ids),
            former_names: owned(self.former_names),
            former_bundles: owned(self.former_bundles),
        }
    }

    /// Reverse-DNS id: macOS bundle id, Flatpak id and Linux desktop-file id.
    pub fn bundle_id(&self) -> &'static str {
        self.bundle_id
    }

    /// Bundle ids this app's packages may carry, current first.
    pub fn bundle_ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.bundle_id).chain(self.former_bundles.iter().copied())
    }

    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases", self.repo)
    }

    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }

    /// The app's website, else its GitHub repository.
    pub fn website(&self) -> String {
        self.website.map_or_else(|| self.repo_url(), str::to_string)
    }

    /// Asset stems this app's releases may use, current first.
    pub fn stems(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.id).chain(self.former_ids.iter().copied())
    }

    /// Display names this app's installers may have registered, current first.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        std::iter::once(self.name).chain(self.former_names.iter().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{EXAMPLE, NEW};

    #[test]
    fn former_names_follow_the_current_one() {
        assert_eq!(NEW.stems().collect::<Vec<_>>(), ["newcraft", "oldcraft"]);
        assert_eq!(NEW.names().collect::<Vec<_>>(), ["NewCraft", "OldCraft"]);
        assert_eq!(NEW.bundle_id(), "org.example.newcraft");
        assert_eq!(NEW.bundle_ids().collect::<Vec<_>>(), ["org.example.newcraft", "org.example.oldcraft"]);
        assert_eq!(NEW.releases_url(), "https://github.com/example-org/newcraft/releases");
    }

    #[test]
    fn nothing_defaults_to_anyones_identity() {
        let bare = App::new("bare", "Bare", "Nothing else", "someone/bare", "org.example.bare");
        assert_eq!(bare.bundle_id(), "org.example.bare");
        assert_eq!(bare.website(), "https://github.com/someone/bare");
        assert_eq!((bare.msi_publisher, bare.apple_team), (None, None));
        assert_eq!(EXAMPLE.website(), "https://example.org/examplecraft");
    }

    const JSON: &str = r##"{
        "schema": 1,
        "id": "examplecraft",
        "name": "ExampleCraft",
        "tagline": "What it's for, in a few words",
        "repo": "example-org/examplecraft",
        "publisher": "Example Org",
        "bundle_id": "org.example.examplecraft",
        "msi_publisher": "Example Org",
        "website": "https://example.org/examplecraft",
        "color": "#7a45b8",
        "former_ids": ["oldexample"],
        "former_bundles": ["org.example.oldexample"]
    }"##;

    #[test]
    fn an_embedded_descriptor_becomes_the_app() {
        let app = App::from_embedded(JSON, "example-org/examplecraft").unwrap();
        assert_eq!((app.id, app.name, app.repo), ("examplecraft", "ExampleCraft", "example-org/examplecraft"));
        assert_eq!(app.bundle_id(), "org.example.examplecraft");
        assert_eq!(app.former_ids, ["oldexample"]);
        assert_eq!(app.bundle_ids().collect::<Vec<_>>(), ["org.example.examplecraft", "org.example.oldexample"]);
        assert_eq!(app.msi_publisher, Some("Example Org"));
        assert_eq!(app.apple_team, None);
        assert_eq!(app.website(), "https://example.org/examplecraft");
        assert_eq!(app.contract_descriptor().color, "#7a45b8");
    }

    #[test]
    fn an_embedded_descriptor_must_name_the_repo_and_keep_out_of_artcraft() {
        assert!(App::from_embedded(JSON, "someone-else/examplecraft").is_err());
        let official = JSON.replace("org.example.examplecraft", "ai.storyteller.examplecraft");
        let official: &'static str = Box::leak(official.into_boxed_str());
        assert!(App::from_embedded(official, "example-org/examplecraft").is_err());
    }

    #[test]
    fn a_hand_written_app_still_has_a_contract_descriptor() {
        let d = EXAMPLE.contract_descriptor();
        assert_eq!(
            (d.id.as_str(), d.bundle_id.as_str(), d.msi_publisher.as_deref()),
            ("examplecraft", "org.example.examplecraft", Some("Example Org"))
        );
    }
}
