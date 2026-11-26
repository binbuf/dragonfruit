#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate the QML brand-glyph registry from the brand SVGs.

The files under ``assets/icons/brand/`` are the single source of truth for the
Dragonfruit and Tux OS-menu marks. Qt 6.11's software-renderer-friendly tint
path is a ``ShapePath`` with a bound ``fillColor``; ``Image``/``MultiEffect``
colorization is GPU-only and silently no-ops on the headless software backend
the QML tests use. This script extracts each SVG's view box and path data once
and emits the generated singleton ``DragonfruitLogo``/``TuxLogo`` render.

Running with ``--check`` (``make check-brand``) fails if the committed
registry is stale, so the SVGs and the rendered marks cannot drift.

Usage:
    scripts/gen-brand-glyphs.py            # (re)write the registry
    scripts/gen-brand-glyphs.py --check    # verify it is current
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ASSET_DIR = REPO / "assets" / "icons" / "brand"
OUT = REPO / "design-system" / "BrandGlyphs.qml"

HEADER = ("GENERATED FILE — DO NOT EDIT. Edit assets/icons/brand/ and run "
          "scripts/gen-brand-glyphs.py.")

_SVG = re.compile(r"<svg\b([^>]*)>", re.IGNORECASE | re.DOTALL)
_PATH = re.compile(r"<path\b([^>]*?)/?>", re.IGNORECASE | re.DOTALL)
_ATTR = re.compile(r'([a-zA-Z-]+)\s*=\s*"([^"]*)"')


def _attrs(blob: str) -> dict[str, str]:
    return {m.group(1): html.unescape(m.group(2)) for m in _ATTR.finditer(blob)}


def _view_box(attrs: dict[str, str], source: Path) -> tuple[float, float]:
    raw = attrs.get("viewBox", "")
    values = [v for v in re.split(r"[\s,]+", raw.strip()) if v]
    if len(values) != 4:
        raise SystemExit(f"{source}: expected a four-number viewBox, got {raw!r}")
    return float(values[2]), float(values[3])


def _paths(text: str) -> list[str]:
    out: list[str] = []
    for match in _PATH.finditer(text):
        path = _attrs(match.group(1)).get("d", "")
        if path:
            out.append(path)
    if not out:
        raise SystemExit("brand SVG has no <path> elements")
    return out


def _load(name: str) -> tuple[float, float, list[str]]:
    source = ASSET_DIR / f"{name}.svg"
    if not source.is_file():
        raise SystemExit(f"missing brand SVG: {source}")
    text = source.read_text(encoding="utf-8")
    svg = _SVG.search(text)
    if svg is None:
        raise SystemExit(f"{source}: no <svg> root")
    width, height = _view_box(_attrs(svg.group(1)), source)
    return width, height, _paths(text)


def _qml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def render() -> str:
    dragonfruit_width, dragonfruit_height, dragonfruit_paths = _load("dragonfruit")
    tux_width, tux_height, tux_paths = _load("tux")
    tux_lines = ",\n        ".join(_qml_string(path) for path in tux_paths)
    return f"""// SPDX-License-Identifier: GPL-3.0-or-later
// {HEADER}
pragma Singleton
import QtQuick

// The brand and OS-menu mark geometry. The menu bar renders these as tintable
// `Shape` paths so either mark can take the wallpaper foreground, contrast
// shadow, and hover/open plate. Do not hand-edit: change the SVGs under
// assets/icons/brand/ and run scripts/gen-brand-glyphs.py.
QtObject {{
    readonly property real dragonfruitViewBoxWidth: {dragonfruit_width:g}
    readonly property real dragonfruitViewBoxHeight: {dragonfruit_height:g}
    readonly property string dragonfruitPath: {_qml_string(dragonfruit_paths[0])}

    readonly property real tuxViewBoxWidth: {tux_width:g}
    readonly property real tuxViewBoxHeight: {tux_height:g}
    readonly property var tuxPaths: [
        {tux_lines}
    ]
}}
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="fail if the committed registry is stale")
    args = parser.parse_args()

    expected = render()
    if args.check:
        actual = OUT.read_text(encoding="utf-8") if OUT.is_file() else ""
        if actual != expected:
            print(f"{OUT} is stale; run scripts/gen-brand-glyphs.py", file=sys.stderr)
            return 1
        print(f"{OUT} is current")
        return 0

    OUT.write_text(expected, encoding="utf-8")
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
