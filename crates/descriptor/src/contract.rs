//! The release contract: what a release must contain before the launcher touches it.
//!
//! - `craft-app.json` and `SHA256SUMS.txt` are attached.
//! - Every package is named `<id>-<version>-<os>-<arch>[-portable].<ext>` with one shared version
//!   that matches the tag.
//! - `SHA256SUMS.txt` lists every package, the descriptor and the icon, and names nothing that
//!   isn't attached.
//! - A release with an `.msi` names its `msi_publisher`.
//!
//! The same check runs in CI (`craft-release-check`), when the launcher lists a release, and before
//! an app's own updater downloads one. A release that breaks it is refused with the reasons, never
//! guessed at.

use crate::{DESCRIPTOR_FILE, Descriptor, SUMS_FILE};

/// The package formats the launcher installs, by suffix.
pub const SUFFIXES: &[&str] = &["-portable.zip", ".tar.gz", ".msi", ".dmg", ".AppImage", ".deb", ".rpm", ".pkg.tar.zst", ".flatpak"];

/// `<os>-<arch>` tokens the launcher understands.
pub const TOKENS: &[&str] = &[
    "windows-x64",
    "windows-x86",
    "windows-arm64",
    "macos-universal",
    "macos-x86_64",
    "macos-aarch64",
    "macos-arm64",
    "linux-x86_64",
    "linux-aarch64",
    "linux-riscv64",
    "freebsd-x86_64",
    "freebsd-aarch64",
];

/// One package, read from its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub token: String,
    pub suffix: &'static str,
}

/// What [`check_release`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Reasons the release is refused. Empty: it keeps the contract.
    pub problems: Vec<String>,
    /// Things worth fixing that don't stop the launcher.
    pub warnings: Vec<String>,
    pub packages: Vec<Package>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }
}

/// Reads `<stem>-<version>-<os>-<arch>[-portable].<ext>` for one of `stems`.
pub fn parse_package(name: &str, stems: &[&str]) -> Option<Package> {
    let (suffix, base) = SUFFIXES.iter().find_map(|s| name.strip_suffix(s).map(|b| (*s, b)))?;
    stems.iter().find_map(|stem| {
        let rest = base.strip_prefix(stem)?.strip_prefix('-')?;
        let mut parts = rest.rsplitn(3, '-');
        let (arch, os, version) = (parts.next()?, parts.next()?, parts.next()?);
        (version.starts_with(|c: char| c.is_ascii_digit()) && semver::Version::parse(version).is_ok()).then(|| Package {
            name: name.to_string(),
            version: version.to_string(),
            token: format!("{os}-{arch}"),
            suffix,
        })
    })
}

/// Checks a release's attached file names and its `SHA256SUMS.txt` text (when fetched) against the
/// contract. `tag` is the release tag (`v1.2.3` or `1.2.3`).
pub fn check_release(d: &Descriptor, tag: &str, assets: &[&str], sums: Option<&str>) -> Report {
    let mut r = Report::default();
    let stems: Vec<&str> = d.stems().collect();
    for name in [DESCRIPTOR_FILE, SUMS_FILE] {
        if !assets.contains(&name) {
            r.problems.push(format!("{name} isn't attached"));
        }
    }
    let mut ignored = Vec::new();
    for name in assets {
        match parse_package(name, &stems) {
            Some(p) => r.packages.push(p),
            None if SUFFIXES.iter().any(|s| name.ends_with(s)) && stems.iter().any(|s| name.starts_with(&format!("{s}-"))) => {
                r.problems.push(format!("{name} looks like a package but isn't named <id>-<version>-<os>-<arch>[-portable].<ext>"));
            }
            None if *name == DESCRIPTOR_FILE || *name == SUMS_FILE || d.icon.as_deref() == Some(*name) => {}
            None => ignored.push(*name),
        }
    }
    if r.packages.is_empty() {
        r.problems.push(format!("no package is named {}-<version>-<os>-<arch>[-portable].<ext>", d.id));
    }
    let want = tag.strip_prefix('v').unwrap_or(tag);
    for p in &r.packages {
        if p.version != want {
            r.problems.push(format!("{} is version {}, but the tag says {want}", p.name, p.version));
        }
        if !TOKENS.contains(&p.token.as_str()) {
            r.problems.push(format!("{}: {} isn't a known <os>-<arch>", p.name, p.token));
        }
    }
    if r.packages.iter().any(|p| p.suffix == ".msi") && d.msi_publisher.is_none() {
        r.problems.push("the release ships an .msi, so craft-app.json must name msi_publisher".into());
    }
    if r.packages.iter().any(|p| p.suffix == ".dmg") && d.apple_team.is_none() {
        r.warnings.push("no apple_team: the launcher won't install the .dmg, only open an app that's already there".into());
    }
    if let Some(icon) = &d.icon
        && !assets.contains(&icon.as_str())
    {
        r.problems.push(format!("the icon {icon} isn't attached"));
    }
    if let Some(text) = sums {
        let listed: Vec<String> = crate::parse_sha256sums(text).into_iter().map(|(_, n)| n).collect();
        let mut must: Vec<&str> = r.packages.iter().map(|p| p.name.as_str()).collect();
        must.push(DESCRIPTOR_FILE);
        must.extend(d.icon.as_deref());
        for name in must {
            if !listed.iter().any(|l| l == name) {
                r.problems.push(format!("{SUMS_FILE} doesn't list {name}"));
            }
        }
        for name in &listed {
            if !assets.contains(&name.as_str()) {
                r.problems.push(format!("{SUMS_FILE} lists {name}, which isn't attached"));
            }
        }
    } else if assets.contains(&SUMS_FILE) {
        r.warnings.push(format!("{SUMS_FILE} wasn't read, so the checksums weren't compared"));
    }
    if !ignored.is_empty() {
        r.warnings.push(format!("ignored (not packages): {}", ignored.join(", ")));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc() -> Descriptor {
        Descriptor::parse(
            br##"{"schema":1,"id":"notecraft","name":"NoteCraft","tagline":"Notes","repo":"o/notecraft","publisher":"O",
                 "bundle_id":"org.o.notecraft","msi_publisher":"O","color":"#112233","icon":"notecraft.png"}"##,
            "o/notecraft",
        )
        .unwrap()
    }

    const GOOD: &[&str] = &[
        "notecraft-1.2.0-windows-x64.msi",
        "notecraft-1.2.0-windows-x64-portable.zip",
        "notecraft-1.2.0-linux-x86_64.AppImage",
        "notecraft-1.2.0-macos-universal.dmg",
        "notecraft.png",
        "craft-app.json",
        "SHA256SUMS.txt",
    ];

    fn sums(names: &[&str]) -> String {
        names.iter().filter(|n| **n != SUMS_FILE).map(|n| format!("{}  {n}\n", "0".repeat(64))).collect()
    }

    #[test]
    fn a_good_release_passes() {
        let r = check_release(&desc(), "v1.2.0", GOOD, Some(&sums(GOOD)));
        assert!(r.ok(), "{:?}", r.problems);
        assert_eq!(r.packages.len(), 4);
        assert_eq!(r.packages[1].suffix, "-portable.zip");
        assert!(r.warnings.iter().any(|w| w.contains("apple_team")));
    }

    #[test]
    fn missing_files_and_drift_fail() {
        let d = desc();
        let no_desc: Vec<&str> = GOOD.iter().copied().filter(|n| *n != DESCRIPTOR_FILE).collect();
        assert!(!check_release(&d, "v1.2.0", &no_desc, None).ok());
        assert!(!check_release(&d, "v1.3.0", GOOD, Some(&sums(GOOD))).ok(), "version must match the tag");
        let short = sums(&GOOD[1..]);
        assert!(check_release(&d, "v1.2.0", GOOD, Some(&short)).problems.iter().any(|p| p.contains("doesn't list notecraft-1.2.0-windows-x64.msi")));
        let extra = sums(GOOD) + &format!("{}  ghost.exe\n", "0".repeat(64));
        assert!(!check_release(&d, "v1.2.0", GOOD, Some(&extra)).ok());
    }

    #[test]
    fn misnamed_packages_are_named_in_the_report() {
        let assets = ["notecraft-setup.msi", "notecraft-1.2.0-amiga-68k.tar.gz", "craft-app.json", "SHA256SUMS.txt", "notecraft.png"];
        let r = check_release(&desc(), "1.2.0", &assets, None);
        assert!(r.problems.iter().any(|p| p.contains("notecraft-setup.msi")), "{:?}", r.problems);
        assert!(r.problems.iter().any(|p| p.contains("amiga-68k")), "{:?}", r.problems);
    }

    #[test]
    fn an_msi_needs_a_publisher() {
        let mut d = desc();
        d.msi_publisher = None;
        assert!(check_release(&d, "v1.2.0", GOOD, Some(&sums(GOOD))).problems.iter().any(|p| p.contains("msi_publisher")));
    }
}
