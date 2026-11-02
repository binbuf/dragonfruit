#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Fail the build when a QML source names a Tabler glyph that is not vendored.

`TablerIcon` resolves a glyph name to a vendored SVG; a typo would otherwise
ship as a blank mark. This gate walks the first-party QML, finds every literal
glyph name passed to `TablerIcon` (and every legacy name passed to the
`Icon` compatibility facade), and fails if the corresponding SVG is not under
``assets/icons/tabler/``. It also checks the dynamic vocabulary the services
and Files models emit, because those names are bound expressions the static
scan cannot see.

It is the "exists or fail" half of the Tabler adoption (the other half is the
generated registry, checked by ``scripts/gen-tabler-glyphs.py --check``).

Usage:
    scripts/check-tabler-icons.py
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ASSET_DIR = REPO / "assets" / "icons" / "tabler"
ICON_FACADE = REPO / "design-system" / "components" / "Icon.qml"
SCAN_ROOTS = ("design-system", "apps", "shell")
EXCLUDE_DIRS = {"build", "target", ".git", ".symphony"}

# Which string property carries the glyph name for each component. `Button`
# passes its literal `icon` through the `Icon` facade, so it shares the legacy
# alias check.
COMPONENT_PROPERTIES = {
    "TablerIcon": "name",
    "Icon": "name",
    "Button": "icon",
}
DEFAULT_WEIGHT = "outline"
WEIGHTS = ("outline", "filled")

# Names the services, Files models, and Control Center pass through the
# compatibility facade at runtime. They are bound expressions, so the literal
# scan cannot validate them; keep this list in sync with the producers
# (services/system-status, services/files-core, apps/files, shell/control-center).
DYNAMIC_LEGACY_NAMES = [
    # Files
    "home", "computer", "trash", "displays", "documents", "downloads",
    "wallpaper", "music", "movies", "volume", "folder", "file",
    "icon-view", "list-view",
    # shell chrome (dynamic names in Icon/StatusItem expressions)
    "brightness", "restore", "minimize", "close", "zoom", "application",
    "control-center", "mission-control", "appearance",
    # system-status / Control Center
    "accessibility", "users", "volume-muted", "bluetooth-disabled",
    "keyboard", "mouse", "trackpad", "touch", "tablet", "input", "lock",
    "menu-bar", "wifi", "wifi-secure", "wifi-disabled", "wifi-connecting",
    "wifi-error", "wifi-off", "vpn", "vpn-off", "focus", "overview",
    "power-saver", "power-balanced", "power-performance", "battery", "printer",
    "privacy", "drive-optical", "drive-removable-media", "drive-harddisk",
    "software-update", "bell",
]


def vendored(weight: str, name: str) -> bool:
    suffix = "-filled" if weight == "filled" else ""
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


def load_aliases() -> dict[str, str]:
    """Parse the legacy -> Tabler map out of the `Icon` facade."""
    text = ICON_FACADE.read_text()
    start = text.find("readonly property var aliases: ({")
    if start < 0:
        raise SystemExit(f"{ICON_FACADE.relative_to(REPO)}: aliases map not found")
    start = text.find("({", start) + 1
    depth, i, n = 0, start, len(text)
    end = -1
    while i < n:
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
        i += 1
    if end < 0:
        raise SystemExit(f"{ICON_FACADE.relative_to(REPO)}: aliases map is unbalanced")
    block = text[start:end]
    return {m.group(1): m.group(2)
            for m in re.finditer(r'"([^"]+)"\s*:\s*"([^"]+)"', block)}


def scan_text(path: Path, aliases: dict[str, str]) -> list[str]:
    text = path.read_text()
    failures: list[str] = []
    for component, prop in COMPONENT_PROPERTIES.items():
        for block in component_blocks(text, component):
            name = literal_property(block, prop)
            if name is None or name == "":
                continue
            if component != "TablerIcon":
                target = aliases.get(name)
                if target is None:
                    failures.append(
                        f"{path.relative_to(REPO)}: {component} glyph \"{name}\" has no "
                        f"Tabler mapping in design-system/components/Icon.qml")
                elif not vendored("outline", target):
                    failures.append(
                        f"{path.relative_to(REPO)}: {component} glyph \"{name}\" maps to "
                        f"Tabler \"{target}\", which is not under assets/icons/tabler/")
                continue
            weight = literal_property(block, "weight") or DEFAULT_WEIGHT
            if weight not in WEIGHTS:
                failures.append(
                    f"{path.relative_to(REPO)}: {component} weight \"{weight}\" is not vendored")
            elif not vendored(weight, name):
                failures.append(
                    f"{path.relative_to(REPO)}: {component} glyph \"{name}\" "
                    f"(weight \"{weight}\") is not under assets/icons/tabler/")
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
        print("assets/icons/tabler is missing", file=sys.stderr)
        return 1

    aliases = load_aliases()
    failures: list[str] = []

    # Every mapping must resolve to a vendored outline glyph, even if no QML
    # literal currently names it (dynamic service names rely on the map).
    for legacy, target in sorted(aliases.items()):
        if not vendored("outline", target):
            failures.append(
                f"design-system/components/Icon.qml: \"{legacy}\" -> \"{target}\" "
                f"is not under assets/icons/tabler/")
    for legacy in DYNAMIC_LEGACY_NAMES:
        if legacy not in aliases:
            failures.append(
                f"dynamic glyph \"{legacy}\" has no Tabler mapping in "
                f"design-system/components/Icon.qml")

    for path in qml_files():
        failures.extend(scan_text(path, aliases))

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print(f"tabler glyph references resolve ({len(qml_files())} QML files scanned, "
          f"{len(aliases)} legacy aliases)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
