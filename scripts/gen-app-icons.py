#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate the first-party app icon SVGs from vendored Phosphor glyphs (T-19.1d).

The two first-party applications carry original artwork built from the pinned
Phosphor set (MIT, T-19.1a):

  * ``org.dragonfruit.Files`` — Phosphor ``folders`` on a plain rounded-square
    app tile (no container treatment, matching every other Dock app tile).
  * ``org.dragonfruit.Settings`` — the System Settings gradient container
    (the ``SettingsCategoryIcon`` treatment: rounded square, vertical gradient,
    top inner highlight) with a Phosphor ``gear``.

The SVGs are the *single* artwork source: they are exposed to QML as bundled
resources (``qrc:/icons/apps/<name>.svg``, T-19.1d) for the Dock before an
install, and installed into the shipped icon theme so launchers, menus, and
the portal file chooser resolve the same file. Only the geometry and colour
constants below are decided here; the glyph paths come from the vendored SVGs,
so a Phosphor update is one file swap plus this generator.

Running with ``--check`` (``make check-phosphor``) fails if a committed icon is
stale, so the assets and the generator cannot drift.

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
PHOSPHOR_DIR = REPO / "assets" / "icons" / "phosphor"
OUT_DIR = REPO / "assets" / "icons" / "apps"

VIEW = 256.0
# Corner radius matches `component.dock.icon.radiusRatio` /
# `component.settingsCategory.radiusRatio` (0.24) at the 256 viewBox.
RADIUS = round(VIEW * 0.24, 2)

# Files: a plain app tile (no gradient container), the deterministic blue of
# our file manager. White fill glyph.
FILES_TILE = "#3b82f6"
FILES_GLYPH = "folders"
FILES_GLYPH_RATIO = 0.6

# Settings: the one app icon that carries the gradient container, matching
# SettingsPanes' "general" category hue (`#8e8e93` -> `#58585c`).
SETTINGS_GLYPH = "gear"
SETTINGS_GRADIENT_START = "#8e8e93"
SETTINGS_GRADIENT_END = "#58585c"
SETTINGS_GLYPH_RATIO = 0.55
SETTINGS_HIGHLIGHT_RATIO = 0.45
SETTINGS_HIGHLIGHT_OPACITY = 0.28

WHITE = "#ffffff"

_D = re.compile(r'<path\b[^>]*\bd="([^"]+)"', re.IGNORECASE | re.DOTALL)


def glyph_path(name: str) -> str:
    """The first path `d` of a vendored Phosphor glyph."""
    path = PHOSPHOR_DIR / f"{name}-fill.svg"
    text = path.read_text()
    match = _D.search(text)
    if match is None:
        raise SystemExit(f"no path in {path.relative_to(REPO)}")
    return match.group(1).strip()


def _centered_glyph(name: str, ratio: float) -> str:
    edge = VIEW * ratio
    off = (VIEW - edge) / 2
    return (f'  <path transform="translate({off:g},{off:g}) scale({ratio:g})" '
            f'fill="{WHITE}" d="{glyph_path(name)}"/>\n')


def files_svg() -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">\n'
        f'  <rect x="0" y="0" width="256" height="256" rx="{RADIUS:g}" '
        f'fill="{FILES_TILE}"/>\n'
        + _centered_glyph(FILES_GLYPH, FILES_GLYPH_RATIO)
        + "</svg>\n"
    )


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
    "org.dragonfruit.Files.svg": files_svg,
    "org.dragonfruit.Settings.svg": settings_svg,
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

    if args.check:
        if stale:
            for path in stale:
                print(f"stale generated app icon: {path}", file=sys.stderr)
            print("run scripts/gen-app-icons.py and commit the result",
                  file=sys.stderr)
            return 1
        print(f"app icon assets are current ({len(ICONS)} icons)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())