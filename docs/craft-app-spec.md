# `craft-app.json`: schema 1 and the release contract

Craft Unofficial Launcher lists, installs and updates **unofficial** Craft-style apps. It knows
nothing about them in advance: each app describes itself in a small JSON file, `craft-app.json`,
attached to every GitHub release, and each release keeps a fixed contract (file names and
checksums). The launcher reads the descriptor as hostile data, checks the release, and refuses
anything that breaks a rule, with the reasons. It never guesses.

The rules below are the ones in `crates/descriptor` (package `craft-app-descriptor`). The
launcher, the app-side updater and the CI checker `craft-release-check` all run that same code, so
a release the checker passes is one the launcher accepts. If this page and the code ever disagree,
the code wins; please report the difference.

Being listed by the launcher vouches for nothing. ArtCraft and the launcher's maintainers don't
review, endorse or support unofficial apps.

## 1. The descriptor

An example:

```json
{
  "schema": 1,
  "id": "examplecraft",
  "name": "ExampleCraft",
  "tagline": "What it's for, in a few words",
  "repo": "example-org/examplecraft",
  "publisher": "Example Org",
  "upstream": null,
  "bundle_id": "org.example.examplecraft",
  "msi_publisher": "Example Org",
  "apple_team": null,
  "website": "https://example.org/examplecraft",
  "color": "#7a45b8",
  "icon": "examplecraft-128.png"
}
```

General rules:

- The file is UTF-8 JSON of at most **16 KB** (16,384 bytes). A larger file is refused unread.
- **Unknown fields are refused.** A typo is an error, not something silently skipped.
- Optional text fields may be left out or set to `null`. The `former_*` lists may be left out or
  set to `[]` (not `null`).
- Every problem is reported at once, so one run of the checker lists everything to fix.
- "Plain text" below means: the given number of characters (Unicode characters, not bytes), at least
  one, no leading or trailing whitespace, no control characters, and no bidirectional-override
  characters (U+200E, U+200F, U+202A–U+202E, U+2066–U+2069), which can make a name read as
  something it isn't.

### Fields

| Field | Required | Rule |
|---|---|---|
| `schema` | yes | The number `1`. Any other value is refused. |
| `id` | yes | Machine name, also the binary, package and asset stem. 1–40 characters of `a-z`, `0-9` and `-`; no leading or trailing hyphen, no `--`. Must not be a reserved id (section 2). |
| `name` | yes | Display name: plain text, 1–40 characters. Must not equal a reserved id when lower-cased (so `PhotoCraft` is refused). |
| `tagline` | yes | Plain text, 1–80 characters. |
| `repo` | yes | The GitHub `owner/name` the releases are published under. The owner is 1–39 characters of letters, digits and `-`, not starting with `-`. The name is 1–100 characters of letters, digits, `-`, `_` and `.`, not `.` or `..`, not ending in `.git`. It must be the repository the descriptor was read from (compared case-insensitively), and the owner must not be an official one (section 2). |
| `publisher` | yes | Who makes it, as the launcher's card shows it. Plain text, 1–60 characters, not a reserved publisher (section 2). |
| `upstream` | no | For a bridge repo (section 5): the `owner/name` of the project it packages. Same format as `repo`. |
| `bundle_id` | yes | Reverse-DNS id, used as the macOS bundle id, the Flatpak id and the Linux desktop-file id. At most 255 bytes, at least two dot-separated parts; each part is non-empty, uses only ASCII letters, digits, `-` and `_`, and doesn't start with a digit. Must not be in ArtCraft's namespace (section 2). |
| `msi_publisher` | when a release ships an `.msi` | The Publisher your MSI registers (the launcher detects installs by it). Plain text, 1–100 characters, not a reserved MSI publisher. |
| `apple_team` | no | The Apple Developer ID Team ID that signs your `.app`: exactly 10 characters, `A-Z` and `0-9`. Not ArtCraft's team. Without it the launcher never installs your `.dmg` (see section 3). |
| `website` | no | An `https://` link of at most 200 bytes, with a host, no `@` or `\` in the host, and no whitespace, control characters, quotes (`"` or `'`), backtick, `<`, `>`, `\|`, `^`, `{` or `}`. |
| `color` | yes | Brand colour, `#rrggbb` (six hex digits, either case). |
| `icon` | no | The file name of a `.png` release asset (`.png` in any case). A bare file name: 1–200 bytes, not starting with `.`, no control characters and none of `/ \ : < > " \| ? *`. The PNG itself is checked too (section 3). |
| `former_ids` | no | Up to 8 older asset stems the releases may still use. Each follows the `id` rule and isn't reserved. Packages named with a former id are accepted. |
| `former_names` | no | Up to 8 older display names its installers registered. Each is plain text of 1–40 characters and not a reserved id. |
| `former_bundles` | no | Up to 8 older bundle ids it shipped under. Each follows the `bundle_id` rule and isn't in ArtCraft's namespace. |

## 2. ArtCraft's reserved namespace

An unofficial app may not claim anything that identifies an official Craft app, or it could pass
itself off as one, or have the launcher "update" an official install with its own build. All of
these are refused when the descriptor is read (`crates/descriptor/src/reserved.rs`), and compared
case-insensitively:

| What | Reserved |
|---|---|
| `id`, `name` (lower-cased), `former_ids`, `former_names` | `photocraft`, `vectorcraft`, `filmcraft`, `lightcraft`, `pdfcraft`, `printcraft`, `effectcraft`, `designcraft`, `figcraft`, `wordcraft`, `soundcraft`, `deckcraft`, `cadcraft`, `gridcraft`, `artcraft`, `artcraft-launcher`, `craft-launcher`, `craft-unofficial-launcher` |
| `bundle_id`, `former_bundles` | anything starting with `ai.storyteller.`, and `ai.storyteller` itself |
| `msi_publisher` (trimmed) | `Learning Machines LLC`, `Learning Machines`, `ArtCraft`, `Storyteller` |
| `apple_team` | `DJ6XS33FX8` |
| `publisher` (trimmed) | `ArtCraft`, `Storyteller`, `Storyteller.ai`, `Learning Machines LLC` |
| `repo` owner | `storytold` |

## 3. The release contract

Every GitHub release of the app must keep these rules (`crates/descriptor/src/contract.rs`).
"Problem" means the release is refused; "warning" means it is accepted but something is worth
fixing.

### Required files

- `craft-app.json` is attached (and is valid, sections 1 and 2).
- `SHA256SUMS.txt` is attached.
- At least one package is attached.
- If the descriptor names an `icon`, that file is attached.

### Package names

Every package is named

```text
<id>-<version>-<os>-<arch>[-portable].<ext>
```

- `<id>` is the descriptor's `id` or one of its `former_ids`.
- `<version>` is a [SemVer](https://semver.org) version that starts with a digit (`1.2.0`,
  `1.2.0-beta.1`). It must equal the release tag without its leading `v` (tag `v1.2.0` or `1.2.0`
  both mean `1.2.0`). All packages share it.
- `<os>-<arch>` is one of:

  | OS | Tokens |
  |---|---|
  | Windows | `windows-x64`, `windows-x86`, `windows-arm64` |
  | macOS | `macos-universal`, `macos-x86_64`, `macos-aarch64`, `macos-arm64` |
  | Linux | `linux-x86_64`, `linux-aarch64`, `linux-riscv64` |
  | FreeBSD | `freebsd-x86_64`, `freebsd-aarch64` |

- The ending is one of: `-portable.zip`, `.tar.gz`, `.msi`, `.dmg`, `.AppImage`, `.deb`, `.rpm`,
  `.pkg.tar.zst`, `.flatpak` (case-sensitive). A Windows zip must be `-portable.zip`.

Examples: `examplecraft-1.2.0-windows-x64.msi`, `examplecraft-1.2.0-windows-x64-portable.zip`,
`examplecraft-1.2.0-macos-universal.dmg`, `examplecraft-1.2.0-linux-x86_64.AppImage`.

A file that starts with the id and ends with a package ending but doesn't follow the pattern
(`examplecraft-setup.msi`) is a problem. Anything else (release notes, `.AppImage.zsync`, a plain
`.zip`) isn't a package: it is ignored and named in a warning.

### `SHA256SUMS.txt`

- It is `sha256sum` output: one `<64 hex digits>  <file name>` line per file (`<hex> *<name>` is
  accepted too). File names are bare names (the same rule as `icon`). Malformed lines are skipped;
  the checker warns about them.
- It lists **every package, `craft-app.json`, and the icon** (if any). Listing other attached files
  is fine.
- It lists **nothing that isn't attached**.
- The checksums must be right. The launcher verifies each file it downloads against it;
  `craft-release-check dir` recomputes every listed checksum, and `craft-release-check github`
  verifies the descriptor and the icon.

### Platform rules

- **MSI publisher.** A release with an `.msi` package must name `msi_publisher` in the descriptor,
  and it must be the Publisher the MSI really registers: the launcher finds your installed copy by
  it.
- **Apple team and the DMG.** A release with a `.dmg` but no `apple_team` gets a warning: the
  launcher only installs a DMG whose `.app` is signed by the named Developer ID team, so without one
  it never installs the DMG, it only opens a copy that is already there.

### The icon

The `icon` asset is a PNG (checked by its signature and `IHDR` header only; it is never run) of at
most **256 KB** (262,144 bytes) and **1–512 px on each side**. Square works best.

## 4. The checker: `craft-release-check`

`crates/release-check` builds the `craft-release-check` binary.

```sh
cargo install --git https://github.com/YalKouassi/craft-unofficial-launcher craft-release-check --locked
```

| Command | What it does |
|---|---|
| `craft-release-check dir <dist> --repo <owner/name> --tag <vX.Y.Z> [--json]` | Checks a local folder holding exactly the files the release will attach, before upload: the descriptor (against `--repo`), every file name, the icon, and `SHA256SUMS.txt`, recomputing every checksum it lists. |
| `craft-release-check github --repo <owner/name> [--tag <vX.Y.Z>] [--json]` | Checks a published release (the latest without `--tag`) through the GitHub API: asset names, the descriptor and `SHA256SUMS.txt` (downloaded, size-capped, the descriptor's and icon's checksums verified), and the contract. `GITHUB_TOKEN`, when set, is sent to `api.github.com` only (it raises the hourly limit and reaches private repos). |
| `craft-release-check sums <dist>` | Writes `<dist>/SHA256SUMS.txt` for every file in the folder (sorted by name, excluding `SHA256SUMS.txt` itself and dot files). |
| `craft-release-check init --repo <owner/name> --id <id> --name <Name>` | Prints a starter `craft-app.json` that already passes the checks, with placeholder publisher, bundle id, MSI publisher and colour to edit. |

Exit codes: `0` the release keeps the contract (warnings may be printed), `1` it breaks it, `2`
the check couldn't run (bad arguments, an unreadable folder, a network error).

`--json` prints `{"ok": bool, "problems": [...], "warnings": [...], "packages": [{"name",
"version", "target", "suffix"}]}`. Inside GitHub Actions the text output also carries `::error::`
and `::warning::` annotations.

A typical local run before tagging:

```sh
craft-release-check sums dist/
craft-release-check dir dist/ --repo example-org/examplecraft --tag v1.2.0
```

### In CI

This repository ships a composite action at `.github/actions/craft-release-check`. It installs
Rust and `craft-release-check`, then runs `craft-release-check dir`. Run it after the packages,
the icon, `craft-app.json` and `SHA256SUMS.txt` are gathered in one folder, and before the upload
step:

```yaml
- uses: YalKouassi/craft-unofficial-launcher/.github/actions/craft-release-check@<commit-sha>
  with:
    dist: dist                     # required
    # repo: owner/name             # default: ${{ github.repository }}
    # tag: v1.2.0                  # default: ${{ github.ref_name }}
    # checker-ref: <commit-sha>    # default: main
```

Pin both the action and `checker-ref` to a commit. After publishing, `craft-release-check github
--repo <owner/name> --tag <tag>` confirms what users will actually see.

## 5. Two ways to integrate

1. **In the app's own repo.** Commit `craft-app.json` at the repo root (`craft-release-check
   init` gives a start), attach it unchanged to every release, list it in `SHA256SUMS.txt`, and run
   the action in the release workflow. The app can embed the same file
   (`include_str!("../craft-app.json")`) to drive its own updater, so the launcher and the app agree
   on its identity.
2. **Through a bridge repo.** When the upstream project doesn't want to adopt the format, an
   independent repo can track it (for example as a submodule), build packages that follow the
   contract, and publish them with `SHA256SUMS.txt` and a `craft-app.json` whose `repo` is the
   **bridge** repo (the releases come from there) and whose `upstream` names the original project.
   The bridge still may not use a reserved id, name, bundle id, publisher or team; `publisher`
   should say who maintains the bridge builds.
