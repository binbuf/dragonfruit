#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate the QML Tabler glyph registry from the vendored SVGs.

The vendored Tabler files under ``assets/icons/tabler/`` are the single source
of truth. Qt 6.11's only software-renderer-friendly tint path is a
``ShapePath`` with a bound ``strokeColor``/``fillColor``: the built-in
``MultiEffect`` colorization is GPU-only and silently no-ops on the headless
software backend the gallery and QML tests use (the same limit
``PhosphorIcon`` documents). This script extracts each glyph's path data once,
at build time, and emits one generated QML singleton that ``TablerIcon``
renders and tints.

Tabler outline SVGs are stroke-only (``stroke="currentColor"`` with
``fill="none"``); the pinned filled SVGs are fill-only. A few outline glyphs
carry one filled dot (for example ``accessible``), so the registry keeps a
separate fill overlay map for the outline weight.

Running with ``--check`` (``make check-tabler``) fails if the committed
registry is stale, so the SVGs and the registry cannot drift.

Usage:
    scripts/gen-tabler-glyphs.py            # (re)write the registry
    scripts/gen-tabler-glyphs.py --check    # verify it is current
"""

from __future__ import annotations

import argparse
import html
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ASSET_DIR = REPO / "assets" / "icons" / "tabler"
OUT = REPO / "design-system" / "TablerGlyphs.qml"

HEADER = ("GENERATED FILE — DO NOT EDIT. Edit assets/icons/tabler/ and run "
          "scripts/gen-tabler-glyphs.py.")

DEFAULT_WEIGHT = "outline"
WEIGHTS = ("outline", "filled")
VIEW_BOX = 24.0

_SVG = re.compile(r"<svg\b([^>]*)>", re.IGNORECASE | re.DOTALL)
_TAG = re.compile(
    r"<(path|line|polyline|polygon|circle|ellipse|rect)\b([^>]*?)/?>",
    re.IGNORECASE | re.DOTALL,
)
_ATTR = re.compile(r'([a-zA-Z-]+)\s*=\s*"([^"]*)"')


def _attrs(blob: str) -> dict[str, str]:
    return {m.group(1): html.unescape(m.group(2)) for m in _ATTR.finditer(blob)}


def _f(value: str | None, default: float = 0.0) -> float:
    try:
        return float(value)  # type: ignore[arg-type]
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


def _points_path(points: str, close: bool) -> str:
    values = [v for v in re.split(r"[\s,]+", points.strip()) if v]
    if len(values) < 4:
        return ""
    pairs = list(zip(values[0::2], values[1::2]))
    parts = [f"M {pairs[0][0]} {pairs[0][1]}"]
    parts += [f"L {x} {y}" for x, y in pairs[1:]]
    if close:
        parts.append("Z")
    return " ".join(parts)


def _element_path(tag: str, a: dict[str, str]) -> str:
    if tag == "path":
        return (a.get("d") or "").strip()
    if tag == "line":
        return (f"M {_f(a.get('x1'))} {_f(a.get('y1'))} "
                f"L {_f(a.get('x2'))} {_f(a.get('y2'))}")
    if tag == "polyline":
        return _points_path(a.get("points", ""), close=False)
    if tag == "polygon":
        return _points_path(a.get("points", ""), close=True)
    if tag == "circle":
        r = _f(a.get("r"))
        return _ellipse_path(_f(a.get("cx")), _f(a.get("cy")), r, r)
    if tag == "ellipse":
        return _ellipse_path(_f(a.get("cx")), _f(a.get("cy")),
                             _f(a.get("rx")), _f(a.get("ry")))
    if tag == "rect":
        return _rect_path(a)
    return ""


def extract_paths(svg_text: str) -> tuple[str, str]:
    """Return ``(stroke_path, fill_path)`` for a Tabler SVG."""
    root = _SVG.search(svg_text)
    if root is None:
        raise ValueError("missing <svg> root")
    root_attrs = _attrs(root.group(1))
    root_fill = root_attrs.get("fill", "none")
    root_stroke = root_attrs.get("stroke", "none")
    stroke: list[str] = []
    fill: list[str] = []

    for match in _TAG.finditer(svg_text):
        tag, blob = match.group(1).lower(), match.group(2)
        a = _attrs(blob)
        d = _element_path(tag, a)
        if not d:
            continue
        if "transform" in a:
            raise ValueError(f"unsupported transform on <{tag}>: {a['transform']}")

        explicit_fill = "fill" in a
        explicit_stroke = "stroke" in a
        effective_fill = a.get("fill", root_fill)
        effective_stroke = a.get("stroke", root_stroke)
        fills = effective_fill not in ("", "none")
        strokes = effective_stroke not in ("", "none")

        if explicit_fill and explicit_stroke:
            if fills:
                fill.append(d)
            if strokes:
                stroke.append(d)
        elif explicit_fill:
            if fills:
                fill.append(d)
        elif explicit_stroke:
            if strokes:
                stroke.append(d)
        elif fills and not strokes:
            fill.append(d)
        elif strokes and not fills:
            stroke.append(d)

    return " ".join(stroke), " ".join(fill)


def load_glyphs() -> dict[str, dict[str, tuple[str, str]]]:
    glyphs: dict[str, dict[str, tuple[str, str]]] = {
        weight: {} for weight in WEIGHTS
    }
    for path in sorted(ASSET_DIR.glob("*.svg")):
        stem = path.stem
        if stem.endswith("-filled"):
            weight, name = "filled", stem[: -len("-filled")]
        else:
            weight, name = "outline", stem
        stroke, fill = extract_paths(path.read_text())
        if weight == "filled" and stroke:
            raise SystemExit(
                f"{path.relative_to(REPO)}: filled glyphs must not contain strokes")
        if not stroke and not fill:
            raise SystemExit(f"{path.relative_to(REPO)}: no drawable path data")
        glyphs[weight][name] = (stroke, fill)
    return glyphs


def _js_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _map_lines(name: str, entries: dict[str, str]) -> list[str]:
    lines = [f"    readonly property var {name}: ({{"]
    for key in sorted(entries):
        value = entries[key]
        if value:
            lines.append(f"        {_js_string(key)}: {_js_string(value)},")
    lines.append("    })")
    lines.append("")
    return lines


def gen_qml(glyphs: dict[str, dict[str, tuple[str, str]]]) -> str:
    outline = {name: stroke for name, (stroke, _) in glyphs["outline"].items()}
    outline_fill = {name: fill for name, (_, fill) in glyphs["outline"].items()}
    filled = {name: fill for name, (_, fill) in glyphs["filled"].items()}

    lines = [
        "// SPDX-License-Identifier: MIT",
        f"// {HEADER}",
        "// Tabler Icons 3.50.0 (v3.50.0, 6904efd245391ad396df17654738f52f6da8c40e), MIT.",
        "pragma Singleton",
        "import QtQuick",
        "",
        "// The vendored Tabler line-glyph vocabulary. `path(weight, name)` returns",
        "// the PathSvg path data for a glyph, or an empty string for a name/weight",
        "// that is not vendored — the visual-regression test renders every entry,",
        "// and `scripts/check-tabler-icons.py` fails the build if a QML source",
        "// references a glyph that is not here.",
        "QtObject {",
        f"    readonly property real viewBox: {VIEW_BOX:g}",
        f"    readonly property var weights: {list(WEIGHTS)}".replace("'", '"'),
        "",
    ]
    lines += _map_lines("outline", outline)
    lines += _map_lines("outlineFill", outline_fill)
    lines += _map_lines("filled", filled)
    lines += [
        "    // Resolve (weight, name) to the primary path data. `\"\"` means the",
        "    // glyph is not vendored; callers must never silently substitute",
        "    // another glyph.",
        "    function path(weight, name) {",
        "        var map = weight === \"filled\" ? filled",
        "                : (weight === \"outline\" || weight === undefined || weight === \"\")",
        "                  ? outline : null;",
        "        if (map === null)",
        "            return \"\";",
        "        var value = map[name];",
        "        return value === undefined ? \"\" : value;",
        "    }",
        "",
        "    // The optional filled overlay for an outline glyph (e.g. the",
        "    // `accessible` head dot). Filled glyphs already return their solid",
        "    // path from `path()`, so this is empty for them.",
        "    function fillPath(weight, name) {",
        "        if (weight === \"filled\")",
        "            return \"\";",
        "        var value = outlineFill[name];",
        "        return value === undefined ? \"\" : value;",
        "    }",
        "",
        "    function has(weight, name) {",
        "        return path(weight, name) !== \"\" || fillPath(weight, name) !== \"\";",
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
            print(f"stale generated glyph registry: {OUT.relative_to(REPO)}",
                  file=sys.stderr)
            print("run scripts/gen-tabler-glyphs.py and commit the result",
                  file=sys.stderr)
            return 1
        print(f"tabler glyph registry is current ({total} glyphs)")
        return 0

    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(REPO)} ({total} glyphs)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
