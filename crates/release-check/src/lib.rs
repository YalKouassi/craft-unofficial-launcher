//! `craft-release-check`: the release contract of `craft-app.json`, checked in CI.
//!
//! The rules themselves live in `craft-app-descriptor` (the same crate the launcher and the
//! app-side updater use), so a release this tool passes is one the launcher accepts. This crate
//! adds what only a maintainer's machine or CI can do: read a local `dist/` folder and recompute
//! every checksum, fetch a published GitHub release, write `SHA256SUMS.txt`, and print a starter
//! descriptor.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod args;
pub mod dir;
pub mod github;
pub mod init;
pub mod outcome;
pub mod sums;

pub use args::{Command, parse_args};
pub use outcome::Outcome;

/// The largest `SHA256SUMS.txt` read, local or downloaded.
pub const MAX_SUMS_BYTES: usize = 1024 * 1024;
