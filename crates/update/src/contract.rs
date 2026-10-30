//! The release contract, enforced before an update downloads anything.
//!
//! The rules live in `craft-app-descriptor` ([`craft_app_descriptor::check_release`]) so the
//! launcher, this updater and the CI checker (`craft-release-check`) all run the same ones: the
//! release attaches `craft-app.json` and `SHA256SUMS.txt`, every package is named
//! `<id>-<version>-<os>-<arch>[-portable].<ext>` with the tag's version, and the sums list every
//! package and nothing that isn't attached. A release that breaks it is refused with the reasons,
//! never guessed at.

use craft_app_descriptor::{SUMS_FILE, check_release};

use crate::app::App;
use crate::http::Http;
use crate::release::{self, Release};

/// The largest `SHA256SUMS.txt` read.
const MAX_SUMS_BYTES: usize = 256 * 1024;

/// Fetches `release`'s `SHA256SUMS.txt` and checks the release against `app`'s contract. Returns
/// the parsed sums (`(sha256, file name)`), which then vouch for the download, so the list is
/// fetched only once.
pub fn check(app: &App, release: &Release, http: &Http) -> Result<Vec<(String, String)>, String> {
    let text = match release.sums_asset() {
        Some(sums) => {
            let bytes = http
                .get_bytes(&sums.url, MAX_SUMS_BYTES)
                .map_err(|e| format!("couldn't fetch {SUMS_FILE} of {} {} ({e}). Try again", app.name, release.version_label()))?
                .ok_or_else(|| format!("{} {} lists a {SUMS_FILE} that isn't there, so it wasn't downloaded", app.name, release.version_label()))?;
            Some(String::from_utf8(bytes).map_err(|_| format!("{SUMS_FILE} of {} {} isn't text", app.name, release.version_label()))?)
        }
        None => None,
    };
    check_text(app, release, text.as_deref())?;
    Ok(text.as_deref().map(release::parse_sha256sums).unwrap_or_default())
}

/// Checks `release`'s attached file names and its `SHA256SUMS.txt` text (`None`: not attached or
/// not read) against `app`'s contract. Pure, so it is what the tests exercise.
pub fn check_text(app: &App, release: &Release, sums: Option<&str>) -> Result<(), String> {
    let descriptor = app.contract_descriptor();
    let names: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
    let report = check_release(&descriptor, &release.tag, &names, sums);
    for warning in &report.warnings {
        log::warn!("{} {}: {warning}", app.name, release.version_label());
    }
    if report.ok() {
        return Ok(());
    }
    Err(format!(
        "{} {} doesn't follow the release contract, so it wasn't downloaded: {}. Report it at {}",
        app.name,
        release.version_label(),
        report.problems.join("; "),
        app.releases_url()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::Asset;
    use crate::testing::EXAMPLE;

    fn release(names: &[&str]) -> Release {
        Release {
            tag: "v1.2.0".into(),
            version: crate::version::parse("1.2.0"),
            assets: names.iter().map(|n| Asset { name: (*n).into(), size: 1, url: String::new(), sha256: None }).collect(),
            ..Release::default()
        }
    }

    fn sums(names: &[&str]) -> String {
        names.iter().map(|n| format!("{}  {n}\n", "a".repeat(64))).collect()
    }

    const GOOD: [&str; 4] = ["examplecraft-1.2.0-windows-x64.msi", "examplecraft-1.2.0-linux-x86_64.AppImage", "craft-app.json", "SHA256SUMS.txt"];

    #[test]
    fn a_release_that_keeps_the_contract_passes() {
        let r = release(&GOOD);
        assert_eq!(check_text(&EXAMPLE, &r, Some(&sums(&GOOD[..3]))), Ok(()));
    }

    #[test]
    fn a_release_without_its_descriptor_or_sums_is_refused() {
        let names = ["examplecraft-1.2.0-linux-x86_64.AppImage"];
        let err = check_text(&EXAMPLE, &release(&names), None).unwrap_err();
        assert!(err.contains("craft-app.json isn't attached") && err.contains("SHA256SUMS.txt isn't attached"), "{err}");
        assert!(err.contains("release contract"), "{err}");
    }

    #[test]
    fn sums_that_miss_a_package_or_name_a_stranger_are_refused() {
        let r = release(&GOOD);
        let err = check_text(&EXAMPLE, &r, Some(&sums(&["examplecraft-1.2.0-windows-x64.msi", "craft-app.json", "evil.exe"]))).unwrap_err();
        assert!(err.contains("doesn't list examplecraft-1.2.0-linux-x86_64.AppImage"), "{err}");
        assert!(err.contains("lists evil.exe"), "{err}");
    }

    #[test]
    fn a_package_from_another_version_is_refused() {
        let names = ["examplecraft-1.1.0-linux-x86_64.AppImage", "craft-app.json", "SHA256SUMS.txt"];
        let err = check_text(&EXAMPLE, &release(&names), Some(&sums(&names[..2]))).unwrap_err();
        assert!(err.contains("the tag says 1.2.0"), "{err}");
    }

    #[test]
    fn an_msi_needs_an_msi_publisher() {
        let bare = App { msi_publisher: None, ..EXAMPLE };
        let err = check_text(&bare, &release(&GOOD), Some(&sums(&GOOD[..3]))).unwrap_err();
        assert!(err.contains("msi_publisher"), "{err}");
    }

    /// A release with no `SHA256SUMS.txt` is refused before the network is touched.
    #[test]
    fn a_missing_sums_file_is_refused_without_fetching() {
        let http = Http::new(crate::TEST_AGENT).unwrap();
        let err = check(&EXAMPLE, &release(&["examplecraft-1.2.0-linux-x86_64.AppImage", "craft-app.json"]), &http).unwrap_err();
        assert!(err.contains("SHA256SUMS.txt isn't attached"), "{err}");
    }
}
