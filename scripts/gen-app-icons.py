#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Assemble the first-party app icon SVGs from pinned artwork.

The two first-party applications use two different, deliberately recorded
sources:

  * ``org.dragonfruit.Files`` — the vendored WhiteSur ``file-manager.svg``,
    copied byte-for-byte from ``assets/icons/whitesur/`` (GPL-3.0-only; see
    that directory's README). The file is still the *single* artwork source,
    so a future theme bump is one upstream swap plus this copy step.
  * ``org.dragonfruit.Settings`` — an original gradient container (the
    ``SettingsCategoryIcon`` treatment: rounded square, vertical gradient, top
    inner highlight) with a Tabler ``settings`` glyph (MIT, ADR 0185).

The SVGs are exposed to QML as bundled resources (``qrc:/icons/apps/<name>.svg``,
T-19.1d) for the Dock before an install, and installed into the shipped icon
theme so launchers, menus, and the portal file chooser resolve the same file.
Running with ``--check`` (``make check-tabler``) fails if a committed icon is
stale, so the assets and their pinned sources cannot drift.

Usage:
    scripts/gen-app-icons.py            # (re)write the two icons
    scripts/gen-app-icons.py --check    # verify they are current
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TABLER_DIR = REPO / "assets" / "icons" / "tabler"
WHITESUR_DIR = REPO / "assets" / "icons" / "whitesur"
OUT_DIR = REPO / "assets" / "icons" / "apps"

VIEW = 256.0
TABLER_VIEW = 24.0
# Corner radius matches `component.dock.icon.radiusRatio` /
# `component.settingsCategory.radiusRatio` (0.24) at the 256 viewBox.
RADIUS = round(VIEW * 0.24, 2)

# Settings: the one app icon that carries the gradient container, matching
# SettingsPanes' "general" category hue (`#8e8e93` -> `#58585c`).
SETTINGS_GLYPH = "settings"
SETTINGS_GRADIENT_START = "#8e8e93"
SETTINGS_GRADIENT_END = "#58585c"
SETTINGS_GLYPH_RATIO = 0.55
SETTINGS_HIGHLIGHT_RATIO = 0.45
SETTINGS_HIGHLIGHT_OPACITY = 0.28

WHITE = "#ffffff"

_TAG = re.compile(r"<path\b([^>]*?)/?>", re.IGNORECASE | re.DOTALL)
_ATTR = re.compile(r'([a-zA-Z-]+)\s*=\s*"([^"]*)"')


def glyph_path(name: str) -> str:
    """The visible path `d` of a vendored Tabler filled glyph."""
    path = TABLER_DIR / f"{name}-filled.svg"
    text = path.read_text()
    for match in _TAG.finditer(text):
        attrs = dict(_ATTR.findall(match.group(1)))
        d = attrs.get("d", "").strip()
        if d and attrs.get("fill") not in ("none",) and attrs.get("stroke") not in ("none",):
            return d
    raise SystemExit(f"no visible path in {path.relative_to(REPO)}")


def _centered_glyph(name: str, ratio: float) -> str:
    edge = VIEW * ratio
    scale = edge / TABLER_VIEW
    off = (VIEW - edge) / 2
    return (f'  <path transform="translate({off:g},{off:g}) scale({scale:g})" '
            f'fill="{WHITE}" d="{glyph_path(name)}"/>\n')


def settings_svg() -> str:
    highlight = VIEW * SETTINGS_HIGHLIGHT_RATIO
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">\n'
        "  <defs>\n"
        '    <linearGradient id="tile" x1="0" y1="0" x2="0" y2="1">\n'
        f'      <stop offset="0" stop-color="{SETTINGS_GRADIENT_START}"/>\n'
        f'      <stop offset="1" stop-color="{SETTINGS_GRADIENT_END}"/>\n'
        "    </linearGradient>\n"
        '    <linearGradient id="highlight" x1="0" y1="0" x2="0" y2="1">\n'
        f'      <stop offset="0" stop-color="{WHITE}" '
        f'stop-opacity="{SETTINGS_HIGHLIGHT_OPACITY:g}"/>\n'
        f'      <stop offset="1" stop-color="{WHITE}" stop-opacity="0"/>\n'
        "    </linearGradient>\n"
        "  </defs>\n"
        f'  <rect x="0" y="0" width="256" height="256" rx="{RADIUS:g}" '
        'fill="url(#tile)"/>\n'
        f'  <rect x="0" y="0" width="256" height="{highlight:g}" '
        f'rx="{RADIUS:g}" fill="url(#highlight)"/>\n'
        + _centered_glyph(SETTINGS_GLYPH, SETTINGS_GLYPH_RATIO)
        + "</svg>\n"
    )


ICONS = {
    "org.dragonfruit.Settings.svg": settings_svg,
}

# The Files tile is upstream artwork, not generated geometry. Copy it
# byte-for-byte so the installed/qrc asset stays one reviewable file swap.
COPIES = {
    "org.dragonfruit.Files.svg": WHITESUR_DIR / "file-manager.svg",
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="verify the committed icons are current")
    args = parser.parse_args()

    stale: list[str] = []
    for name, build in ICONS.items():
        text = build()
        dest = OUT_DIR / name
        if args.check:
            if not dest.exists() or dest.read_text() != text:
                stale.append(str(dest.relative_to(REPO)))
            continue
        OUT_DIR.mkdir(parents=True, exist_ok=True)
        dest.write_text(text)
        print(f"wrote {dest.relative_to(REPO)}")

    for name, source in COPIES.items():
        data = source.read_bytes()
        dest = OUT_DIR / name
        if args.check:
            if not dest.exists() or dest.read_bytes() != data:
                stale.append(str(dest.relative_to(REPO)))
            continue
        OUT_DIR.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        print(f"copied {source.relative_to(REPO)} -> {dest.relative_to(REPO)}")

    if args.check:
        if stale:
            for path in stale:
                print(f"stale generated app icon: {path}", file=sys.stderr)
            print("run scripts/gen-app-icons.py and commit the result",
                  file=sys.stderr)
            return 1
        print(f"app icon assets are current ({len(ICONS) + len(COPIES)} icons)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())