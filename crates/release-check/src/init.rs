//! `init`: a starter `craft-app.json` that already passes [`Descriptor::check`].

use craft_app_descriptor::{Descriptor, SCHEMA};

/// Builds a starter descriptor for `repo`. Publisher, bundle id, MSI publisher and colour are
/// placeholders derived from the repo owner; the maintainer edits them.
pub fn starter(repo: &str, id: &str, name: &str) -> Result<Descriptor, String> {
    let owner = repo.split('/').next().unwrap_or_default();
    let d = Descriptor {
        schema: SCHEMA,
        id: id.to_string(),
        name: name.to_string(),
        tagline: "What it's for, in a few words".into(),
        repo: repo.to_string(),
        publisher: owner.to_string(),
        upstream: None,
        bundle_id: format!("io.github.{}.{}", bundle_part(owner), bundle_part(id)),
        msi_publisher: Some(owner.to_string()),
        apple_team: None,
        website: None,
        color: "#808080".into(),
        icon: None,
        former_ids: Vec::new(),
        former_names: Vec::new(),
        former_bundles: Vec::new(),
    };
    d.check(repo).map_err(|p| format!("can't make a descriptor from these arguments: {p}"))?;
    Ok(d)
}

/// The note printed (to stderr) beside a starter descriptor.
pub const NOTE: &str = "\
Edit before committing:
  tagline        what the app is for, 1-80 characters
  publisher      who makes it, as the launcher's card shows it
  bundle_id      the reverse-DNS id your macOS bundle, Flatpak and .desktop file really use
  msi_publisher  the Publisher your MSI really registers (remove it if you ship no .msi)
  color          your brand colour, #rrggbb
Optional: website (https://), icon (a .png release asset, at most 512 px and 256 KB),
apple_team (your 10-character Developer ID Team ID), upstream (for a bridge repo).
See docs/craft-app-spec.md.";

/// A bundle-id segment: ASCII letters, digits, `-` and `_`, not starting with a digit.
fn bundle_part(s: &str) -> String {
    let clean: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '_' }).collect();
    match clean.chars().next() {
        Some(c) if c.is_ascii_digit() => format!("_{clean}"),
        None => "app".into(),
        _ => clean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_starter_passes_the_checks() {
        let d = starter("Example-Org/3dcraft", "3dcraft", "3DCraft").unwrap();
        assert_eq!(d.bundle_id, "io.github.example-org._3dcraft");
        assert_eq!(Descriptor::parse(d.to_json().as_bytes(), "example-org/3dcraft").unwrap(), d);
    }

    #[test]
    fn bad_arguments_are_refused_with_reasons() {
        assert!(starter("o/n", "PhotoCraft", "X").unwrap_err().contains("id"));
        assert!(starter("o/n", "photocraft", "X").is_err(), "reserved id");
        assert!(starter("storytold/x", "x", "X").is_err(), "official owner");
        assert!(starter("not-a-repo", "x", "X").is_err());
    }
}
