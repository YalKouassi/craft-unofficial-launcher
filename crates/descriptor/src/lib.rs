//! `craft-app.json`: how an unofficial Craft-style app describes itself, and the release contract
//! its GitHub releases must keep for Craft Unofficial Launcher to list, install and update it.
//!
//! Everything here is pure (no I/O) and builds for `wasm32-unknown-unknown`, so the launcher, the
//! app-side updater (`craft-unofficial-update`) and the CI checker (`craft-release-check`) all run
//! exactly the same rules.
//!
//! A descriptor is hostile input. It is size-capped, every field is checked, and it may not claim
//! ArtCraft's namespace (see [`reserved`]): the launcher only ever *reads* it, it never runs it.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod contract;
pub mod descriptor;
pub mod index;
pub mod reserved;

pub use contract::{Report, check_release};
pub use descriptor::{Descriptor, Problems};
pub use index::{Index, IndexEntry};

/// The descriptor's file name, both in an app's repo and as a release asset.
pub const DESCRIPTOR_FILE: &str = "craft-app.json";
/// The checksum file every release attaches.
pub const SUMS_FILE: &str = "SHA256SUMS.txt";
/// The newest descriptor schema this crate reads.
pub const SCHEMA: u32 = 1;
/// A descriptor larger than this is refused unread.
pub const MAX_DESCRIPTOR_BYTES: usize = 16 * 1024;
/// An icon larger than this is refused unread.
pub const MAX_ICON_BYTES: usize = 256 * 1024;
/// The largest icon edge accepted, in pixels.
pub const MAX_ICON_EDGE: u32 = 512;

/// Reads `sha256sum` output: `<hex>  <name>` (or `<hex> *<name>`) per line. Malformed lines are
/// skipped; names are bare file names.
pub fn parse_sha256sums(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let (hash, rest) = line.trim().split_once(char::is_whitespace)?;
            let name = rest.trim_start().trim_start_matches('*');
            (hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && valid_file_name(name))
                .then(|| (hash.to_ascii_lowercase(), name.to_string()))
        })
        .collect()
}

/// A bare file name: no paths, no control characters, nothing Windows refuses.
pub fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && !name.starts_with('.')
        && !name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*'))
}

/// Checks that `bytes` is a PNG no larger than [`MAX_ICON_BYTES`] and [`MAX_ICON_EDGE`] square,
/// reading only its header. Returns the width and height.
pub fn check_icon(bytes: &[u8]) -> Result<(u32, u32), String> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() > MAX_ICON_BYTES {
        return Err(format!("the icon is {} bytes; the limit is {MAX_ICON_BYTES}", bytes.len()));
    }
    if !bytes.starts_with(SIGNATURE) || bytes.get(12..16) != Some(b"IHDR") {
        return Err("the icon isn't a PNG".into());
    }
    let be = |at: usize| bytes.get(at..at + 4).and_then(|b| <[u8; 4]>::try_from(b).ok()).map(u32::from_be_bytes);
    let (Some(w), Some(h)) = (be(16), be(20)) else { return Err("the icon's PNG header is cut short".into()) };
    if w == 0 || h == 0 || w > MAX_ICON_EDGE || h > MAX_ICON_EDGE {
        return Err(format!("the icon is {w}×{h}; it must be between 1 and {MAX_ICON_EDGE} pixels on each side"));
    }
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_skip_bad_lines_and_paths() {
        let h = "a".repeat(64);
        let text = format!("{h}  app-1.0.0-linux-x86_64.AppImage\n{h} *craft-app.json\nnot a line\n{h}  ../evil\n");
        let sums = parse_sha256sums(&text);
        assert_eq!(sums.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(), ["app-1.0.0-linux-x86_64.AppImage", "craft-app.json"]);
    }

    #[test]
    fn icons_are_checked_from_the_header() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend(128u32.to_be_bytes());
        png.extend(128u32.to_be_bytes());
        assert_eq!(check_icon(&png), Ok((128, 128)));
        let mut huge = png.clone();
        huge.splice(16..20, 4096u32.to_be_bytes());
        assert!(check_icon(&huge).is_err());
        assert!(check_icon(b"GIF89a").is_err());
        assert!(check_icon(&png[..18]).is_err());
    }
}
