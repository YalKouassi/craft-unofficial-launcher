# AGENTS.md: guide for AI agents and contributors

Craft Unofficial Launcher installs, updates and launches **unofficial Craft-style apps**: apps
made outside ArtCraft that describe themselves with a `craft-app.json` attached to their GitHub
releases ([docs/craft-app-spec.md](docs/craft-app-spec.md)). It is an independent fork of
[storytold/craft-launcher](https://github.com/storytold/craft-launcher) (MIT OR Apache-2.0) and
implements the proposal in [storytold/craft-launcher#989](https://github.com/storytold/craft-launcher/issues/989).
It is **not** an ArtCraft product and **never lists official ArtCraft apps**.

It is written in **Rust only**: the desktop app is native egui/eframe, and the web build is the same
Rust compiled to WebAssembly. **No Tauri, Electron or webview shells**, ever.

## 0. Names

- Product name in user-facing text (UI, window titles, About, tray, notifications, installers,
  release names, docs): **Craft Unofficial Launcher**. Never "ArtCraft Launcher", never "Craft App
  Suite" for what this launcher lists. Call those apps "unofficial apps".
- Machine names: binary and release-asset stem `craft-unofficial-launcher`; app id
  `io.github.yalkouassi.craft-unofficial-launcher`; GitHub repo `YalKouassi/craft-unofficial-launcher`.
- The workspace crates keep upstream's package names (`artcraft-launcher-core`, `-native`,
  `-ui-egui`, the `apps/artcraft-launcher*` shells) so that merging upstream stays practical. The new
  crates are `craft-app-descriptor`, `craft-release-check` and `craft-unofficial-update`.
- Everything this launcher stores on disk, registers with the OS or listens on (data folders,
  single-instance socket, login item, MSI UpgradeCode, registry keys, URL handlers, env vars
  `CRAFT_UNOFFICIAL_LAUNCHER_*`) must differ from the official launcher's, so the two can be
  installed side by side without touching each other's state.

## 1. Workspace map

```text
crates/
  descriptor     craft-app-descriptor     craft-app.json schema 1, hostile-input checks, the reserved
                                          ArtCraft namespace, the release contract, the community index.
                                          Pure and wasm-safe. The ONE place these rules live.
  release-check  craft-release-check      CLI for app maintainers' CI (dir | github | sums | init)
  update         craft-unofficial-update  the app-side updater: a fork of craft-libs' craft-update with
                                          per-app identity (bundle id, MSI publisher, Apple team)
  core           artcraft-launcher-core   the runtime catalog (catalog::apps / set_apps), versions,
                                          GitHub release parsing, asset choice. Pure, wasm-safe
  native         artcraft-launcher-native HTTP, the app registry (registry.rs), detection, install
  ui-egui        artcraft-launcher-ui-egui the shared egui UI (desktop and web)
apps/            desktop shell (eframe, tray, notifications) and the web build
index/apps.json  the community index (repo names only)
xtask/           cargo xtask ci | wasm | version | ico
packaging/       macOS, Windows, Linux and web packaging scripts
```

Layering: `descriptor` depends on no workspace crate. `core` depends only on `descriptor` and must
build for `wasm32-unknown-unknown`. `native` may use `core`. `ui-egui` may use `core` (and `native`
only behind `cfg(not(target_arch = "wasm32"))`). Nothing below the apps opens windows, tray icons
or notifications.

### How an app gets into the catalog

The catalog is data. `native::registry` keeps the user's list (`<data>/catalog/apps.json`). Adding
`owner/repo` fetches the latest release, `SHA256SUMS.txt`, `craft-app.json` and the icon, checks the
descriptor and icon against the sums, runs `craft_app_descriptor::check_release`, pins the identity
(id, bundle id, MSI publisher, Apple team) and calls `core::catalog::set_apps`. Every check
refreshes the descriptors. A descriptor that fails, or whose identity drifts from the pin, is
refused and the pinned entry stays.

`core::sample` is upstream's suite with explicit identities. It exists **only for tests**
(`cfg(test)` or the `sample-catalog` feature, enabled only in dev-dependencies) so inherited
install and detection tests keep running. Release builds start with an empty catalog.

## 2. Golden rules

### The trust model (outranks feature work)

- **A descriptor is hostile input.** Every field is checked in `crates/descriptor`; never relax a
  check elsewhere. Size caps: descriptor 16 KB, icon 256 KB and 512 px, index 64 KB.
- **Nothing from an app runs inside the launcher.** The launcher reads JSON, decodes a PNG, and
  downloads packages that the release's `SHA256SUMS.txt` vouches for. No plugin engines, scripts or
  webviews.
- **ArtCraft's namespace is reserved** (`descriptor::reserved`): `ai.storyteller.*`, Learning
  Machines LLC, ArtCraft's Apple team, the `storytold` owner, the official app ids and names. Never
  remove an entry. Add one when ArtCraft ships a new app.
- **Downloads come only from the app's pinned repo** (`https://github.com/<repo>/releases/…` and
  GitHub's asset CDN), plus a developer's loopback mirror behind the `local-mirror` feature.
- **Identity is pinned** when the user adds an app; drift is refused, never auto-accepted.
- **No silent installs or updates.** Nothing elevated without telling the user; removing an app from
  the list never deletes its files.
- **Agents never add an app to `index/apps.json`, or approve one, on their own.** Adding an index
  entry is the maintainer's call. The same goes for loosening any rule in this section.
- **The launcher never contacts ArtCraft's servers** (the upstream release feed is switched off:
  `backend::check_plan`).

### The disclaimer

Every place that lists or adds unofficial apps shows (or links to) the notice that they are not
created, verified, monitored or supported by ArtCraft or by this launcher's maintainers, that no
liability is accepted, and that support goes to each app's own repository
(`ui-egui/src/unofficial.rs`). Don't remove it or shorten it into something vaguer.

### Never crash

A bad network response, a malformed release, a half-downloaded file, a full disk or a corrupt
settings file must produce an error the user can act on, never a panic.

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!` or
  `unimplemented!`. Return the crate's error type and propagate with `?`.
- Every production crate root carries
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`.
- **Everything from the network is hostile.** Use `get()` instead of indexing, checked arithmetic
  for sizes and offsets, and cap downloads and JSON sizes.

### Look and feel

Upstream's design language stays: hard square corners, 1 px hairlines instead of shadows, Archivo
for headings and app names, Inter for body text, upper-case Geist Mono labels and buttons, colours
by token name (`theme::Tokens`). Build new UI from `widgets.rs`; never hard-code a colour.
Animations follow the OS's reduced-motion setting (`crates/ui-egui/src/motion.rs`).

ArtCraft's name, logo and "A" mark are trademarks and are **not** used anywhere in this project.
The launcher's own icon is original (see `ATTRIBUTION.md`).

### Languages

Every user-facing string goes through `crates/ui-egui/src/i18n` (`tl!("…")` for literals, `fmt`
for `{placeholders}`, `tn` for counts). English in the code is the lookup key. Adding a string means
adding its row to every catalog marked `complete_menus` (the `*.tsv` files), which the i18n tests
check. Translate from the meaning of the English text, never through another catalog. An app's own
name, tagline and publisher come from its descriptor and are shown as written, untranslated.

### Other rules

1. **Assets need attribution.** Every bundled asset gets a row in `ATTRIBUTION.md` (path, title,
   author, source, license) in the same change.
2. **Never break wasm.** `cargo xtask wasm` must pass. File-system, process and OS code is
   `cfg(not(target_arch = "wasm32"))`.
3. **Tests are the gate.** Descriptor and contract rules get unit tests in `crates/descriptor`,
   including the hostile cases. Release parsing and asset picking in `core` use real GitHub release
   JSON fixtures.
4. When fetching anything, use a generic `Craft-Unofficial-Launcher/<version>` User-Agent and never
   put a person's name, email or other personal details in requests.
5. **Time goes through `jiff`.** Never hand-roll calendar conversion.
6. **Merging upstream:** `git fetch upstream && git merge upstream/main`. Expect conflicts in
   `catalog.rs` (ours is the runtime registry; theirs is the compiled suite: keep ours), in
   branding, and in `crates/native/src/self_update.rs`. Never re-introduce the official catalog,
   the ArtCraft release feed or the ArtCraft brand assets.

## 3. Before you finish a task

```sh
cargo xtask ci              # fmt --check, clippy -D warnings, test, wasm check
```

Or piecemeal: `cargo test -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
`cargo xtask wasm`.

## 4. Releasing

The version lives only in `[workspace.package] version` in the root `Cargo.toml`. Release assets are
named `craft-unofficial-launcher-<ver>-<os>-<arch>[-portable].<ext>` with `SHA256SUMS.txt`, the same
contract the launcher asks of the apps. The launcher's self-update relies on these names.

## 5. Parallel agents

- Use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`) and edit only the files you own.
- Keep every `Cargo.toml` valid at all times: the `crates/*` and `apps/*` globs mean one broken
  manifest breaks everyone's build.
- If someone else's in-progress edit breaks the build, wait and retry; don't fix their files.
