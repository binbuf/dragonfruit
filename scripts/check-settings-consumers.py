#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Settings consumer conformance gate (T304, ADR 0316).

A settings key marked ``stored_only: false`` is a promise that the user's
change has an observable effect. This checker reads the authoritative schema
from ``services/settingsd/src/schema.rs`` and requires every live key to have
an entry in ``scripts/settings-consumers.json`` with:

    consumer  the component that reads the key and reacts to its Changed signal
    test      a test (or test file) that observes the effect
    state     "verified" or "known-gap"
    task      the task that owns the claim; for a known gap, the task that
              will remove the entry

A live key with no entry fails. A ``known-gap`` entry whose task is already
marked done in the task-set roadmap fails too: once the fix lands, the table
must stop claiming the key is still a gap.

The checker is deliberately a parser, not a second schema. It fails closed:
an unreadable schema, a malformed KeySpec, a missing table, or an empty key set
is an error, not a pass.

Run directly:

    python3 scripts/check-settings-consumers.py

or through ``make check-settings-consumers``. ``--write-known-gaps`` writes the
current known-gap list for the T304 baseline target (local, gitignored).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SCHEMA = ROOT / "services" / "settingsd" / "src" / "schema.rs"
DEFAULT_TABLE = ROOT / "scripts" / "settings-consumers.json"
DEFAULT_ROADMAP = ROOT / "docs" / "tasks" / "stability" / "plan" / "ROADMAP.md"

STATES = {"verified", "known-gap"}
TASK_RE = re.compile(r"^T\d+[a-z]?$")
LANDED_RE = re.compile(r"^\s*-\s*\[[xX]\]\s*(T\d+[a-z]?)\b", re.MULTILINE)
KEY_RE = re.compile(r'\bkey:\s*"([^"]*)"')
CONSUMER_RE = re.compile(r'\bconsumer:\s*"([^"]*)"')
STORED_ONLY_RE = re.compile(r"\bstored_only:\s*(true|false)\b")


class CheckError(Exception):
    """A schema/table problem that must fail the gate."""


def _matching(text: str, start: int, opener: str, closer: str) -> int:
    """Return the index of the brace/bracket matching ``text[start]``.

    Strings and their escapes are skipped so a summary containing a brace
    cannot end a KeySpec early.
    """
    depth = 0
    in_string = False
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char == opener:
            depth += 1
        elif char == closer:
            depth -= 1
            if depth == 0:
                return index
    raise CheckError(f"unbalanced {opener}{closer} in schema")


def parse_schema(path: Path) -> list[dict]:
    """Parse every KeySpec block in ``KEYS`` into a small record."""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise CheckError(f"cannot read schema {path}: {exc}") from exc

    keys_at = text.find("pub const KEYS")
    if keys_at < 0:
        raise CheckError(f"{path}: cannot find `pub const KEYS`")
    # Skip the `&[KeySpec]` type annotation and start at the literal `&[`.
    assign_at = text.find("= &[", keys_at)
    if assign_at < 0:
        raise CheckError(f"{path}: cannot find the KEYS array literal")
    array_at = text.find("[", assign_at)
    if array_at < 0:
        raise CheckError(f"{path}: cannot find the KEYS array")
    array_end = _matching(text, array_at, "[", "]")
    array_text = text[array_at : array_end + 1]

    specs: list[dict] = []
    cursor = 0
    while True:
        start = array_text.find("KeySpec {", cursor)
        if start < 0:
            break
        brace = array_text.find("{", start)
        end = _matching(array_text, brace, "{", "}")
        block = array_text[start : end + 1]
        cursor = end + 1

        key_match = KEY_RE.search(block)
        consumer_match = CONSUMER_RE.search(block)
        stored_match = STORED_ONLY_RE.search(block)
        if not key_match or not consumer_match or not stored_match:
            raise CheckError(
                f"{path}: a KeySpec is missing key/consumer/stored_only "
                f"(block {len(specs) + 1})"
            )
        key = key_match.group(1)
        consumer = consumer_match.group(1)
        stored_only = stored_match.group(1) == "true"
        if not key:
            raise CheckError(f"{path}: a KeySpec has an empty key")
        if not stored_only and not consumer:
            raise CheckError(f"{path}: live key {key!r} has no consumer")
        specs.append(
            {"key": key, "consumer": consumer, "stored_only": stored_only}
        )

    if not specs:
        raise CheckError(f"{path}: parsed no KeySpec entries")
    live = [spec for spec in specs if not spec["stored_only"]]
    if not live:
        raise CheckError(f"{path}: parsed no live (stored_only: false) keys")

    seen: set[str] = set()
    for spec in specs:
        if spec["key"] in seen:
            raise CheckError(f"{path}: duplicate key {spec['key']!r}")
        seen.add(spec["key"])
    return specs


def load_table(path: Path) -> dict[str, dict]:
    """Load the JSON table, accepting either a key map or a list of entries."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise CheckError(f"cannot read consumer table {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise CheckError(f"{path}: invalid JSON: {exc}") from exc

    if not isinstance(data, dict):
        raise CheckError(f"{path}: the table must be a JSON object")

    raw_entries = data.get("keys")
    if isinstance(raw_entries, dict):
        entries: dict[str, dict] = {}
        for key, entry in raw_entries.items():
            if not isinstance(entry, dict):
                raise CheckError(f"{path}: entry {key!r} is not an object")
            entries[key] = entry
        return entries
    if isinstance(raw_entries, list):
        entries = {}
        for index, entry in enumerate(raw_entries):
            if not isinstance(entry, dict):
                raise CheckError(f"{path}: entry {index} is not an object")
            key = entry.get("key")
            if not isinstance(key, str) or not key:
                raise CheckError(f"{path}: entry {index} has no key")
            if key in entries:
                raise CheckError(f"{path}: duplicate entry {key!r}")
            entries[key] = entry
        return entries
    raise CheckError(f"{path}: `keys` must be an object or a list")


def landed_tasks(roadmap: Path) -> set[str]:
    """Tasks the harness has marked done in the task-set roadmap.

    The roadmap is local/gitignored; when it is absent there is no landed-task
    claim to check and the gate still runs the schema/table half.
    """
    try:
        text = roadmap.read_text(encoding="utf-8")
    except OSError:
        return set()
    return set(LANDED_RE.findall(text))


def check(
    schema_path: Path, table_path: Path, roadmap: Path
) -> tuple[list[dict], dict[str, dict], list[dict]]:
    specs = parse_schema(schema_path)
    table = load_table(table_path)
    landed = landed_tasks(roadmap)

    live = [spec for spec in specs if not spec["stored_only"]]
    live_names = {spec["key"] for spec in live}
    all_names = {spec["key"] for spec in specs}
    errors: list[str] = []

    for name in sorted(set(table) - live_names):
        if name in all_names:
            errors.append(
                f"{name}: stored-only keys must not be in the consumer table"
            )
        else:
            errors.append(f"{name}: table entry is not a schema key")

    known_gaps: list[dict] = []
    verified = 0
    for spec in live:
        key = spec["key"]
        entry = table.get(key)
        if entry is None:
            errors.append(f"{key}: live key has no consumer-table entry")
            continue

        consumer = entry.get("consumer")
        test = entry.get("test")
        state = entry.get("state")
        task = entry.get("task")

        if not isinstance(consumer, str) or not consumer.strip():
            errors.append(f"{key}: entry needs a non-empty `consumer`")
        if not isinstance(test, str):
            errors.append(f"{key}: entry needs a string `test`")
        if state not in STATES:
            errors.append(
                f"{key}: `state` must be one of {sorted(STATES)}, got {state!r}"
            )
        if not isinstance(task, str) or not TASK_RE.match(task):
            errors.append(f"{key}: `task` must look like T315, got {task!r}")
            continue

        if state == "verified":
            if not isinstance(test, str) or not test.strip():
                errors.append(f"{key}: a verified entry needs a named test")
            else:
                test_file = test.split("::", 1)[0]
                if not (ROOT / test_file).exists():
                    errors.append(
                        f"{key}: verified test {test_file!r} does not exist"
                    )
            verified += 1
        elif state == "known-gap":
            known_gaps.append(
                {
                    "key": key,
                    "consumer": consumer,
                    "test": test,
                    "task": task,
                }
            )
            if task in landed:
                errors.append(
                    f"{key}: known-gap names {task}, which is marked landed "
                    "in the roadmap; remove the entry or name the real owner"
                )

    if errors:
        raise CheckError("\n".join(errors))
    return specs, table, known_gaps


def write_known_gaps(path: Path, known_gaps: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# T304 — settings-consumer known gaps",
        "",
        "Generated by `make t304-stability-baseline` from",
        "`scripts/settings-consumers.json`. Local working note; not tracked.",
        "",
        "| Key | Consumer | Planned task |",
        "|---|---|---|",
    ]
    for gap in known_gaps:
        lines.append(
            f"| `{gap['key']}` | {gap['consumer']} | {gap['task']} |"
        )
    lines.append("")
    lines.append(f"{len(known_gaps)} known gap(s).")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--schema", type=Path, default=DEFAULT_SCHEMA, help="schema.rs to parse"
    )
    parser.add_argument(
        "--table", type=Path, default=DEFAULT_TABLE, help="consumer JSON table"
    )
    parser.add_argument(
        "--roadmap",
        type=Path,
        default=DEFAULT_ROADMAP,
        help="task-set roadmap used to detect a landed known-gap task",
    )
    parser.add_argument(
        "--write-known-gaps",
        type=Path,
        metavar="PATH",
        help="write the current known-gap table to PATH",
    )
    args = parser.parse_args(argv)

    try:
        specs, _table, known_gaps = check(
            args.schema, args.table, args.roadmap
        )
    except CheckError as exc:
        print(f"settings-consumer check failed:\n{exc}", file=sys.stderr)
        return 1

    if args.write_known_gaps:
        write_known_gaps(args.write_known_gaps, known_gaps)
        print(f"wrote {len(known_gaps)} known gap(s) to {args.write_known_gaps}")

    live = sum(1 for spec in specs if not spec["stored_only"])
    stored = len(specs) - live
    print(
        "settings-consumer check passed: "
        f"{live} live key(s) ({live - len(known_gaps)} verified, "
        f"{len(known_gaps)} known gap(s)), {stored} stored-only key(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
