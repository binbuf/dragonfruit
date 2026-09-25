#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate the QML token singleton and the Rust token module from one source.

FR-2: tokens are defined once in ``design-system/tokens/tokens.json`` and
consumed everywhere. This script is the only writer of the two generated
artifacts; ``--check`` (run in CI / ``make check-tokens``) fails if either is
stale, so the QML and Rust token sets cannot drift apart by hand editing.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCE = REPO / "design-system" / "tokens" / "tokens.json"
QML_OUT = REPO / "design-system" / "Theme.qml"
RUST_OUT = REPO / "compositor" / "src" / "design_tokens.rs"

HEADER = "GENERATED FILE — DO NOT EDIT. Edit design-system/tokens/tokens.json and run scripts/gen-tokens.py."


def load() -> dict:
    with SOURCE.open() as handle:
        return json.load(handle)


def resolve(value, root: dict, trail: tuple[str, ...] = ()):
    """Resolve ``{"$ref": "primitive.color.accent"}`` chains."""
    if isinstance(value, dict) and "$ref" in value:
        ref = value["$ref"]
        if ref in trail:
            raise ValueError(f"cyclic token reference: {' -> '.join(trail + (ref,))}")
        node = root
        for part in ref.split("."):
            if not isinstance(node, dict) or part not in node:
                raise ValueError(f"unknown token reference: {ref}")
            node = node[part]
        return resolve(node, root, trail + (ref,))
    return value


def qml_literal(value) -> str:
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(qml_literal(v) for v in value) + "]"
    raise TypeError(f"cannot render {value!r} as QML")


def rust_ident(name: str) -> str:
    snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    snake = re.sub(r"([A-Za-z])([0-9])", r"\1_\2", snake)
    return snake.upper()


def rust_module(name: str) -> str:
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).lower()


def rust_literal(value, kind: str) -> str:
    if isinstance(value, str):
        if kind == "color" and re.fullmatch(r"#[0-9a-fA-F]{6}", value):
            r, g, b = (int(value[i : i + 2], 16) for i in (1, 3, 5))
            return f"[{r:#04x}, {g:#04x}, {b:#04x}, 0xff]"
        if kind == "color" and re.fullmatch(r"#[0-9a-fA-F]{8}", value):
            a, r, g, b = (int(value[i : i + 2], 16) for i in (1, 3, 5, 7))
            return f"[{r:#04x}, {g:#04x}, {b:#04x}, {a:#04x}]"
        return json.dumps(value)
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return f"{value}_u32" if kind == "duration" else f"{value}.0_f32"
    if isinstance(value, float):
        return repr(value) + "_f32"
    raise TypeError(f"cannot render {value!r} as Rust")


# ---------------------------------------------------------------------------
# QML
# ---------------------------------------------------------------------------


def qml_object(body: list[str], indent: int) -> list[str]:
    pad = " " * indent
    lines = [pad + "QtObject {"]
    for line in body:
        lines.append(" " * (indent + 4) + line)
    lines.append(pad + "}")
    return lines


# The accent family is overridable at runtime (T-09.2, `appearance.accent`).
# Every other semantic color stays a token literal; only these four consult
# the single top-level `accentOverride`, deriving hover/muted/on-content from
# it so a custom accent cannot leave a stale magenta hover or label.
ACCENT_OVERRIDE_KEYS = {"accent", "accentHover", "accentMuted", "accentContent"}


def accent_qml(key: str, value, scheme: str, muted_surface: str) -> str:
    """The QML binding for one accent-family scheme color."""
    fallback = qml_literal(value)
    if key == "accent":
        return f"tokens.hasAccentOverride ? tokens.accentOverrideColor : {fallback}"
    if key == "accentHover":
        return f"tokens.hasAccentOverride ? Qt.lighter(tokens.accentOverrideColor, 1.15) : {fallback}"
    if key == "accentMuted":
        # A low-alpha accent wash over the scheme's muted surface.
        return (f"tokens.hasAccentOverride ? Qt.tint({muted_surface}, "
                f"Qt.rgba(tokens.accentOverrideColor.r, tokens.accentOverrideColor.g, "
                f"tokens.accentOverrideColor.b, {0.28 if scheme == 'dark' else 0.14})) "
                f": {fallback}")
    # accentContent: pick a legible on-accent by luminance (0.5 threshold), so
    # a light custom accent gets dark text and a dark accent gets white.
    return ("tokens.hasAccentOverride ? (tokens.accentOverrideColor.r * 0.299 "
            "+ tokens.accentOverrideColor.g * 0.587 + tokens.accentOverrideColor.b * 0.114 > 0.5 "
            f'? "#130f17" : "#ffffff") : {fallback}')


def emit_qml_group(name: str, node: dict, root: dict, indent: int, kind: str,
                   scheme: str | None = None,
                   muted_surface: str = "transparent") -> list[str]:
    """Emit ``readonly property QtObject <name>: QtObject { ... }``."""
    pad = " " * indent
    lines = [f"{pad}readonly property var {name}: QtObject {{"]
    for key, raw in node.items():
        value = resolve(raw, root)
        if isinstance(value, dict):
            lines.extend(emit_qml_group(key, value, root, indent + 4, kind,
                                        scheme=scheme,
                                        muted_surface=muted_surface))
        else:
            if isinstance(value, list):
                prop = "var"
            elif isinstance(value, str):
                prop = "color" if re.fullmatch(r"#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?", value) else "string"
            elif isinstance(value, bool):
                prop = "bool"
            elif isinstance(value, int):
                prop = "int"
            else:
                prop = "real"
            if scheme is not None and key in ACCENT_OVERRIDE_KEYS:
                lines.append(" " * (indent + 4)
                             + f"readonly property {prop} {key}: "
                             + accent_qml(key, value, scheme, muted_surface))
            else:
                lines.append(" " * (indent + 4) + f"readonly property {prop} {key}: {qml_literal(value)}")
    lines.append(pad + "}")
    return lines


def gen_qml(data: dict) -> str:
    root = data
    lines = [
        "// SPDX-License-Identifier: MIT",
        f"// {HEADER}",
        "pragma Singleton",
        "import QtQuick",
        "",
        "QtObject {",
        "    id: tokens",
        "",
    ]

    lines.extend(emit_qml_group("primitive", data["primitive"], root, 4, "primitive"))
    lines.append("")

    for scheme in ("light", "dark"):
        node = data["semantic"][scheme]
        muted = resolve(node["color"]["surfaceMuted"], root)
        lines.extend(
            emit_qml_group(f"{scheme}Scheme", node, root, 4, "semantic",
                           scheme=scheme, muted_surface=qml_literal(muted))
        )
    lines.append("")

    lines.extend(
        [
            "    // Active scheme. The shell binds this to the host appearance;",
            "    // the gallery and tests set it directly. Assigning it breaks the",
            "    // binding, which is exactly what a preview toggle wants.",
            "    property bool dark: Application.styleHints.colorScheme === Qt.Dark",
            "    // Reduced motion is a first-class token (FR-5): every animation has",
            "    // a variant that removes translation/scale but keeps state legible.",
            "    property bool reducedMotion: false",
            "    // Accent override (T-09.2, `appearance.accent`): empty means the",
            "    // active scheme's token accent. The shell's ThemeBinding and the",
            "    // Settings app's local Theme bindings write it; the accent-family",
            "    // scheme colors above consult it, so every accent consumer follows",
            "    // without a per-component override.",
            "    property string accentOverride: \"\"",
            "    readonly property bool hasAccentOverride: tokens.accentOverride !== \"\"",
            "    readonly property color accentOverrideColor: tokens.hasAccentOverride ? tokens.accentOverride : \"transparent\"",
            "    readonly property var color: tokens.dark ? tokens.darkScheme.color : tokens.lightScheme.color",
            "    readonly property var material: tokens.dark ? tokens.darkScheme.material : tokens.lightScheme.material",
            "",
        ]
    )

    lines.extend(emit_qml_group("controls", data["component"], root, 4, "component"))
    lines.append("")

    # Motion is special: duration collapses to the reduced variant.
    lines.append("    readonly property var motion: QtObject {")
    for name, spec in data["motion"].items():
        duration = resolve(spec["duration"], root)
        reduced = resolve(spec.get("reducedDuration", 0), root)
        curve = resolve(spec["curve"], root)
        curve6 = list(curve) + [1.0, 1.0] if len(curve) == 4 else list(curve)
        lines.append(f"        readonly property QtObject {name}: QtObject {{")
        lines.append(f"            readonly property int fullDuration: {duration}")
        lines.append("            readonly property int duration: tokens.reducedMotion ? "
                     f"{reduced} : {duration}")
        lines.append(f"            readonly property var curve: {qml_literal(curve6)}")
        lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------


def emit_rust_group(name: str, node: dict, root: dict, indent: int, kind: str) -> list[str]:
    pad = " " * indent
    lines = [f"{pad}pub mod {rust_module(name)} {{"]
    for key, raw in node.items():
        value = resolve(raw, root)
        if isinstance(value, dict):
            lines.extend(emit_rust_group(key, value, root, indent + 4, kind))
        else:
            const = rust_ident(key)
            ty, lit = rust_type_literal(value, kind)
            lines.append(" " * (indent + 4) + f"pub const {const}: {ty} = {lit};")
    lines.append(pad + "}")
    return lines


def rust_type_literal(value, kind: str) -> tuple[str, str]:
    if isinstance(value, str):
        if re.fullmatch(r"#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?", value):
            return "[u8; 4]", rust_literal(value, "color")
        return "&str", rust_literal(value, kind)
    if isinstance(value, bool):
        return "bool", rust_literal(value, kind)
    if isinstance(value, int):
        return ("u32" if kind == "duration" else "f32"), rust_literal(value, kind)
    if isinstance(value, float):
        return "f32", rust_literal(value, kind)
    raise TypeError(f"cannot render {value!r} as Rust")


def gen_rust(data: dict) -> str:
    root = data
    lines = [
        "// SPDX-License-Identifier: MIT",
        f"//! {HEADER}",
        "//!",
        "//! Consumed by the compositor's server-side decoration renderer (T-13);",
        "//! because it is generated from the same source as the QML singleton, an",
        "//! app `TitleBar` and a compositor SSD titlebar cannot drift (FR-3).",
        "#![allow(dead_code)]",
        "",
    ]
    lines.extend(emit_rust_group("primitive", data["primitive"], root, 0, "primitive"))
    lines.append("")
    lines.append("pub mod semantic {")
    for scheme in ("light", "dark"):
        lines.extend(emit_rust_group(scheme, data["semantic"][scheme], root, 4, "semantic"))
    lines.append("}")
    lines.append("")
    lines.extend(emit_rust_group("component", data["component"], root, 0, "component"))
    lines.append("")
    lines.append("/// A named motion: full duration, cubic-bezier control points, and")
    lines.append("/// the reduced-motion duration (0 = state change is instant).")
    lines.append("#[derive(Debug, Clone, Copy, PartialEq)]")
    lines.append("pub struct Motion {")
    lines.append("    pub duration_ms: u32,")
    lines.append("    pub curve: [f32; 4],")
    lines.append("    pub reduced_duration_ms: u32,")
    lines.append("}")
    lines.append("")
    lines.append("pub mod motion {")
    lines.append("    use super::Motion;")
    for name, spec in data["motion"].items():
        duration = resolve(spec["duration"], root)
        reduced = resolve(spec.get("reducedDuration", 0), root)
        curve = resolve(spec["curve"], root)
        curve_lit = ", ".join(repr(float(v)) + "_f32" for v in curve)
        lines.append(f"    pub const {rust_ident(name)}: Motion = Motion {{")
        lines.append(f"        duration_ms: {duration}_u32,")
        lines.append(f"        curve: [{curve_lit}],")
        lines.append(f"        reduced_duration_ms: {reduced}_u32,")
        lines.append("    };")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify generated files are current")
    args = parser.parse_args()

    data = load()
    outputs = {
        QML_OUT: gen_qml(data),
        RUST_OUT: gen_rust(data),
    }

    if args.check:
        stale = [path for path, text in outputs.items() if not path.exists() or path.read_text() != text]
        if stale:
            for path in stale:
                print(f"stale generated token file: {path.relative_to(REPO)}", file=sys.stderr)
            print("run scripts/gen-tokens.py and commit the result", file=sys.stderr)
            return 1
        print("token generation is current")
        return 0

    for path, text in outputs.items():
        path.write_text(text)
        print(f"wrote {path.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
