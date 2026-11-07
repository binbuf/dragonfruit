#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""User-facing naming gate (T220).

The human naming map lives in ``docs/design/interface-naming-and-licensing.md``.
This checker enforces the machine-readable vocabulary in
``docs/design/naming-gate.json`` over the places a user can actually see a
string:

  * QML ``qsTr()`` strings and bare user-visible property literals;
  * C++ ``tr()``, ``QCoreApplication::translate()``, ``QT_TR_NOOP()``, and
    ``QStringLiteral``/``QLatin1String`` labels (internal keys are skipped);
  * ``*.desktop`` values (the launcher/session entry);
  * packaging metadata and manifests under ``packaging/``.

Design docs, source comments, internal identifiers, and test fixtures are not
user-facing and are deliberately out of scope. A line carrying
``df-allow-name`` (or the older ``df-allow-desktop-name``) is exempt, which is
how a negative test can spell a forbidden term on purpose.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_MAP = ROOT / "docs" / "design" / "naming-gate.json"

SKIP_DIRS = {
    ".git",
    ".docs",
    "build",
    "build-release",
    "node_modules",
    "target",
}
SOURCE_SCOPES = ("shell", "apps", "design-system")
PACKAGING_DIR = "packaging"
DESKTOP_GLOBS = ("*.desktop",)
PACKAGING_SUFFIXES = {
    "",
    ".conf",
    ".desktop",
    ".install",
    ".json",
    ".service",
    ".sh",
    ".spec",
    ".toml",
    ".xml",
    ".yaml",
    ".yml",
}
PACKAGING_NAMES = {"control", "PKGBUILD", "rules"}

STRING_LITERAL = r'"(?:[^"\\]|\\.)*"'
QML_QSTR = re.compile(
    r"qsTr\(\s*(" + STRING_LITERAL + r"(?:\s*\+\s*" + STRING_LITERAL + r")*)\s*\)"
)
CPP_TR = re.compile(r"(?<![A-Za-z0-9_])tr\(\s*(" + STRING_LITERAL + r")\s*\)")
CPP_TRANSLATE = re.compile(
    r'QCoreApplication::translate\(\s*("(?:[^"\\]|\\.)*")\s*,\s*('
    + STRING_LITERAL
    + r")\s*[,)]"
)
QT_TR_NOOP = re.compile(r"QT_TR_NOOP\(\s*(" + STRING_LITERAL + r")\s*\)")
CPP_QSTRINGLITERAL = re.compile(
    r"(?:QStringLiteral|QLatin1String)\(\s*(" + STRING_LITERAL + r")\s*\)"
)
# Internal keys/identifiers are not user-facing: `wallpaper.showOnAllSpaces`,
# `mission-control`, `screenReaderOn`. A literal that exactly equals a
# forbidden term (for example `iCloud`) is still checked.
KEYLIKE = re.compile(r"^(?:[a-z0-9]+(?:[._-][a-z0-9]+)+|[a-z][A-Za-z0-9]*)$")
TRANSLATABLE_PROPERTY = re.compile(
    r"^\s*(?:text|title|label|placeholderText|description|tooltip|"
    r"accessibleName|Accessible\.name|Accessible\.description):\s*("
    + STRING_LITERAL
    + r")\s*$"
)
ESCAPES = {
    "n": "\n",
    "t": "\t",
    "r": "\r",
    '"': '"',
    "\\": "\\",
    "'": "'",
    "0": "\0",
}


def unescape(token: str) -> str:
    """Decode a Qt/JS string literal body (without the surrounding quotes)."""
    out: list[str] = []
    i = 0
    while i < len(token):
        ch = token[i]
        if ch != "\\":
            out.append(ch)
            i += 1
            continue
        i += 1
        if i >= len(token):
            break
        esc = token[i]
        if esc == "u" and i + 4 < len(token):
            out.append(chr(int(token[i + 1 : i + 5], 16)))
            i += 5
        elif esc == "x" and i + 2 < len(token):
            out.append(chr(int(token[i + 1 : i + 3], 16)))
            i += 3
        elif esc in ESCAPES:
            out.append(ESCAPES[esc])
            i += 1
        else:
            out.append(esc)
            i += 1
    return "".join(out)


def string_literals(argument: str) -> str:
    """Join a `+`-separated run of string literals, unescaped."""
    return "".join(unescape(m.group(0)[1:-1]) for m in re.finditer(STRING_LITERAL, argument))


def line_of(text: str, position: int) -> int:
    return text.count("\n", 0, position) + 1


def qml_strings(text: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    for match in QML_QSTR.finditer(text):
        found.append((line_of(text, match.start()), string_literals(match.group(1))))
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("//"):
            continue
        match = TRANSLATABLE_PROPERTY.match(line)
        if match:
            found.append((number, unescape(match.group(1)[1:-1])))
    return found


def cpp_strings(text: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    for match in CPP_TRANSLATE.finditer(text):
        found.append((line_of(text, match.start()), unescape(match.group(2)[1:-1])))
    for match in CPP_TR.finditer(text):
        found.append((line_of(text, match.start()), unescape(match.group(1)[1:-1])))
    for match in QT_TR_NOOP.finditer(text):
        found.append((line_of(text, match.start()), unescape(match.group(1)[1:-1])))
    for match in CPP_QSTRINGLITERAL.finditer(text):
        found.append((line_of(text, match.start()), unescape(match.group(1)[1:-1])))
    return found


def desktop_strings(text: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith(("#", "[")):
            continue
        key, sep, value = stripped.partition("=")
        if sep:
            found.append((number, value))
    return found


def packaging_strings(text: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        found.append((number, stripped))
    return found


def iter_files(root: Path):
    for path in sorted(root.rglob("*")):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.is_file():
            yield path


def source_files(root: Path):
    for scope in SOURCE_SCOPES:
        base = root / scope
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file() or path.suffix not in {".qml", ".cpp", ".h"}:
                continue
            if "tests" in path.parts:
                continue
            yield path


def packaging_files(root: Path):
    base = root / PACKAGING_DIR
    if not base.is_dir():
        return
    for path in sorted(base.rglob("*")):
        if not path.is_file():
            continue
        if path.name in PACKAGING_NAMES or path.suffix in PACKAGING_SUFFIXES:
            yield path


def load_map(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema") != 1:
        raise ValueError(f"{path}: unsupported schema {data.get('schema')!r}")
    rows = data.get("forbid")
    if not isinstance(rows, list) or not rows:
        raise ValueError(f"{path}: no forbid rows")
    for row in rows:
        if not isinstance(row, dict) or not row.get("term") or not row.get("replacement"):
            raise ValueError(f"{path}: each forbid row needs term and replacement")
        pattern = row.get("pattern")
        if not pattern:
            raise ValueError(f"{path}: forbid row {row['term']!r} needs a pattern")
        try:
            compiled = re.compile(pattern)
        except re.error as exc:
            raise ValueError(f"{path}: bad pattern for {row['term']!r}: {exc}") from exc
        if not compiled.search(row["term"]):
            raise ValueError(
                f"{path}: pattern for {row['term']!r} does not match its own term"
            )
    return data


def scan(root: Path, map_data: dict) -> list[str]:
    rows = map_data["forbid"]
    compiled = [(row, re.compile(row["pattern"])) for row in rows]
    errors: list[str] = []

    def check(path: Path, strings: list[tuple[int, str]]) -> None:
        rel = path.relative_to(root)
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        for number, value in strings:
            source_line = lines[number - 1] if 0 < number <= len(lines) else ""
            if "df-allow-name" in source_line or "df-allow-desktop-name" in source_line:
                continue
            for row, pattern in compiled:
                if not pattern.search(value):
                    continue
                if KEYLIKE.match(value) and value != row["term"]:
                    continue
                errors.append(
                    f"{rel}:{number}: forbidden term {row['term']!r} "
                    f"(use {row['replacement']!r}): {source_line.strip()}"
                )

    for path in source_files(root):
        text = path.read_text(encoding="utf-8", errors="replace")
        strings = qml_strings(text) if path.suffix == ".qml" else cpp_strings(text)
        check(path, strings)

    for path in sorted(root.rglob("*.desktop")):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        # scripts/zoo is a third-party compatibility fixture, not our metadata.
        if "zoo" in path.parts:
            continue
        check(path, desktop_strings(path.read_text(encoding="utf-8", errors="replace")))

    for path in packaging_files(root):
        check(path, packaging_strings(path.read_text(encoding="utf-8", errors="replace")))

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    parser.add_argument("--map", type=Path, default=None, help="naming gate JSON")
    parser.add_argument("--quiet", action="store_true", help="only print failures")
    args = parser.parse_args()

    root = args.root.resolve()
    map_path = args.map or (root / "docs" / "design" / "naming-gate.json")
    try:
        map_data = load_map(map_path)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"check-user-facing-names: {exc}", file=sys.stderr)
        return 2

    errors = scan(root, map_data)
    if errors:
        print(f"check-user-facing-names: {len(errors)} forbidden user-facing term(s):")
        for error in errors:
            print(f"  ERROR: {error}")
        print(
            "check-user-facing-names: use the generic term from "
            "docs/design/interface-naming-and-licensing.md, or mark a deliberate "
            "negative-test line with df-allow-name",
            file=sys.stderr,
        )
        return 1

    if not args.quiet:
        print(
            f"check-user-facing-names: OK — {len(map_data['forbid'])} forbidden "
            "terms checked across shell/apps/design-system, desktop entries, and packaging"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
