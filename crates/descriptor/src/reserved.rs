//! ArtCraft's namespace, which no unofficial descriptor may claim.
//!
//! An unofficial app that borrowed an official app's id, name, bundle id, Windows publisher, Apple
//! team or GitHub owner could pass itself off as ArtCraft's, or have the launcher "update" an
//! official install with its own build. Each of these is refused at parse time.

/// The official Craft App Suite's ids, ArtCraft Launcher's, and this launcher's own.
pub const IDS: &[&str] = &[
    "photocraft",
    "vectorcraft",
    "filmcraft",
    "lightcraft",
    "pdfcraft",
    "printcraft",
    "effectcraft",
    "designcraft",
    "figcraft",
    "wordcraft",
    "soundcraft",
    "deckcraft",
    "cadcraft",
    "gridcraft",
    "artcraft",
    "artcraft-launcher",
    "craft-launcher",
    "craft-unofficial-launcher",
];

/// Bundle-id prefixes (lowercase) that belong to ArtCraft.
pub const BUNDLE_PREFIXES: &[&str] = &["ai.storyteller."];

/// Windows Installer publishers (lowercase) that belong to ArtCraft.
pub const MSI_PUBLISHERS: &[&str] = &["learning machines llc", "learning machines", "artcraft", "storyteller"];

/// Apple Developer ID teams that belong to ArtCraft.
pub const APPLE_TEAMS: &[&str] = &["DJ6XS33FX8"];

/// GitHub owners (lowercase) that publish official apps.
pub const GITHUB_OWNERS: &[&str] = &["storytold"];

/// Publisher names (lowercase) an unofficial app may not show on its card.
pub const PUBLISHERS: &[&str] = &["artcraft", "storyteller", "storyteller.ai", "learning machines llc"];

/// Whether `id` (or the lowercase form of a display `name`) is reserved.
pub fn id(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    IDS.contains(&lower.as_str())
}

/// This launcher's own id and the one repository allowed to publish under it: the launcher's
/// releases carry a `craft-app.json` too, so its self-update and `craft-release-check` run the same
/// contract. Nobody else may claim the id, and the launcher never lists itself (`native::registry`).
pub const LAUNCHER_ID: &str = "craft-unofficial-launcher";
pub const LAUNCHER_REPO: &str = "YalKouassi/craft-unofficial-launcher";

/// Whether a descriptor read from `repo` may not use `id`: [`id`], except the launcher's own id in
/// the launcher's own repository.
pub fn id_for_repo(id: &str, repo: &str) -> bool {
    self::id(id) && !(id == LAUNCHER_ID && repo.eq_ignore_ascii_case(LAUNCHER_REPO))
}

pub fn bundle(bundle: &str) -> bool {
    let lower = bundle.to_ascii_lowercase();
    BUNDLE_PREFIXES.iter().any(|p| lower.starts_with(p) || lower == p.trim_end_matches('.'))
}

pub fn msi_publisher(publisher: &str) -> bool {
    MSI_PUBLISHERS.contains(&publisher.trim().to_ascii_lowercase().as_str())
}

pub fn apple_team(team: &str) -> bool {
    APPLE_TEAMS.iter().any(|t| t.eq_ignore_ascii_case(team.trim()))
}

pub fn publisher(publisher: &str) -> bool {
    PUBLISHERS.contains(&publisher.trim().to_ascii_lowercase().as_str())
}

/// Whether `repo` (`owner/name`) lives under an official owner.
pub fn repo(repo: &str) -> bool {
    repo.split('/').next().is_some_and(|owner| GITHUB_OWNERS.iter().any(|o| o.eq_ignore_ascii_case(owner)))
}

#[cfg(test)]
mod tests {
    #[test]
    fn artcraft_namespace_is_caught_in_any_case() {
        assert!(super::id("PhotoCraft"));
        assert!(super::bundle("AI.Storyteller.notecraft"));
        assert!(!super::bundle("ai.storytellers.notecraft"));
        assert!(super::msi_publisher(" Learning Machines LLC "));
        assert!(super::apple_team("dj6xs33fx8"));
        assert!(super::repo("StoryTold/notecraft"));
        assert!(!super::repo("someone/storytold"));
        assert!(!super::id("notecraft"));
    }

    #[test]
    fn only_the_launchers_repo_may_use_the_launchers_id() {
        assert!(!super::id_for_repo("craft-unofficial-launcher", "YalKouassi/craft-unofficial-launcher"));
        assert!(!super::id_for_repo("craft-unofficial-launcher", "yalkouassi/CRAFT-UNOFFICIAL-LAUNCHER"));
        assert!(super::id_for_repo("craft-unofficial-launcher", "someone/craft-unofficial-launcher"));
        assert!(super::id_for_repo("Craft-Unofficial-Launcher", "YalKouassi/craft-unofficial-launcher"));
        assert!(super::id_for_repo("photocraft", "YalKouassi/craft-unofficial-launcher"));
        assert!(super::id_for_repo("craft-launcher", "YalKouassi/craft-unofficial-launcher"));
    }
}
