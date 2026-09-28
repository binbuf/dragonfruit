#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""String-extraction gate and catalog generator (T-16.7).

The shell, the first-party apps, and the shared design-system components
externalize every user-visible string with Qt's `qsTr()` / `tr()` /
`QCoreApplication::translate()`. This script owns the one source of truth for
which strings those are:

  * `--check` (default) parses the sources, parses `translations/dragonfruit.ts`
    (the checked-in template), and fails when the two disagree — a new
    `qsTr("...")` with no template entry, or a stale template entry whose
    source string is gone. It also fails on a bare string literal assigned to a
    user-visible property (`text:`, `title:`, `Accessible.name:`, ...) that was
    never wrapped in `qsTr()`.
  * `--update` rewrites `translations/dragonfruit.ts` from the sources. The
    template lists every translatable string with an empty `<translation>`; the
    locale catalogs (`dragonfruit_es.ts`, ...) are the translated set and are
    maintained by hand.

The workspace intentionally does not depend on Qt's Linguist tools
(`lupdate`/`lrelease`); this extractor is the portable, CI-safe replacement and
the `.ts` files it writes are the standard format those tools understand.
"""

from __future__ import annotations

import argparse
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TRANSLATIONS = ROOT / "translations"
TEMPLATE = TRANSLATIONS / "dragonfruit.ts"

# Directories whose strings ship: the shell process, the first-party apps, and
# the shared component library they all render.
SCOPES = ("shell", "apps", "design-system/components")

STRING_LITERAL = r'"(?:[^"\\]|\\.)*"'
# `qsTr("a" + "b")` concatenation, as lupdate accepts it.
QML_QSTR = re.compile(r"qsTr\(\s*(" + STRING_LITERAL + r"(?:\s*\+\s*" + STRING_LITERAL + r")*)\s*\)")
CPP_TR = re.compile(r"(?<![A-Za-z0-9_])tr\(\s*(" + STRING_LITERAL + r")\s*\)")
CPP_TRANSLATE = re.compile(
    r'QCoreApplication::translate\(\s*("(?:[^"\\]|\\.)*")\s*,\s*('
    + STRING_LITERAL
    + r")\s*[,)]"
)
QT_TR_NOOP = re.compile(r"QT_TR_NOOP\(\s*(" + STRING_LITERAL + r")\s*\)")

# A user-visible QML property whose bare literal should have been externalized.
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
    out = []
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


def enclosing_class(text: str, position: int) -> str | None:
    """The nearest `class`/`struct` name declared before `position` (C++ `tr`)."""
    name = None
    for match in re.finditer(r"\b(?:class|struct)\s+([A-Za-z_]\w*)", text[:position]):
        name = match.group(1)
    return name


def extract_file(path: Path) -> list[tuple[str, str]]:
    text = path.read_text(encoding="utf-8")
    found: list[tuple[str, str]] = []
    stem = path.stem

    if path.suffix == ".qml":
        for match in QML_QSTR.finditer(text):
            found.append((stem, string_literals(match.group(1))))
        return found

    for match in CPP_TRANSLATE.finditer(text):
        context = unescape(match.group(1)[1:-1])
        found.append((context, unescape(match.group(2)[1:-1])))
    for match in CPP_TR.finditer(text):
        found.append((enclosing_class(text, match.start()) or stem,
                      unescape(match.group(1)[1:-1])))
    for match in QT_TR_NOOP.finditer(text):
        found.append((stem, unescape(match.group(1)[1:-1])))
    return found


def iter_sources():
    for scope in SCOPES:
        base = ROOT / scope
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file() or path.suffix not in {".qml", ".cpp", ".h"}:
                continue
            if "tests" in path.parts:
                continue
            yield path


def extracted() -> dict[str, set[str]]:
    by_context: dict[str, set[str]] = {}
    for path in iter_sources():
        for context, source in extract_file(path):
            if source:
                by_context.setdefault(context, set()).add(source)
    return by_context


def parse_template() -> dict[str, set[str]]:
    if not TEMPLATE.exists():
        return {}
    tree = ET.parse(TEMPLATE)
    by_context: dict[str, set[str]] = {}
    for context in tree.getroot().findall("context"):
        name = context.findtext("name") or ""
        sources = {message.findtext("source") or ""
                   for message in context.findall("message")}
        by_context[name] = {s for s in sources if s}
    return by_context


def write_template(by_context: dict[str, set[str]]) -> None:
    lines = [
        '<?xml version="1.0" encoding="utf-8"?>',
        "<!DOCTYPE TS>",
        '<TS version="2.1" language="en_US" sourcelanguage="en_US">',
    ]
    for context in sorted(by_context):
        lines.append("<context>")
        lines.append(f"    <name>{context}</name>")
        for source in sorted(by_context[context]):
            lines.append("    <message>")
            lines.append(f"        <source>{escape_xml(source)}</source>")
            lines.append('        <translation type="unfinished"></translation>')
            lines.append("    </message>")
        lines.append("</context>")
    lines.append("</TS>")
    lines.append("")
    TRANSLATIONS.mkdir(parents=True, exist_ok=True)
    TEMPLATE.write_text("\n".join(lines), encoding="utf-8")


def escape_xml(text: str) -> str:
    return (text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))


def bare_literals() -> list[tuple[Path, int, str]]:
    hits: list[tuple[Path, int, str]] = []
    for path in iter_sources():
        if path.suffix != ".qml":
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for number, line in enumerate(lines, 1):
            stripped = line.strip()
            if not stripped or stripped.startswith("//"):
                continue
            if "df-allow-untranslated" in line:
                continue
            if number >= 2 and "df-allow-untranslated" in lines[number - 2]:
                continue
            match = TRANSLATABLE_PROPERTY.match(line)
            if not match:
                continue
            value = unescape(match.group(1)[1:-1])
            # Decorative/placeholder content (empty, whitespace, glyph-only)
            # carries no language.
            if not any(ch.isalpha() for ch in value):
                continue
            hits.append((path, number, stripped))
    return hits


def report_completeness(by_context: dict[str, set[str]], template: dict[str, set[str]]) -> bool:
    status = True
    for context in sorted(set(by_context) | set(template)):
        live = by_context.get(context, set())
        known = template.get(context, set())
        missing = sorted(live - known)
        stale = sorted(known - live)
        if missing:
            status = False
            print(f"ERROR: {context}: {len(missing)} string(s) missing from the template:")
            for source in missing:
                print(f"    + {source!r}")
        if stale:
            status = False
            print(f"ERROR: {context}: {len(stale)} stale template string(s):")
            for source in stale:
                print(f"    - {source!r}")
    return status


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--update", action="store_true",
                      help="regenerate translations/dragonfruit.ts from the sources")
    mode.add_argument("--check", action="store_true",
                      help="fail when the sources and the template disagree (default)")
    args = parser.parse_args()

    by_context = extracted()
    total = sum(len(v) for v in by_context.values())

    if args.update:
        write_template(by_context)
        print(f"i18n-extract: wrote {TEMPLATE} ({total} strings, "
              f"{len(by_context)} contexts)")
        return 0

    status = True
    if not report_completeness(by_context, parse_template()):
        print("i18n-extract: run scripts/i18n-extract.py --update to refresh the template",
              file=sys.stderr)
        status = False

    bare = bare_literals()
    if bare:
        status = False
        print(f"ERROR: {len(bare)} user-visible literal(s) not wrapped in qsTr():")
        for path, number, line in bare:
            rel = path.relative_to(ROOT)
            print(f"    {rel}:{number}: {line}")

    if status:
        print(f"i18n-extract: OK — {total} translatable strings across "
              f"{len(by_context)} contexts, template in sync, no bare literals")
    return 0 if status else 1


if __name__ == "__main__":
    raise SystemExit(main())