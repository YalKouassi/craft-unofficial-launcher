#!/usr/bin/env bash
# Regenerate every app and tray icon from the launcher's own mark,
# assets/app-icon/craft-unofficial-launcher.svg (original work, MIT OR Apache-2.0; see ATTRIBUTION.md).
#
# - App icon (Dock, taskbar, Start menu, Linux menus, installers): the full-colour mark.
# - Tray: the black template (assets/app-icon/tray/craft-unofficial-launcher-tray-black.svg) for the
#   macOS menu bar; the full-colour mark on Windows, Linux and BSD.
#
# Needs: resvg (brew install resvg / cargo install resvg). The .ico and .icns are packed by
# `cargo xtask ico` / `cargo xtask icns`, so this runs on any OS. The outputs are committed, so
# packaging (and the Rust build, which include_bytes!s some of them) never needs these tools.
#
#   packaging/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIR="$ROOT/assets/app-icon"
NAME="craft-unofficial-launcher"
SVG="$DIR/$NAME.svg"
ID="io.github.yalkouassi.craft-unofficial-launcher"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# macOS icons get a transparent margin (Apple's 824/1024 icon grid) by widening the view box.
BOX='viewBox="0 0 128 128"'
grep -q "$BOX" "$SVG" || { echo "error: expected $BOX in $SVG" >&2; exit 1; }
MAC="$TMP/mac.svg"
sed "s/$BOX/viewBox=\"-12.66 -12.66 153.32 153.32\"/" "$SVG" >"$MAC"
FLAT="$SVG"

render() {
  local src="$1" size="$2" out="$3"
  resvg -w "$size" -h "$size" "$src" "$out" </dev/null
}

render "$MAC" 1024 "$DIR/$NAME-1024.png"

# Linux hicolor theme.
for s in 16 24 32 48 64 128 256 512; do
  mkdir -p "$DIR/hicolor/${s}x${s}/apps"
  render "$FLAT" "$s" "$DIR/hicolor/${s}x${s}/apps/$ID.png"
done
mkdir -p "$DIR/hicolor/scalable/apps"
cp "$SVG" "$DIR/hicolor/scalable/apps/$ID.svg"

# Windows .ico.
ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  render "$FLAT" "$s" "$TMP/ico-$s.png"
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/$NAME.ico" "${ICO_PNGS[@]}")

# macOS .icns.
ICNS_PNGS=()
for s in 16 32 64 128 256 512 1024; do
  render "$MAC" "$s" "$TMP/icns-$s.png"
  ICNS_PNGS+=("$TMP/icns-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- icns "$DIR/$NAME.icns" "${ICNS_PNGS[@]}")

# Tray icons: the black template for the macOS menu bar (44 px = 22 pt @2x; macOS tints it); the
# colour mark for the Windows notification area and Linux/BSD status notifiers.
mkdir -p "$DIR/tray"
render "$DIR/tray/$NAME-tray-black.svg" 44 "$DIR/tray/$NAME-tray-black-44.png"
render "$SVG" 64 "$DIR/tray/$NAME-tray-64.png"
echo "icons written to $DIR"
