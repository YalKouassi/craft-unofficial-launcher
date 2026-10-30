//! `dir`: a local release folder, checked before anything is uploaded.

use std::path::Path;

use craft_app_descriptor::{DESCRIPTOR_FILE, Descriptor, MAX_DESCRIPTOR_BYTES, MAX_ICON_BYTES, SUMS_FILE, check_icon, check_release};

use crate::outcome::Outcome;
use crate::{MAX_SUMS_BYTES, sums};

/// Checks `dist` as the release `tag` of `repo`. `Err` means the check couldn't run at all (the
/// folder is unreadable); everything wrong with the release itself is in the [`Outcome`].
pub fn check_dir(dist: &Path, repo: &str, tag: &str) -> Result<Outcome, String> {
    let files = sums::list_files(dist)?;
    let has = |name: &str| files.iter().any(|f| f == name);
    let mut out = Outcome::default();

    // The descriptor's own size cap is part of the contract: read one byte past it so
    // Descriptor::parse reports an oversized file as such.
    let descriptor = if has(DESCRIPTOR_FILE) {
        match sums::read_capped(&dist.join(DESCRIPTOR_FILE), MAX_DESCRIPTOR_BYTES.saturating_add(1)) {
            Ok(bytes) => match Descriptor::parse(&bytes, repo) {
                Ok(d) => Some(d),
                Err(p) => {
                    out.problems.extend(p.0);
                    None
                }
            },
            Err(e) => {
                out.problems.push(e);
                None
            }
        }
    } else {
        out.problems.push(format!("{DESCRIPTOR_FILE} isn't in {}", dist.display()));
        None
    };

    let sums_text = if has(SUMS_FILE) {
        match sums::read_capped(&dist.join(SUMS_FILE), MAX_SUMS_BYTES)
            .and_then(|b| String::from_utf8(b).map_err(|_| format!("{SUMS_FILE} isn't UTF-8 text")))
        {
            Ok(text) => Some(text),
            Err(e) => {
                out.problems.push(e);
                None
            }
        }
    } else {
        None
    };
    if let Some(text) = &sums_text {
        out.problems.extend(sums::verify(dist, text, &files));
        match sums::malformed_lines(text) {
            0 => {}
            n => out.warnings.push(format!("{SUMS_FILE} has {n} line(s) that aren't `<sha256>  <file name>`; the launcher ignores them")),
        }
    }

    let Some(d) = descriptor else {
        // Without a descriptor there are no ids to read package names with; say so once.
        out.problems.push("the release contract wasn't checked, because craft-app.json couldn't be read".into());
        return Ok(out.finish());
    };

    if let Some(icon) = d.icon.as_deref().filter(|i| has(i)) {
        match sums::read_capped(&dist.join(icon), MAX_ICON_BYTES.saturating_add(1)) {
            Ok(bytes) => {
                if let Err(e) = check_icon(&bytes) {
                    out.problems.push(format!("{icon}: {e}"));
                }
            }
            Err(e) => out.problems.push(e),
        }
    }

    let assets: Vec<&str> = files.iter().map(String::as_str).filter(|n| !n.starts_with('.')).collect();
    out.absorb(check_release(&d, tag, &assets, sums_text.as_deref()));
    Ok(out.finish())
}
