//! Craft Unofficial Launcher updating itself.
//!
//! Delegated to [`craft_unofficial_update::self_update`], which checks each release against the
//! release contract (`craft-app.json`, `SHA256SUMS.txt`, package names) before downloading it. The
//! launcher only updates when the user asks; it never updates silently.

use std::ffi::OsString;

use artcraft_launcher_core::platform::Platform;
use artcraft_launcher_core::protocol::JobPhase;
use artcraft_launcher_core::release::Release;
use artcraft_launcher_core::version::Version;

use craft_unofficial_update::App;

use crate::http::{Http, Stop};
use crate::paths::Paths;

pub use craft_unofficial_update::self_update::{FINISH_FLAG, Install, PRETEND_VAR, is_self_updating};

/// The launcher, described like an app so it updates itself with the same machinery (asset
/// names `craft-unofficial-launcher-<ver>-…`). No Apple Developer ID team yet, so on macOS it never
/// installs a DMG over itself; it links to the release instead.
pub const LAUNCHER: App = App {
    msi_publisher: Some("Craft Unofficial Launcher contributors"),
    ..App::new(
        "craft-unofficial-launcher",
        "Craft Unofficial Launcher",
        "Installs and updates unofficial Craft-style apps",
        "YalKouassi/craft-unofficial-launcher",
        "io.github.yalkouassi.craft-unofficial-launcher",
    )
};

/// The user agent of the launcher's own update requests: product and version, nothing personal.
const USER_AGENT: &str = concat!("Craft-Unofficial-Launcher/", env!("CARGO_PKG_VERSION"));

/// The version this launcher counts as (see [`PRETEND_VAR`]).
pub fn current_version() -> Option<Version> {
    craft_unofficial_update::self_update::current_version(&LAUNCHER, env!("CARGO_PKG_VERSION"))
}

/// How this copy is installed, if it's one the launcher knows how to replace.
pub fn current() -> Option<Install> {
    craft_unofficial_update::self_update::current(&LAUNCHER)
}

fn to_update_paths(paths: &Paths) -> craft_unofficial_update::Paths {
    craft_unofficial_update::Paths { data: paths.data.clone(), cache: paths.cache.clone() }
}

fn to_update_release(release: &Release) -> craft_unofficial_update::Release {
    craft_unofficial_update::Release {
        tag: release.tag.clone(),
        version: release.version.clone(),
        name: release.name.clone(),
        published_at: release.published_at,
        html_url: release.html_url.clone(),
        notes: release.notes.clone(),
        prerelease: release.prerelease,
        assets: release
            .assets
            .iter()
            .map(|a| craft_unofficial_update::Asset { name: a.name.clone(), size: a.size, url: a.url.clone(), sha256: a.sha256.clone() })
            .collect(),
    }
}

fn to_update_platform(platform: Platform) -> craft_unofficial_update::Platform {
    craft_unofficial_update::Platform {
        os: match platform.os {
            artcraft_launcher_core::Os::Windows => craft_unofficial_update::Os::Windows,
            artcraft_launcher_core::Os::MacOs => craft_unofficial_update::Os::MacOs,
            artcraft_launcher_core::Os::Linux => craft_unofficial_update::Os::Linux,
            artcraft_launcher_core::Os::FreeBsd => craft_unofficial_update::Os::FreeBsd,
        },
        arch: match platform.arch {
            artcraft_launcher_core::Arch::X64 => craft_unofficial_update::Arch::X64,
            artcraft_launcher_core::Arch::Arm64 => craft_unofficial_update::Arch::Arm64,
            artcraft_launcher_core::Arch::X86 => craft_unofficial_update::Arch::X86,
            artcraft_launcher_core::Arch::Riscv64 => craft_unofficial_update::Arch::Riscv64,
        },
    }
}

/// Whether `release` is the version this launcher just updated itself to.
pub fn just_updated_to(paths: &Paths, release: &Release) -> bool {
    let update_paths = to_update_paths(paths);
    let update_release = to_update_release(release);
    craft_unofficial_update::self_update::just_updated_to(&update_paths, &update_release)
}

/// The error of the last update, if it failed, once.
pub fn take_failure(paths: &Paths) -> Option<String> {
    let update_paths = to_update_paths(paths);
    craft_unofficial_update::self_update::take_failure(&update_paths)
}

/// Downloads `release` for `platform`, puts it in place and starts the helper that relaunches
/// it once this process has exited. On success the caller must quit promptly. `last` is called
/// once the download is ready and returns when the launcher may replace itself and restart.
#[allow(clippy::too_many_arguments)]
pub fn update(
    install: &Install,
    release: &Release,
    platform: Platform,
    _http: &Http,
    paths: &Paths,
    stop: &Stop,
    phase: &mut dyn FnMut(JobPhase),
    last: &dyn Fn() -> Result<(), String>,
) -> Result<(), String> {
    let update_release = to_update_release(release);
    let update_platform = to_update_platform(platform);
    let update_paths = to_update_paths(paths);
    let update_http = craft_unofficial_update::Http::new(USER_AGENT)?;
    let mut update_phase = |p: craft_unofficial_update::JobPhase| {
        phase(match p {
            craft_unofficial_update::JobPhase::Downloading { done, total } => JobPhase::Downloading { done, total },
            craft_unofficial_update::JobPhase::Reconnecting { done, total } => JobPhase::Reconnecting { done, total },
            craft_unofficial_update::JobPhase::Waiting => JobPhase::Waiting,
            craft_unofficial_update::JobPhase::Verifying => JobPhase::Verifying,
            craft_unofficial_update::JobPhase::Installing => JobPhase::Installing,
            craft_unofficial_update::JobPhase::Removing => JobPhase::Removing,
        });
    };
    craft_unofficial_update::self_update::update(
        &LAUNCHER,
        install,
        &update_release,
        update_platform,
        &update_http,
        &update_paths,
        stop,
        &mut update_phase,
        last,
        Some(env!("CARGO_PKG_VERSION")),
    )
}

/// Finishes an update when this program was started as the update helper.
pub fn finish(args: &[OsString]) -> Option<Result<(), String>> {
    craft_unofficial_update::self_update::finish(&LAUNCHER, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_development_build_does_not_update_itself() {
        assert_eq!(current(), None);
    }

    #[test]
    fn the_launcher_is_not_artcraft() {
        assert_eq!(LAUNCHER.bundle_id(), "io.github.yalkouassi.craft-unofficial-launcher");
        assert_eq!(LAUNCHER.website(), "https://github.com/YalKouassi/craft-unofficial-launcher");
        assert_eq!(LAUNCHER.apple_team, None);
        assert!(USER_AGENT.starts_with("Craft-Unofficial-Launcher/"));
    }
}
