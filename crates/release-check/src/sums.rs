//! SHA-256 of local files, and `SHA256SUMS.txt` written and checked against them.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use craft_app_descriptor::{SUMS_FILE, parse_sha256sums, valid_file_name};
use sha2::{Digest, Sha256};

/// Lower-case hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of a byte slice, as lower-case hex.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 of a file, streamed (release packages can be hundreds of MB).
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

/// Reads at most `max` bytes of a file, refusing a larger one rather than truncating it.
pub fn read_capped(path: &Path, max: usize) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take((max as u64).saturating_add(1)).read_to_end(&mut bytes).map_err(|e| format!("can't read {}: {e}", path.display()))?;
    if bytes.len() > max {
        return Err(format!("{} is larger than {max} bytes", path.display()));
    }
    Ok(bytes)
}

/// The regular files directly in `dir`, by name, sorted. Sub-folders are skipped; a name that
/// isn't UTF-8 is an error (it couldn't be a release asset name).
pub fn list_files(dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("can't read the folder {}: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("can't read the folder {}: {e}", dir.display()))?;
        let is_file = entry.file_type().map_err(|e| format!("can't read {}: {e}", entry.path().display()))?.is_file();
        if !is_file {
            continue;
        }
        let name = entry.file_name().into_string().map_err(|n| format!("{} isn't a UTF-8 file name", n.to_string_lossy()))?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// The `sha256sum`-format text for every file in `dir` except `SHA256SUMS.txt` itself and dot
/// files (which can't be release assets).
pub fn sums_text(dir: &Path) -> Result<String, String> {
    let mut out = String::new();
    for name in list_files(dir)? {
        if name == SUMS_FILE || name.starts_with('.') {
            continue;
        }
        if !valid_file_name(&name) {
            return Err(format!("{name:?} can't be a release asset name"));
        }
        let hash = sha256_file(&dir.join(&name)).map_err(|e| format!("can't read {name}: {e}"))?;
        out += &format!("{hash}  {name}\n");
    }
    Ok(out)
}

/// Writes `dir/SHA256SUMS.txt`. Returns how many files it lists.
pub fn write_sums(dir: &Path) -> Result<usize, String> {
    let text = sums_text(dir)?;
    let path = dir.join(SUMS_FILE);
    std::fs::write(&path, &text).map_err(|e| format!("can't write {}: {e}", path.display()))?;
    Ok(text.lines().count())
}

/// Recomputes every checksum `sums` lists for a file in `dir`, returning a problem for each that
/// doesn't match. Names it lists that aren't in the folder are left to the contract check, which
/// reports them as not attached.
pub fn verify(dir: &Path, sums: &str, present: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for (want, name) in parse_sha256sums(sums) {
        if !present.contains(&name) {
            continue;
        }
        match sha256_file(&dir.join(&name)) {
            Ok(got) if got == want => {}
            Ok(got) => problems.push(format!("{name}: its SHA-256 is {got}, but {SUMS_FILE} says {want}")),
            Err(e) => problems.push(format!("can't read {name} to check its SHA-256: {e}")),
        }
    }
    problems
}

/// Lines of `sums` that aren't `<sha256>  <file name>` (and so are silently ignored by readers).
pub fn malformed_lines(sums: &str) -> usize {
    let good = parse_sha256sums(sums).len();
    sums.lines().filter(|l| !l.trim().is_empty()).count().saturating_sub(good)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_sha256() {
        assert_eq!(sha256_bytes(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn sums_are_written_sorted_and_checked() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("b.tar.gz"), b"bee").unwrap();
        std::fs::write(tmp.path().join("a.msi"), b"ay").unwrap();
        std::fs::write(tmp.path().join(".DS_Store"), b"junk").unwrap();
        std::fs::create_dir(tmp.path().join("sub")).unwrap();
        assert_eq!(write_sums(tmp.path()).unwrap(), 2);
        let text = std::fs::read_to_string(tmp.path().join(SUMS_FILE)).unwrap();
        assert_eq!(text, format!("{}  a.msi\n{}  b.tar.gz\n", sha256_bytes(b"ay"), sha256_bytes(b"bee")));
        // Writing again doesn't list the sums file itself.
        assert_eq!(write_sums(tmp.path()).unwrap(), 2);

        let present = list_files(tmp.path()).unwrap();
        assert_eq!(present, [".DS_Store", "SHA256SUMS.txt", "a.msi", "b.tar.gz"]);
        assert!(verify(tmp.path(), &text, &present).is_empty());
        std::fs::write(tmp.path().join("a.msi"), b"tampered").unwrap();
        let problems = verify(tmp.path(), &text, &present);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("a.msi: its SHA-256 is"), "{problems:?}");
    }

    #[test]
    fn malformed_lines_are_counted() {
        let h = "0".repeat(64);
        assert_eq!(malformed_lines(&format!("{h}  a.msi\n\nnonsense\n{h}  ../b\n")), 2);
    }

    #[test]
    fn oversized_files_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("x");
        std::fs::write(&p, [0u8; 10]).unwrap();
        assert_eq!(read_capped(&p, 10).unwrap().len(), 10);
        assert!(read_capped(&p, 9).is_err());
    }
}
