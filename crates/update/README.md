# craft-unofficial-update

Release checking, downloading, installing and self-updating for **unofficial** Craft-style apps,
and for Craft Unofficial Launcher itself. An app describes itself with the `craft-app.json` it
ships; nothing in this crate defaults to ArtCraft's identity (bundle id, MSI publisher, Apple
Developer ID team or website), and every update is checked against the release contract before
anything is downloaded.

Unofficial apps are not made, reviewed, endorsed or supported by ArtCraft or the craft-launcher
maintainers.

## Adopting it in your app

1. **Add `craft-app.json`** at the root of your repo. Start from
   `craft-release-check init --repo <owner/name> --id <id> --name <Name>` and edit it; the schema
   is in [`docs/craft-app-spec.md`](../../docs/craft-app-spec.md). It must name your own repo,
   bundle id (`org.example.yourapp`), and, if you ship an `.msi`, the `msi_publisher` your
   installer registers. Leave `apple_team` out unless your `.app` is signed with your own Developer
   ID: without it a DMG is never installed, only opened.

2. **Depend on the crate by git**, pinned to a commit:

   ```toml
   [dependencies]
   craft-unofficial-update = { git = "https://github.com/YalKouassi/craft-unofficial-launcher", rev = "<commit-sha>", features = ["egui"] }
   ```

   The `egui` feature adds the startup prompt and the [`Updater`](src/updater.rs) that drives it.

3. **Build your `App` from the embedded descriptor**, once per process, and **create an
   `Updater`**:

   ```rust
   use craft_unofficial_update::App;
   use craft_unofficial_update::updater::{Action, Updater};

   fn app() -> &'static App {
       static APP: std::sync::OnceLock<Option<&'static App>> = std::sync::OnceLock::new();
       // `from_embedded` checks the descriptor exactly as the launcher does, and must name this repo.
       let app = *APP.get_or_init(|| App::from_embedded(include_str!("../craft-app.json"), "example-org/examplecraft").ok());
       app.unwrap_or(&FALLBACK)
   }

   // Used only if the descriptor fails its checks (CI catches that before a release).
   const FALLBACK: App = App::new("examplecraft", "ExampleCraft", "What it's for", "example-org/examplecraft", "org.example.examplecraft");

   fn main() {
       // When started as the update helper, finish the update and exit.
       let args: Vec<_> = std::env::args_os().skip(1).collect();
       if let Some(done) = craft_unofficial_update::self_update::finish(app(), &args) {
           std::process::exit(i32::from(done.is_err()));
       }
       let mut updater = Updater::new(app(), env!("CARGO_PKG_VERSION"));
       // Each frame:
       //     updater.check_at_start(ctx);
       //     updater.tick(ctx);
       //     match updater.show_if(ctx, !has_unsaved_work) {
       //         Action::Close => /* close through your normal close path */,
       //         Action::OpenUrl(url) => /* open it in the browser */,
       //         Action::None => {}
       //     }
   }
   ```

   Updates are never silent: the `Updater` shows a prompt, and only installs when the user picks
   **Update and restart**. Before it downloads anything it fetches the release's `SHA256SUMS.txt`
   and runs the release contract (`craft_unofficial_update::contract`); a release that breaks it
   is refused with the reasons.

4. **Run `craft-release-check` in CI** before uploading a release, so every release keeps the
   contract the updater and the launcher enforce:

   ```yaml
   - uses: YalKouassi/craft-unofficial-launcher/.github/actions/craft-release-check@<commit-sha>
     with:
       dist: dist   # the packages, the icon, craft-app.json and SHA256SUMS.txt
   ```

   Locally: `craft-release-check sums dist/` then
   `craft-release-check dir dist/ --repo <owner/name> --tag v1.2.0`.

## The release contract, in short

- Attach `craft-app.json` and `SHA256SUMS.txt` to every release.
- Name every package `<id>-<version>-<os>-<arch>[-portable].<ext>`, with the tag's version.
- `SHA256SUMS.txt` lists every package, the descriptor and the icon, and nothing that isn't
  attached.
- A release with an `.msi` needs `msi_publisher` in `craft-app.json`.

## Requests

Requests made by the `Updater` carry the generic user agent `Craft-Unofficial-Update/<version>`
([`USER_AGENT`](src/lib.rs)): no app, person or machine is named. Downloads only come from the
app's own `https://github.com/<repo>/releases`.

## Notice

Derived from the `crates/update` crate of
[storytold/craft-libs](https://github.com/storytold/craft-libs) at rev
`92d392cff132eaf53e13afff77f38b5c11b73aa0`, Copyright (c) 2026 ArtCraft Team and the craft-libs
contributors, licensed under the MIT License or the Apache License, Version 2.0, at your option.
This fork is modified: the official app catalog and ArtCraft's identity were removed, the identity
moved into `App` (built from `craft-app.json`), and the release contract is enforced before
updates. It is distributed under the same terms (`MIT OR Apache-2.0`).
