#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Fail the build when a QML source names a Phosphor glyph that is not vendored.

`PhosphorIcon` resolves a glyph name to a vendored SVG; a typo would otherwise
ship as a blank mark. This gate walks the first-party QML, finds every literal
glyph name passed to a Phosphor-aware component, and fails if the corresponding
SVG is not under ``assets/icons/phosphor/``. It is the "exists or fail" half of
T-19.1a (the other half is the generated registry, checked by
``scripts/gen-phosphor-glyphs.py --check``).

Dynamic names (a bound expression, not a string literal) cannot be checked here
and are skipped; those must be covered by a test that renders the glyph.

Usage:
    scripts/check-phosphor-icons.py
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ASSET_DIR = REPO / "assets" / "icons" / "phosphor"
SCAN_ROOTS = ("design-system", "apps", "shell")
EXCLUDE_DIRS = {"build", "target", ".git", ".symphony"}

# Which string property carries the glyph name for each component.
COMPONENT_PROPERTIES = {
    "PhosphorIcon": "name",
    "SettingsCategoryIcon": "source",
}
DEFAULT_WEIGHT = "regular"
WEIGHTS = ("regular", "fill")


def vendored(weight: str, name: str) -> bool:
    suffix = "" if weight == DEFAULT_WEIGHT else f"-{weight}"
    return (ASSET_DIR / f"{name}{suffix}.svg").is_file()


def _strip_strings_and_comments(text: str) -> str:
    """Replace string/comment contents with spaces, preserving length."""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        ch = text[i]
        if ch == "/" and i + 1 < n and text[i + 1] == "/":
            while i < n and text[i] != "\n":
                out[i] = " "
                i += 1
        elif ch == "/" and i + 1 < n and text[i + 1] == "*":
            out[i] = out[i + 1] = " "
            i += 2
            while i + 1 < n and not (text[i] == "*" and text[i + 1] == "/"):
                out[i] = " "
                i += 1
            if i + 1 < n:
                out[i] = out[i + 1] = " "
                i += 2
        elif ch == '"':
            out[i] = " "
            i += 1
            while i < n and text[i] != '"':
                if text[i] == "\\" and i + 1 < n:
                    out[i] = out[i + 1] = " "
                    i += 2
                    continue
                out[i] = " "
                i += 1
            if i < n:
                out[i] = " "
                i += 1
        else:
            i += 1
    return "".join(out)


def component_blocks(text: str, component: str) -> list[str]:
    """Return the brace-balance body of every `<component> { ... }` block."""
    blocks: list[str] = []
    masked = _strip_strings_and_comments(text)
    for match in re.finditer(rf"\b{component}\s*\{{", masked):
        start = match.end() - 1
        depth, i, n = 0, start, len(masked)
        while i < n:
            if masked[i] == "{":
                depth += 1
            elif masked[i] == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(text[start + 1 : i])
                    break
            i += 1
    return blocks


def literal_property(block: str, prop: str) -> str | None:
    match = re.search(rf"\b{prop}\s*:\s*\"([^\"]*)\"", block)
    return match.group(1) if match else None


def scan_text(path: Path) -> list[str]:
    text = path.read_text()
    failures: list[str] = []
    for component, prop in COMPONENT_PROPERTIES.items():
        for block in component_blocks(text, component):
            name = literal_property(block, prop)
            if name is None:
                continue
            weight = literal_property(block, "weight") or DEFAULT_WEIGHT
            if weight not in WEIGHTS:
                failures.append(
                    f"{path.relative_to(REPO)}: {component} weight \"{weight}\" is not vendored")
            elif not vendored(weight, name):
                failures.append(
                    f"{path.relative_to(REPO)}: {component} glyph \"{name}\" "
                    f"(weight \"{weight}\") is not under assets/icons/phosphor/")
    return failures


def qml_files() -> list[Path]:
    files: list[Path] = []
    for root in SCAN_ROOTS:
        base = REPO / root
        if not base.is_dir():
            continue
        for path in base.rglob("*.qml"):
            if any(part in EXCLUDE_DIRS for part in path.parts):
                continue
            files.append(path)
    return sorted(files)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()

    if not ASSET_DIR.is_dir():
        print("assets/icons/phosphor is missing", file=sys.stderr)
        return 1

    failures: list[str] = []
    for path in qml_files():
        failures.extend(scan_text(path))

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print(f"phosphor glyph references resolve ({len(qml_files())} QML files scanned)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())