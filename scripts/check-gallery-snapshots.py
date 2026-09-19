#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Headless visual regression for the component gallery (T-08, FR-6).

Runs the gallery app in snapshot mode on the offscreen platform + software
scene graph, then checks the rendered pages. The gate is deterministic and
font-independent: it samples exact token colors at known component positions
rather than comparing whole images byte-for-byte. Committed goldens under
``design-system/gallery/snapshots`` are the art-direction reference; pass
``--strict`` to also diff against them, or ``--update`` to regenerate them.

Usage:
    scripts/check-gallery-snapshots.py [--build-dir build] [--strict] [--update]
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GOLDEN_DIR = REPO / "design-system" / "gallery" / "snapshots"
PAGES = [
    "tokens", "window", "titlebar", "trafficlights", "toggle", "popup", "menu", "ssd",
    "buttons", "sidebar", "toolbar", "splitview", "settings", "segmented", "contextmenu",
    "searchfield", "sourcelist", "dialog", "sheet", "popover", "scrollview", "icons",
]
SCHEMES = [("light", False), ("dark", False), ("dark", True)]


def load_tokens() -> dict:
    spec = importlib.util.spec_from_file_location("gen_tokens", REPO / "scripts" / "gen-tokens.py")
    gen = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(gen)
    data = gen.load()
    return data, gen.resolve


def hex_rgb(value: str) -> tuple[int, int, int]:
    value = value.lstrip("#")
    if len(value) == 8:
        value = value[2:]  # drop alpha
    return tuple(int(value[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def read_png(path: Path) -> tuple[int, int, bytes]:
    """Minimal 8-bit RGBA/RGB PNG reader (no Pillow dependency)."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", f"{path}: not a PNG"
    pos, width, height, color_type, idat = 8, 0, 0, 0, b""
    while pos < len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        chunk = data[pos + 4 : pos + 8]
        payload = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if chunk == b"IHDR":
            width, height, _depth, color_type = struct.unpack(">IIBB", payload[:10])
        elif chunk == b"IDAT":
            idat += payload
        elif chunk == b"IEND":
            break
    channels = {0: 1, 2: 3, 4: 2, 6: 4}[color_type]
    raw = zlib.decompress(idat)
    stride = width * channels
    pixels = bytearray()
    previous = bytearray(stride)
    offset = 0
    for _ in range(height):
        filter_type = raw[offset]
        offset += 1
        line = bytearray(raw[offset : offset + stride])
        offset += stride
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = previous[x]
            c = previous[x - channels] if x >= channels else 0
            if filter_type == 1:
                line[x] = (line[x] + a) & 0xFF
            elif filter_type == 2:
                line[x] = (line[x] + b) & 0xFF
            elif filter_type == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 0xFF
            elif filter_type == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 0xFF
        pixels += line
        previous = line
    return width, height, bytes(pixels)


def contains_color(path: Path, rgb: tuple[int, int, int], tolerance: int = 0) -> bool:
    width, height, pixels = read_png(path)
    channels = len(pixels) // (width * height)
    target = bytes(rgb)
    for i in range(0, len(pixels), channels):
        pixel = pixels[i : i + 3]
        if tolerance == 0:
            if pixel == target:
                return True
        elif all(abs(pixel[j] - target[j]) <= tolerance for j in range(3)):
            return True
    return False


def run_gallery(binary: Path, output: Path) -> None:
    env = dict(os.environ)
    env.update(
        QT_QPA_PLATFORM="offscreen",
        QT_QUICK_BACKEND="software",
        DRAGONFRUIT_GALLERY_SNAPSHOT=str(output),
    )
    result = subprocess.run([str(binary)], env=env, capture_output=True, text=True)
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(f"gallery snapshot run failed (exit {result.returncode})")


def invariant_checks(output: Path, tokens: dict, resolve) -> list[str]:
    light = tokens["semantic"]["light"]["color"]
    dark = tokens["semantic"]["dark"]["color"]
    failures: list[str] = []

    def check(condition: bool, message: str) -> None:
        if not condition:
            failures.append(message)

    for scheme, reduced in SCHEMES:
        suffix = f"{scheme}{'_reduced' if reduced else ''}"
        for page in PAGES:
            path = output / f"{page}_{suffix}.png"
            check(path.exists(), f"missing snapshot {path.name}")

    # Semantic token colors must survive into the rendered pixels.
    check(contains_color(output / "trafficlights_light.png", hex_rgb(resolve(light["close"], tokens))),
          "light traffic lights: close color not found")
    check(contains_color(output / "trafficlights_light.png", hex_rgb(resolve(light["zoom"], tokens))),
          "light traffic lights: zoom color not found")
    check(contains_color(output / "trafficlights_dark.png", hex_rgb(resolve(dark["close"], tokens))),
          "dark traffic lights: close color not found")
    check(contains_color(output / "toggle_dark.png", hex_rgb(resolve(dark["accent"], tokens))),
          "dark toggle: accent color not found")
    check(contains_color(output / "window_light.png", hex_rgb(resolve(light["surface"], tokens))),
          "light window: surface color not found")
    check(contains_color(output / "window_dark.png", hex_rgb(resolve(dark["surface"], tokens))),
          "dark window: surface color not found")
    check(contains_color(output / "buttons_light.png", hex_rgb(resolve(light["accent"], tokens))),
          "light buttons: accent color not found")
    check(contains_color(output / "sidebar_dark.png", hex_rgb(resolve(dark["accent"], tokens))),
          "dark sidebar: selected accent color not found")
    check(contains_color(output / "dialog_dark.png", hex_rgb(resolve(dark["surfaceElevated"], tokens))),
          "dark dialog: elevated surface color not found")
    check(contains_color(output / "settings_light.png", hex_rgb(resolve(light["surfaceElevated"], tokens))),
          "light settings: elevated surface color not found")
    check(contains_color(output / "searchfield_light.png", hex_rgb(resolve(light["controlFill"], tokens))),
          "light search field: control fill color not found")

    # Light and dark renders must actually differ.
    light_bytes = (output / "window_light.png").read_bytes()
    dark_bytes = (output / "window_dark.png").read_bytes()
    check(light_bytes != dark_bytes, "light and dark window renders are identical")

    return failures


def strict_compare(output: Path, tolerance: int) -> list[str]:
    failures: list[str] = []
    for golden in sorted(GOLDEN_DIR.glob("*.png")):
        candidate = output / golden.name
        if not candidate.exists():
            failures.append(f"missing generated snapshot {golden.name}")
            continue
        gw, gh, gp = read_png(golden)
        cw, ch, cp = read_png(candidate)
        if (gw, gh) != (cw, ch):
            failures.append(f"{golden.name}: size {cw}x{ch} != golden {gw}x{gh}")
            continue
        differing = 0
        for i in range(0, min(len(gp), len(cp)), 4):
            if any(abs(gp[i + j] - cp[i + j]) > tolerance for j in range(3)):
                differing += 1
        ratio = differing / (gw * gh)
        if ratio > 0.01:
            failures.append(f"{golden.name}: {ratio:.2%} pixels differ from golden")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", default=str(REPO / "build"))
    parser.add_argument("--binary", default="")
    parser.add_argument("--strict", action="store_true", help="also diff against committed goldens")
    parser.add_argument("--update", action="store_true", help="write goldens instead of checking")
    parser.add_argument("--tolerance", type=int, default=2, help="per-channel tolerance for --strict")
    args = parser.parse_args()

    binary = Path(args.binary) if args.binary else Path(args.build_dir) / "design-system" / "gallery" / "dragonfruit-gallery-app"
    if not binary.exists():
        raise SystemExit(f"gallery binary not found: {binary} (run `make build`)")

    tokens, resolve = load_tokens()

    with tempfile.TemporaryDirectory() as tmp:
        output = Path(tmp)
        run_gallery(binary, output)

        if args.update:
            GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
            for snapshot in output.glob("*.png"):
                shutil.copy(snapshot, GOLDEN_DIR / snapshot.name)
            print(f"updated {len(list(GOLDEN_DIR.glob('*.png')))} gallery goldens")
            return 0

        failures = invariant_checks(output, tokens, resolve)
        if args.strict:
            failures += strict_compare(output, args.tolerance)

        if failures:
            for failure in failures:
                print(f"FAIL: {failure}", file=sys.stderr)
            return 1

        print(f"gallery visual regression passed ({len(PAGES) * len(SCHEMES)} snapshots)")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
