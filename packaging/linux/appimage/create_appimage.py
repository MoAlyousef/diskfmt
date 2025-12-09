#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "plumbum",
#   "toml",
# ]
# ///
from __future__ import annotations

import os
import stat
import sys
import urllib.request
from pathlib import Path

import toml
from plumbum import CommandNotFound, local, ProcessExecutionError
from plumbum.cmd import convert as im_convert
from plumbum.cmd import cargo


SIZES = [16, 32, 48, 64, 128, 256]


def die(msg: str, code: int = 1) -> None:
    print(msg, file=sys.stderr)
    raise SystemExit(code)


def read_version_from_cargo(cargo_toml: Path) -> str | None:
    """Parse version from Cargo.toml."""
    if not cargo_toml.is_file():
        return None
    data = toml.loads(cargo_toml.read_text(encoding="utf-8"))
    return data.get("package", {}).get("version")


def ensure_executable(path: Path) -> None:
    st = path.stat()
    path.chmod(st.st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def download_tool(url: str, target: Path) -> None:
    """
    Replacement for:
        curl -L --fail --retry 3 --retry-delay 2 -o target url
    Using urllib.request with basic retry logic.
    """
    if target.is_file():
        return

    print(f"Downloading {target.name} from {url}")

    attempts = 3
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(url) as resp, target.open("wb") as out:
                out.write(resp.read())
            break
        except Exception as e:
            if attempt == attempts:
                die(f"Failed to download {url}: {e}")
            else:
                print(f"Download failed (attempt {attempt}/{attempts}), retrying…")
                import time

                time.sleep(2)

    ensure_executable(target)


def ensure_newline_at_end(path: Path) -> None:
    if not path.is_file():
        return
    data = path.read_bytes()
    if data and not data.endswith(b"\n"):
        path.write_bytes(data + b"\n")


def main() -> None:
    script_path = Path(__file__).resolve()
    script_dir = script_path.parent
    root_dir = (script_dir / ".." / ".." / "..").resolve()

    build_dir = root_dir / "target"
    appimage_dir = build_dir / "appimage"
    appdir = appimage_dir / "AppDir"

    bin_path = build_dir / "release" / "diskfmt"
    desktop_file = script_dir / "diskfmt.desktop"
    metadata_file = root_dir / "packaging" / "linux" / "diskfmt.metainfo.xml"

    icon_src = Path(os.environ.get("ICON_SRC", root_dir / "assets" / "icon.png"))
    arch = os.environ.get("ARCH", os.uname().machine)

    version = os.environ.get("VERSION")
    if not version:
        version = read_version_from_cargo(root_dir / "Cargo.toml")

    if not version:
        die("Failed to read version from Cargo.toml")

    linuxdeploy = Path(
        os.environ.get("LINUXDEPLOY", appimage_dir / f"linuxdeploy-{arch}.AppImage")
    )
    appimagetool = Path(
        os.environ.get("APPIMAGETOOL", appimage_dir / f"appimagetool-{arch}.AppImage")
    )

    linuxdeploy_url = os.environ.get(
        "LINUXDEPLOY_URL",
        f"https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-{arch}.AppImage",
    )
    appimagetool_url = os.environ.get(
        "APPIMAGETOOL_URL",
        f"https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-{arch}.AppImage",
    )

    os.environ["ARCH"] = arch
    os.environ.setdefault("APPIMAGE_EXTRACT_AND_RUN", "1")

    # Tool checks
    try:
        im_convert = local["convert"]
    except CommandNotFound:
        die("ImageMagick 'convert' is required to generate icons")

    if not desktop_file.is_file():
        die(f"Desktop file not found: {desktop_file}")
    if not icon_src.is_file():
        die(f"Icon not found: {icon_src}")
    if not metadata_file.is_file():
        die(f"Metadata not found: {metadata_file}")

    appimage_dir.mkdir(parents=True, exist_ok=True)

    # Download dependencies
    download_tool(linuxdeploy_url, linuxdeploy)
    download_tool(appimagetool_url, appimagetool)

    # Build if binary missing
    if not (bin_path.is_file() and os.access(bin_path, os.X_OK)):
        print(f"Building release binary (missing at {bin_path})")
        with local.cwd(root_dir):
            cargo["build", "--release"]()

    # Clean AppDir
    import shutil

    shutil.rmtree(appdir, ignore_errors=True)
    (appdir / "usr/share/applications").mkdir(parents=True, exist_ok=True)

    # Desktop file
    desktop_target = appdir / "usr/share/applications/diskfmt.desktop"
    shutil.copy2(desktop_file, desktop_target)

    if "X-AppImage-Version=" not in desktop_target.read_text():
        ensure_newline_at_end(desktop_target)
        desktop_target.write_text(
            desktop_target.read_text() + f"X-AppImage-Version={version}\n"
        )

    # Symlink
    symlink = appdir / "diskfmt.desktop"
    if symlink.exists():
        symlink.unlink()
    symlink.symlink_to("usr/share/applications/diskfmt.desktop")

    # Metadata
    metainfo_dir = appdir / "usr/share/metainfo"
    metainfo_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(metadata_file, metainfo_dir / "diskfmt.metainfo.xml")

    # Icons
    for size in SIZES:
        icon_dir = appdir / f"usr/share/icons/hicolor/{size}x{size}/apps"
        icon_dir.mkdir(parents=True, exist_ok=True)
        im_convert[str(icon_src), "-resize", f"{size}x{size}", str(icon_dir / "diskfmt.png")]()

    shutil.copy2(
        appdir / "usr/share/icons/hicolor/256x256/apps/diskfmt.png",
        appdir / "diskfmt.png",
    )

    # linuxdeploy
    local[linuxdeploy](
        "--appdir", str(appdir),
        "--executable", str(bin_path),
        "--desktop-file", str(desktop_target),
        "--icon-file", str(appdir / "diskfmt.png"),
    )

    output = Path(
        os.environ.get(
            "OUTPUT", appimage_dir / f"diskfmt-{version}-{arch}.AppImage"
        )
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        output.unlink()

    # appimagetool
    local[appimagetool](str(appdir), str(output))

    print(f"Created {output}")


if __name__ == "__main__":
    try:
        main()
    except ProcessExecutionError as e:
        die(f"Command failed ({e.retcode}): {e}")
