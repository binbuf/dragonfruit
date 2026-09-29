#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate the QML Phosphor glyph registry from the vendored SVGs (T-19.1a).

The vendored Phosphor files under ``assets/icons/phosphor/`` are the single
source of truth. Qt 6.11's only software-renderer-friendly tint path is a
``ShapePath`` with a bound ``fillColor``: the built-in ``MultiEffect``
colorization is GPU-only and silently no-ops on the headless software backend
the gallery and QML tests use (the same limit ``Shadow.qml`` documents). So
this script extracts each glyph's path data once, at build time, and emits one
generated QML singleton that ``PhosphorIcon`` renders and tints.

Running with ``--check`` (``make check-phosphor``) fails if the committed
registry is stale, so the SVGs and the registry cannot drift.

Usage:
    scripts/gen-phosphor-glyphs.py            # (re)write the registry
    scripts/gen-phosphor-glyphs.py --check    # verify it is current
"""

from __future__ import annotations

import argparse
import html
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ASSET_DIR = REPO / "assets" / "icons" / "phosphor"
OUT = REPO / "design-system" / "PhosphorGlyphs.qml"

HEADER = ("GENERATED FILE — DO NOT EDIT. Edit assets/icons/phosphor/ and run "
          "scripts/gen-phosphor-glyphs.py.")

# The two vendored weights. The default is `regular`; a file name ends in
# `-<weight>.svg` for every other weight (Phosphor's upstream convention).
DEFAULT_WEIGHT = "regular"
WEIGHTS = ("regular", "fill")
VIEW_BOX = 256.0

_TAG = re.compile(r"<(path|rect|circle|ellipse)\b([^>]*?)/?>", re.IGNORECASE | re.DOTALL)
_ATTR = re.compile(r'([a-zA-Z-]+)\s*=\s*"([^"]*)"')


def _attrs(blob: str) -> dict[str, str]:
    return {m.group(1): html.unescape(m.group(2)) for m in _ATTR.finditer(blob)}


def _f(value: str, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def _rect_path(a: dict[str, str]) -> str:
    x, y = _f(a.get("x")), _f(a.get("y"))
    w, h = _f(a.get("width")), _f(a.get("height"))
    rx = _f(a.get("rx") if "rx" in a else a.get("ry", "0"))
    ry = _f(a.get("ry") if "ry" in a else a.get("rx", "0"))
    rx, ry = min(rx, w / 2), min(ry, h / 2)
    if rx <= 0 or ry <= 0:
        return f"M {x} {y} H {x + w} V {y + h} H {x} Z"
    return (
        f"M {x + rx} {y} H {x + w - rx} "
        f"A {rx} {ry} 0 0 1 {x + w} {y + ry} V {y + h - ry} "
        f"A {rx} {ry} 0 0 1 {x + w - rx} {y + h} H {x + rx} "
        f"A {rx} {ry} 0 0 1 {x} {y + h - ry} V {y + ry} "
        f"A {rx} {ry} 0 0 1 {x + rx} {y} Z"
    )


def _ellipse_path(cx: float, cy: float, rx: float, ry: float) -> str:
    return (f"M {cx - rx} {cy} a {rx} {ry} 0 1 0 {2 * rx} 0 "
            f"a {rx} {ry} 0 1 0 {-2 * rx} 0 Z")


def _circle_path(a: dict[str, str]) -> str:
    r = _f(a.get("r"))
    return _ellipse_path(_f(a.get("cx")), _f(a.get("cy")), r, r)


def _ellipse_elem_path(a: dict[str, str]) -> str:
    return _ellipse_path(_f(a.get("cx")), _f(a.get("cy")),
                         _f(a.get("rx")), _f(a.get("ry")))


def extract_path(svg_text: str) -> str:
    """Return one PathSvg-usable path string for a Phosphor SVG."""
    parts: list[str] = []
    for match in _TAG.finditer(svg_text):
        tag, blob = match.group(1).lower(), match.group(2)
        attrs = _attrs(blob)
        if tag == "path":
            d = (attrs.get("d") or "").strip()
            if d:
                parts.append(d)
        elif tag == "rect":
            parts.append(_rect_path(attrs))
        elif tag == "circle":
            parts.append(_circle_path(attrs))
        elif tag == "ellipse":
            parts.append(_ellipse_elem_path(attrs))
    return " ".join(parts)


def load_glyphs() -> dict[str, dict[str, str]]:
    glyphs: dict[str, dict[str, str]] = {weight: {} for weight in WEIGHTS}
    for path in sorted(ASSET_DIR.glob("*.svg")):
        stem = path.stem
        if stem.endswith("-fill"):
            weight, name = "fill", stem[: -len("-fill")]
        else:
            weight, name = "regular", stem
        if weight not in WEIGHTS:
            continue
        glyphs[weight][name] = extract_path(path.read_text())
    return glyphs


def _js_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def gen_qml(glyphs: dict[str, dict[str, str]]) -> str:
    lines = [
        "// SPDX-License-Identifier: MIT",
        f"// {HEADER}",
        "pragma Singleton",
        "import QtQuick",
        "",
        "// The vendored Phosphor glyph vocabulary (T-19.1a). `path(weight, name)`",
        "// returns the PathSvg path data for a glyph, or an empty string for a",
        "// name/weight that is not vendored — the visual-regression test renders",
        "// every entry, and `scripts/check-phosphor-icons.py` fails the build if a",
        "// QML source references a glyph that is not here.",
        "QtObject {",
        f"    readonly property real viewBox: {VIEW_BOX:g}",
        f"    readonly property var weights: {list(WEIGHTS)}".replace("'", '"'),
        "",
    ]
    for weight in WEIGHTS:
        lines.append(f"    readonly property var {weight}: ({{")
        for name in sorted(glyphs[weight]):
            lines.append(f"        {_js_string(name)}: {_js_string(glyphs[weight][name])},")
        lines.append("    })")
        lines.append("")
    lines += [
        "    // Resolve (weight, name) to path data. `\"\"` means the glyph is not",
        "    // vendored; callers must never silently substitute another glyph.",
        "    function path(weight, name) {",
        "        var map = weight === \"fill\" ? fill",
        "                : (weight === \"regular\" || weight === undefined || weight === \"\")",
        "                  ? regular : null;",
        "        if (map === null)",
        "            return \"\";",
        "        var value = map[name];",
        "        return value === undefined ? \"\" : value;",
        "    }",
        "",
        "    function has(weight, name) {",
        "        return path(weight, name) !== \"\";",
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="verify the committed registry is current")
    args = parser.parse_args()

    glyphs = load_glyphs()
    text = gen_qml(glyphs)
    total = sum(len(v) for v in glyphs.values())

    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"stale generated glyph registry: {OUT.relative_to(REPO)}", file=sys.stderr)
            print("run scripts/gen-phosphor-glyphs.py and commit the result", file=sys.stderr)
            return 1
        print(f"phosphor glyph registry is current ({total} glyphs)")
        return 0

    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(REPO)} ({total} glyphs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())