//! Release checking, downloading, installing and self-updating for **unofficial** Craft-style apps
//! and Craft Unofficial Launcher.
//!
//! An app describes itself with the `craft-app.json` it ships ([`App::from_embedded`]); nothing
//! here defaults to ArtCraft's identity, and every update is checked against the release
//! contract ([`contract`]) before anything is downloaded. See the crate's `README.md`.
//!
//! Derived from the `crates/update` crate of
//! [storytold/craft-libs](https://github.com/storytold/craft-libs) at rev
//! `92d392cff132eaf53e13afff77f38b5c11b73aa0` (MIT OR Apache-2.0, Copyright (c) 2026 ArtCraft Team
//! and the craft-libs contributors), modified for unofficial apps. Not made, endorsed or supported
//! by ArtCraft.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod app;
pub mod contract;
pub mod desktop_entry;
pub mod detect;
pub mod distro;
mod folders;
pub mod http;
pub mod install;
pub mod manifest;
pub mod packages;
pub mod paths;
pub mod platform;
#[cfg(windows)]
pub mod portable;
pub mod process;
pub mod protocol;
pub mod proxy;
pub mod queue;
pub mod release;
pub mod self_update;
pub mod source;
pub mod status;
pub mod time;
pub mod version;

#[cfg(feature = "egui")]
pub mod dialog;
#[cfg(feature = "egui")]
pub mod updater;

pub use app::App;
pub use craft_app_descriptor::{self as descriptor, Descriptor, Problems};
pub use http::{Http, Stop};
pub use paths::Paths;
pub use platform::{Arch, Os, PackageKind, Platform, pick_asset};
pub use protocol::{InstallLocations, JobPhase, LatestRelease, Source};
pub use release::{Asset, Release, parse_github_release, parse_github_release_list};
pub use status::{InstallSource, Installed};
pub use time::parse_rfc3339;
pub use version::{Version, is_newer, parse as parse_version};

#[cfg(feature = "egui")]
pub use dialog::{StartupPrompt, StartupPromptResponse};

/// The product token of every request this crate makes on an app's behalf:
/// `Craft-Unofficial-Update/<crate version>`. Generic on purpose: it names no app, person or
/// machine.
pub const USER_AGENT: &str = concat!("Craft-Unofficial-Update/", env!("CARGO_PKG_VERSION"));

/// Checks for an update against GitHub releases for the given app.
pub fn check_for_update(app: &App, http: &Http, current_version: &str) -> Result<Option<Release>, String> {
    let url = format!("https://api.github.com/repos/{}/releases?per_page=30", app.repo);
    match http.get_api(&url, None)? {
        crate::http::Fetched::Body { text, .. } => {
            let latest = crate::release::parse_github_release_list(&text, app.repo, false)?;
            if let Some(release) = latest
                && let Some(rel_v) = &release.version
                && let Some(cur_v) = crate::version::parse(current_version)
                && crate::version::is_newer(rel_v, &cur_v)
            {
                return Ok(Some(release));
            }
            Ok(None)
        }
        crate::http::Fetched::RateLimited { .. } => Err("GitHub's request limit was reached".into()),
        crate::http::Fetched::NotModified | crate::http::Fetched::NotFound => Ok(None),
    }
}

/// Unix seconds now.
pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
pub(crate) const TEST_AGENT: &str = "Craft-Unofficial-Launcher-Test/0.0";

/// Apps the tests work on. Made up: an unofficial SDK ships no official app's identity.
#[cfg(test)]
pub(crate) mod testing {
    use crate::App;

    pub const EXAMPLE: App = App {
        msi_publisher: Some("Example Org"),
        apple_team: Some("EXAMPLE123"),
        website: Some("https://example.org/examplecraft"),
        ..App::new("examplecraft", "ExampleCraft", "Image editing", "example-org/examplecraft", "org.example.examplecraft")
    };
    pub const SAMPLE: App = App {
        msi_publisher: Some("Example Org"),
        apple_team: Some("EXAMPLE123"),
        ..App::new("samplecraft", "SampleCraft", "Writing", "example-org/samplecraft", "org.example.samplecraft")
    };
    pub const DEMO: App =
        App { msi_publisher: Some("Example Org"), ..App::new("democraft", "DemoCraft", "Audio", "example-org/democraft", "org.example.democraft") };
    pub const TEST: App =
        App { msi_publisher: Some("Example Org"), ..App::new("testcraft", "TestCraft", "Video", "example-org/testcraft", "org.example.testcraft") };
    /// Shipped as OldCraft before.
    pub const NEW: App = App {
        former_ids: &["oldcraft"],
        former_names: &["OldCraft"],
        former_bundles: &["org.example.oldcraft"],
        msi_publisher: Some("Example Org"),
        ..App::new("newcraft", "NewCraft", "PDFs", "example-org/newcraft", "org.example.newcraft")
    };
    /// A launcher, described like an app so it updates itself with the same machinery.
    pub const LAUNCHER: App = App {
        msi_publisher: Some("Example Org"),
        ..App::new(
            "example-launcher",
            "Example Launcher",
            "Installs and updates apps",
            "example-org/example-launcher",
            "org.example.example-launcher",
        )
    };

    pub const APPS: [App; 5] = [EXAMPLE, SAMPLE, DEMO, TEST, NEW];

    /// The test app with this id.
    pub fn app(id: &str) -> &'static App {
        static ALL: [App; 6] = [EXAMPLE, SAMPLE, DEMO, TEST, NEW, LAUNCHER];
        ALL.iter().find(|a| a.id == id).unwrap_or_else(|| panic!("no test app {id}"))
    }
}
