//! Reading system package lists (dpkg, rpm, pacman, Flatpak) and matching them to Craft Apps.
//!
//! Pure functions over the tools' text output, so they're tested on every platform; only Linux
//! runs the tools (`detect::linux`).

use crate::{App, InstallSource, version};

/// One installed package that belongs to a Craft App.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub manager: InstallSource,
    /// The package's name (`examplecraft-bin`) or Flatpak app id.
    pub name: String,
    /// As the package manager reports it (`0.5.0-1`); empty when it doesn't say.
    pub version: String,
}

/// `dpkg-query -W -f '${Package}|${db:Status-Status}|${Version}\n'`: installed packages only.
pub fn parse_dpkg(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split('|');
            match (f.next(), f.next(), f.next()) {
                (Some(name), Some("installed"), Some(ver)) => {
                    let name = name.split_once(':').map(|(n, _)| n).unwrap_or(name);
                    Some((name.to_string(), ver.to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

/// `rpm -qa --qf '%{NAME}|%{VERSION}\n'`.
pub fn parse_rpm(text: &str) -> Vec<(String, String)> {
    text.lines().filter_map(|line| line.split_once('|').map(|(n, v)| (n.to_string(), v.to_string()))).collect()
}

/// `pacman -Q`: `name version` per line.
pub fn parse_pacman(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split_whitespace();
            Some((f.next()?.to_string(), f.next()?.to_string()))
        })
        .collect()
}

/// `flatpak list --app --columns=application,version`: the version column is often empty for
/// bundles built from a release tarball.
pub fn parse_flatpak_list(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split('\t');
            let id = f.next()?.trim();
            (!id.is_empty()).then(|| (id.to_string(), f.next().unwrap_or("").trim().to_string()))
        })
        .collect()
}

/// The `Version:` line of `flatpak info <id>`.
pub fn parse_flatpak_info(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.trim().strip_prefix("Version:").map(|v| v.trim().to_string())).filter(|v| !v.is_empty())
}

/// The package names an app may be installed under with `manager`. On Arch the AUR carries
/// each app three ways: built from source (`examplecraft`), the release binary (`examplecraft-bin`)
/// and the latest commit (`examplecraft-git`). Debian and RPM packages may use either the short stem
/// (`examplecraft`) or a reverse-DNS bundle id, current or former (`org.example.examplecraft`).
pub fn candidate_names(app: &App, manager: InstallSource) -> Vec<String> {
    match manager {
        InstallSource::Flatpak => app.bundle_ids().map(str::to_string).collect(),
        InstallSource::Pacman => {
            app.stems().flat_map(|s| [s.to_string(), format!("{s}-bin"), format!("{s}-git")]).chain(app.bundle_ids().map(str::to_string)).collect()
        }
        InstallSource::Deb | InstallSource::Rpm => app.stems().map(str::to_string).chain(app.bundle_ids().map(str::to_string)).collect(),
        _ => app.stems().map(str::to_string).collect(),
    }
}

/// The package of `app` among `installed`, preferring the manager listed first in `order`.
pub fn find<'a>(app: &App, installed: &'a [Package], order: &[InstallSource]) -> Option<&'a Package> {
    order.iter().find_map(|manager| {
        let names = candidate_names(app, *manager);
        names.iter().find_map(|n| installed.iter().find(|p| p.manager == *manager && p.name == *n))
    })
}

/// A package's version, read the package-manager way (`0.5.0-1` is 0.5.0).
pub fn version_of(p: &Package) -> Option<version::Version> {
    version::parse_package(&p.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkgs(manager: InstallSource, list: Vec<(String, String)>) -> Vec<Package> {
        list.into_iter().map(|(name, version)| Package { manager, name, version }).collect()
    }

    #[test]
    fn reads_each_package_managers_listing() {
        assert_eq!(
            parse_dpkg("examplecraft|installed|0.5.0-1\nexamplecraft:amd64|installed|0.6.0-1\nsamplecraft|config-files|0.3.0-1\n"),
            [("examplecraft".into(), "0.5.0-1".into()), ("examplecraft".into(), "0.6.0-1".into())]
        );
        assert_eq!(parse_rpm("samplecraft|0.3.0\n"), [("samplecraft".into(), "0.3.0".into())]);
        assert_eq!(parse_pacman("examplecraft-bin 0.5.0-1\nlinux 6.11.1.arch1-1\n").len(), 2);
        assert_eq!(
            parse_flatpak_list("org.example.samplecraft\t\norg.example.democraft\t0.3.0\n"),
            [("org.example.samplecraft".into(), String::new()), ("org.example.democraft".into(), "0.3.0".into())]
        );
        let info = "\nSampleCraft - Word processor\n\n          ID: org.example.samplecraft\n     Version: 0.3.0\n      Branch: master\n";
        assert_eq!(parse_flatpak_info(info).as_deref(), Some("0.3.0"));
        assert_eq!(parse_flatpak_info("Version: \n"), None);
    }

    #[test]
    fn candidate_names_cover_bundle_ids_for_deb_rpm_and_pacman() {
        let photo = crate::testing::app("examplecraft");
        let deb_names = candidate_names(photo, InstallSource::Deb);
        assert!(deb_names.contains(&"examplecraft".to_string()));
        assert!(deb_names.contains(&"org.example.examplecraft".to_string()));

        let rpm_names = candidate_names(photo, InstallSource::Rpm);
        assert!(rpm_names.contains(&"examplecraft".to_string()));
        assert!(rpm_names.contains(&"org.example.examplecraft".to_string()));

        let pacman_names = candidate_names(photo, InstallSource::Pacman);
        assert!(pacman_names.contains(&"examplecraft".to_string()));
        assert!(pacman_names.contains(&"examplecraft-bin".to_string()));
        assert!(pacman_names.contains(&"org.example.examplecraft".to_string()));
    }

    #[test]
    fn finds_aur_packages_in_every_flavour() {
        let photo = crate::testing::app("examplecraft");
        let pdf = crate::testing::app("newcraft");
        let installed =
            pkgs(InstallSource::Pacman, parse_pacman("linux 6.11.1.arch1-1\nexamplecraft-git 0.3.0.r47.g47f9306-1\noldcraft-bin 0.2.1-1\n"));
        let hit = find(photo, &installed, &[InstallSource::Pacman]).unwrap();
        assert_eq!(hit.name, "examplecraft-git");
        assert_eq!(version_of(hit).unwrap().to_string(), "0.3.0");
        // NewCraft under its old name.
        assert_eq!(find(pdf, &installed, &[InstallSource::Pacman]).unwrap().name, "oldcraft-bin");
        assert!(find(crate::testing::app("samplecraft"), &installed, &[InstallSource::Pacman]).is_none());
    }

    #[test]
    fn prefers_managers_in_order_and_reads_revisions() {
        let photo = crate::testing::app("examplecraft");
        let mut installed = pkgs(InstallSource::Deb, parse_dpkg("examplecraft|installed|0.5.0-1\n"));
        installed.extend(pkgs(InstallSource::Flatpak, parse_flatpak_list("org.example.examplecraft\t0.4.0\n")));
        let hit = find(photo, &installed, &[InstallSource::Deb, InstallSource::Flatpak]).unwrap();
        assert_eq!(hit.manager, InstallSource::Deb);
        // 0.5.0-1 is the 0.5.0 release, not a pre-release of it.
        assert_eq!(version_of(hit).unwrap().to_string(), "0.5.0");
        assert_eq!(find(photo, &installed, &[InstallSource::Flatpak]).unwrap().version, "0.4.0");
    }
}
