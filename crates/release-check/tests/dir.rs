//! `dir` end to end on a fake release folder.

use std::path::Path;

use craft_release_check::{dir::check_dir, sums};

const DESC: &str = r##"{
  "schema": 1,
  "id": "notecraft",
  "name": "NoteCraft",
  "tagline": "Notes",
  "repo": "example-org/notecraft",
  "publisher": "Example Org",
  "bundle_id": "org.example.notecraft",
  "msi_publisher": "Example Org",
  "color": "#112233",
  "icon": "notecraft-128.png"
}
"##;

fn png(edge: u32) -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    png.extend(edge.to_be_bytes());
    png.extend(edge.to_be_bytes());
    png.extend([8, 6, 0, 0, 0]);
    png
}

fn fake_release(dir: &Path) {
    std::fs::write(dir.join("craft-app.json"), DESC).unwrap();
    std::fs::write(dir.join("notecraft-128.png"), png(128)).unwrap();
    std::fs::write(dir.join("notecraft-1.2.0-windows-x64.msi"), b"msi").unwrap();
    std::fs::write(dir.join("notecraft-1.2.0-windows-x64-portable.zip"), b"zip").unwrap();
    std::fs::write(dir.join("notecraft-1.2.0-linux-x86_64.tar.gz"), b"tgz").unwrap();
    sums::write_sums(dir).unwrap();
}

#[test]
fn a_good_release_passes() {
    let tmp = tempfile::tempdir().unwrap();
    fake_release(tmp.path());
    let o = check_dir(tmp.path(), "example-org/notecraft", "v1.2.0").unwrap();
    assert!(o.ok, "{:?}", o.problems);
    assert_eq!(o.packages.len(), 3);
}

#[test]
fn a_wrong_checksum_fails() {
    let tmp = tempfile::tempdir().unwrap();
    fake_release(tmp.path());
    std::fs::write(tmp.path().join("notecraft-1.2.0-linux-x86_64.tar.gz"), b"rebuilt after the sums were written").unwrap();
    let o = check_dir(tmp.path(), "example-org/notecraft", "v1.2.0").unwrap();
    assert!(!o.ok);
    assert_eq!(o.problems.len(), 1, "{:?}", o.problems);
    assert!(o.problems[0].starts_with("notecraft-1.2.0-linux-x86_64.tar.gz: its SHA-256 is"), "{:?}", o.problems);
}

#[test]
fn contract_and_icon_problems_are_reported_together() {
    let tmp = tempfile::tempdir().unwrap();
    fake_release(tmp.path());
    std::fs::write(tmp.path().join("notecraft-128.png"), png(4096)).unwrap();
    std::fs::write(tmp.path().join("notecraft-1.3.0-linux-aarch64.deb"), b"deb").unwrap();
    sums::write_sums(tmp.path()).unwrap();
    let o = check_dir(tmp.path(), "example-org/notecraft", "v1.2.0").unwrap();
    assert!(o.problems.iter().any(|p| p.starts_with("notecraft-128.png:")), "{:?}", o.problems);
    assert!(o.problems.iter().any(|p| p.contains("but the tag says 1.2.0")), "{:?}", o.problems);
}

#[test]
fn a_descriptor_for_another_repo_fails() {
    let tmp = tempfile::tempdir().unwrap();
    fake_release(tmp.path());
    let o = check_dir(tmp.path(), "someone-else/notecraft", "v1.2.0").unwrap();
    assert!(o.problems.iter().any(|p| p.contains("doesn't match")), "{:?}", o.problems);
}

#[test]
fn a_missing_folder_is_an_error_not_a_report() {
    assert!(check_dir(Path::new("this/does/not/exist"), "o/n", "v1").is_err());
}
