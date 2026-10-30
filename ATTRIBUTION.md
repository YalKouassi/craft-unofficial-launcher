# Attribution

Every non-code asset in this repository, with its author, source and license. Craft Unofficial
Launcher's own code and original assets are MIT OR Apache-2.0 (see [`LICENSE-MIT`](LICENSE-MIT),
[`LICENSE-APACHE`](LICENSE-APACHE) and [`NOTICE`](NOTICE)). When you add an asset, add a row here
in the same change; third-party assets must be permissively licensed and keep their license file
next to them.

ArtCraft's name, logo and "A" mark are trademarks of their owners and are not used anywhere in this
project. The launcher bundles no icon or picture of any other app: an app's icon is fetched from
its own release when the user adds it, and shown under that app's own license.

## Bundled in the app

| Path | Title | Author | Source | License |
|---|---|---|---|---|
| `assets/app-icon/` (all files, including `tray/`) | Craft Unofficial Launcher app and tray icon: three tiles and an amber "launch" wedge on a dark square | Craft Unofficial Launcher contributors | Original work; `craft-unofficial-launcher.svg` and `tray/craft-unofficial-launcher-tray-black.svg` are the sources, every other file is rendered from them by `packaging/icons.sh` (see [`assets/app-icon/README.md`](assets/app-icon/README.md)) | MIT OR Apache-2.0, like the code |
| `packaging/macos/dmg/` (all files) | macOS DMG window background and layout | Craft Unofficial Launcher contributors | Original work, see [`packaging/macos/dmg/README.md`](packaging/macos/dmg/README.md); the launcher's own mark drawn inline in `background.svg`, rendered into `background.tiff` | MIT OR Apache-2.0, like the code. The TIFF carries LittleCMS's built-in sRGB profile ("No copyright, use freely") |
| `assets/social/github.svg`, `assets/social/globe.svg` | GitHub logo and a globe, both linking to this project's repository | [Simple Icons](https://simpleicons.org/) contributors; [Lucide](https://lucide.dev) contributors | Simple Icons `github.svg` and Lucide `globe.svg` (lucide-static 0.468.0), white for tinting; see [`assets/social/README.md`](assets/social/README.md) | CC0 1.0 (GitHub logo; a trademark of GitHub, Inc.); ISC (globe) |
| `assets/fonts/Inter-Variable.ttf` | Inter 4.001, variable (body text) | The Inter Project Authors (Rasmus Andersson) | Google Fonts, <https://github.com/google/fonts/tree/main/ofl/inter> (`Inter[opsz,wght].ttf`, renamed) | SIL OFL 1.1, [`assets/fonts/OFL-Inter.txt`](assets/fonts/OFL-Inter.txt) |
| `assets/fonts/Archivo-Variable.ttf` | Archivo 2.001, variable (headings and app names) | The Archivo Project Authors (Omnibus-Type) | Google Fonts, <https://github.com/google/fonts/tree/main/ofl/archivo> (`Archivo[wdth,wght].ttf`, renamed) | SIL OFL 1.1, [`assets/fonts/OFL-Archivo.txt`](assets/fonts/OFL-Archivo.txt) |
| `assets/fonts/GeistMono-Variable.ttf` | Geist Mono 1.701, variable (labels and buttons) | The Geist Project Authors (Vercel) | Google Fonts, <https://github.com/google/fonts/tree/main/ofl/geistmono> (`GeistMono[wght].ttf`, renamed) | SIL OFL 1.1, [`assets/fonts/OFL-GeistMono.txt`](assets/fonts/OFL-GeistMono.txt) |
| (dependency) `epaint_default_fonts/fonts/NotoEmoji-Regular.ttf` | Noto Emoji Regular, monochrome emoji fallback for arbitrary release-note, feed and alert text | The Noto Project Authors (Google) | Bundled transitively by `egui 0.37`'s `monochrome_emoji_fonts` feature from `epaint_default_fonts`; <https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts> | SIL OFL 1.1, `epaint_default_fonts/fonts/OFL.txt`; see also the dependency `README` and egui `README.md` |
| (dependency) `epaint_default_fonts/fonts/emoji-icon-font.ttf` | emoji-icon-font, monochrome symbol/emoji fallback | John Slegers | Bundled transitively by `egui 0.37`'s `monochrome_emoji_fonts` feature from `epaint_default_fonts`; <https://github.com/jslegers/emoji-icon-font> | MIT, Copyright (c) 2014 John Slegers, `epaint_default_fonts/fonts/emoji-icon-font-mit-license.txt` |
| (dependency) `epaint_default_fonts/fonts/egui-icons.ttf` | egui-icons, five-glyph subset of emoji-icon-font | John Slegers | [egui 6d518fc7](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts); included by default_fonts independently of monochrome_emoji_fonts | MIT, Copyright (c) 2014 John Slegers; full upstream text in `NOTICE` |
| (dependency) `epaint_default_fonts/fonts/Ubuntu-Light.ttf` | Ubuntu Light, default font fallback | Canonical Ltd. | [Pinned egui source](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts) | Ubuntu Font Licence 1.0; full upstream text and copyright in `NOTICE` |
| (dependency) `epaint_default_fonts/fonts/Hack-Regular.ttf` | Hack Regular, default monospace fallback | Source Foundry Authors; Bitstream Inc.; DejaVu contributors | [Pinned egui source](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts) | MIT and Bitstream Vera License; DejaVu work public domain; full upstream notices in `NOTICE` |

## Documentation only

Not bundled in the app.

| Path | Title | Author | Source | License |
|---|---|---|---|---|
| `docs/screenshot-apps.png` | Screenshot of the launcher's app list, used in the README | Craft Unofficial Launcher contributors | Captured from the launcher | The launcher's interface: MIT OR Apache-2.0, like the code. The app names, icons and descriptions shown belong to each app's own authors, under that app's own license |
