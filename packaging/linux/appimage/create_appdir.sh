#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(realpath "$SCRIPT_DIR/../../..")"
BUILD_DIR="$ROOT_DIR/target"
APPIMAGE_DIR="$BUILD_DIR/appimage"
APPDIR="$APPIMAGE_DIR/AppDir"
BIN="$BUILD_DIR/release/diskfmt"
DESKTOP_FILE="$SCRIPT_DIR/diskfmt.desktop"
ICON_SRC="${ICON_SRC:-$ROOT_DIR/assets/icon.png}"
ARCH="${ARCH:-$(uname -m)}"
VERSION="${VERSION:-$(awk -F '\"' '/^version[[:space:]]*=/{print $2; exit}' "$ROOT_DIR/Cargo.toml")}"
METADATA_FILE="$ROOT_DIR/packaging/linux/diskfmt.metainfo.xml"
LINUXDEPLOY="${LINUXDEPLOY:-$APPIMAGE_DIR/linuxdeploy-${ARCH}.AppImage}"
APPIMAGETOOL="${APPIMAGETOOL:-$APPIMAGE_DIR/appimagetool-${ARCH}.AppImage}"
LINUXDEPLOY_URL="${LINUXDEPLOY_URL:-https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$ARCH.AppImage}"
APPIMAGETOOL_URL="${APPIMAGETOOL_URL:-https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$ARCH.AppImage}"
SIZES=(16 32 48 64 128 256)

export ARCH
export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"

if ! command -v convert >/dev/null 2>&1; then
  echo "ImageMagick's convert is required to generate icons"
  exit 1
fi

if [ -z "$VERSION" ]; then
  echo "Failed to read version from Cargo.toml"
  exit 1
fi

if [ ! -f "$DESKTOP_FILE" ]; then
  echo "Desktop file not found at $DESKTOP_FILE"
  exit 1
fi

if [ ! -f "$ICON_SRC" ]; then
  echo "Icon source not found at $ICON_SRC"
  exit 1
fi

if [ ! -f "$METADATA_FILE" ]; then
  echo "AppStream metadata not found at $METADATA_FILE"
  exit 1
fi

mkdir -p "$APPIMAGE_DIR"

download_tool() {
  local url="$1"
  local target="$2"

  if [ -f "$target" ]; then
    return
  fi

  echo "Downloading $(basename "$target") from $url"
  curl -L --fail --retry 3 --retry-delay 2 -o "$target" "$url"
  chmod +x "$target"
}

download_tool "$LINUXDEPLOY_URL" "$LINUXDEPLOY"
download_tool "$APPIMAGETOOL_URL" "$APPIMAGETOOL"

if [ ! -x "$BIN" ]; then
  echo "Building release binary (missing at $BIN)"
  (cd "$ROOT_DIR" && cargo build --release)
fi

rm -rf "$APPDIR"

mkdir -p "$APPDIR/usr/share/applications"

install -Dm644 "$DESKTOP_FILE" "$APPDIR/usr/share/applications/diskfmt.desktop"

DESKTOP_TARGET="$APPDIR/usr/share/applications/diskfmt.desktop"
if ! grep -q "^X-AppImage-Version=" "$DESKTOP_TARGET"; then
  if [ -s "$DESKTOP_TARGET" ] && [ "$(tail -c1 "$DESKTOP_TARGET" | wc -l)" -eq 0 ]; then
    printf '\n' >> "$DESKTOP_TARGET"
  fi
  printf 'X-AppImage-Version=%s\n' "$VERSION" >> "$DESKTOP_TARGET"
fi
ln -sf usr/share/applications/diskfmt.desktop "$APPDIR/diskfmt.desktop"

install -Dm644 "$METADATA_FILE" "$APPDIR/usr/share/metainfo/diskfmt.metainfo.xml"

for size in "${SIZES[@]}"; do
  icon_dir="$APPDIR/usr/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$icon_dir"
  convert "$ICON_SRC" -resize "${size}x${size}" "$icon_dir/diskfmt.png"
done

cp "$APPDIR/usr/share/icons/hicolor/256x256/apps/diskfmt.png" "$APPDIR/diskfmt.png"

"$LINUXDEPLOY" \
  --appdir "$APPDIR" \
  --executable "$BIN" \
  --desktop-file "$DESKTOP_TARGET" \
  --icon-file "$APPDIR/usr/share/icons/hicolor/256x256/apps/diskfmt.png"

OUTPUT="${OUTPUT:-$APPIMAGE_DIR/diskfmt-${VERSION}-${ARCH}.AppImage}"
mkdir -p "$(dirname "$OUTPUT")"
rm -f "$OUTPUT"

"$APPIMAGETOOL" "$APPDIR" "$OUTPUT"

echo "Created $OUTPUT"
