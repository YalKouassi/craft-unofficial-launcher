# Craft Unofficial Launcher app icon

The launcher's own mark: on a dark rounded square (`#16181d`), three off-white square tiles
(`#f2efe8`) in an L, and an amber right-angled wedge (`#f5a524`) in the fourth cell, pointing up and
out, for "launch". It is original work by the Craft Unofficial Launcher contributors, licensed
MIT OR Apache-2.0 like the code (see `LICENSE.txt` here and [`ATTRIBUTION.md`](../../ATTRIBUTION.md)).
It deliberately resembles no ArtCraft mark.

## Files

- `craft-unofficial-launcher.svg`: the source; also the hicolor scalable icon.
- `craft-unofficial-launcher-1024.png`: 1024 px render, with extra transparent margin for Apple's
  icon grid.
- `craft-unofficial-launcher.icns`: macOS bundle icon (`CFBundleIconFile`).
- `craft-unofficial-launcher.ico`: Windows icon, 16–256 px, embedded in the `.exe` by
  `apps/artcraft-launcher/build.rs` and used by the MSI.
- `hicolor/<size>/apps/io.github.yalkouassi.craft-unofficial-launcher.png` (16–512) and
  `hicolor/scalable/apps/io.github.yalkouassi.craft-unofficial-launcher.svg`: Linux icon theme.
- `tray/craft-unofficial-launcher-tray-black.svg`: the tray template source (the tiles and wedge in
  black, no backdrop), rendered to `tray/craft-unofficial-launcher-tray-black-44.png` (macOS menu
  bar, 22 pt @2x; macOS tints it). `tray/craft-unofficial-launcher-tray-64.png` is the colour icon
  for the Windows notification area and Linux/FreeBSD status notifiers.

## Regenerate

Edit the SVG sources, then run `packaging/icons.sh` on any OS. It needs `resvg`
(`cargo install resvg`); the `.ico` and `.icns` are packed by `cargo xtask ico` / `cargo xtask icns`.
