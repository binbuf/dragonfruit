#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Deferred-register conformance gate (T346, ADR 0357).

Every accepted P2 deferral is one row in the deferred register in
``docs/design/27-stability-and-daily-driver.md`` §7. A row must name:

    Deferred           the deferred capability
    Reason             why the daily-driver gate does not include it
    User impact        what the user cannot do while it is deferred
    Owner              the design doc or task that owns the promotion
    Promotion trigger  the condition (hardware, user, release) that promotes it
    STATUS             the open gap id in ``docs/design/STATUS.md``

The checker is deliberately a table parser, not a second roadmap. It fails
closed: a missing section, a malformed table, an empty field, an owner with no
document/task, an unknown or resolved STATUS id, a duplicate row, or a missing
review item is an error, not a pass.

Run directly:

    python3 scripts/check-deferred-register.py

or through ``make check-deferred-register`` (part of ``make verify-stability``).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_DESIGN = ROOT / "docs" / "design" / "27-stability-and-daily-driver.md"
DEFAULT_STATUS = ROOT / "docs" / "design" / "STATUS.md"

HEADING_RE = re.compile(r"^##\s+7\.\s+Deferred register\s*$", re.MULTILINE)
GAP_HEADING_RE = re.compile(r"^##\s+5\.\s+Gap register\s*$", re.MULTILINE)
NEXT_HEADING_RE = re.compile(r"^##\s+", re.MULTILINE)
STATUS_ID_RE = re.compile(r"^G-\d{2}$")
OWNER_RE = re.compile(r"(\.md\b|ADR\s*\d{3,4}\b|T\d+[a-z]?\b)")
PLACEHOLDER_RE = re.compile(r"\b(someone|somebody|tbd|todo|nobody)\b", re.IGNORECASE)

HEADERS = [
    "deferred",
    "reason",
    "user impact",
    "owner",
    "promotion trigger",
    "status",
]

# The 2026-11-22 review's P2 list. The register must cover every one of these;
# a new item can be added to the table without editing the checker.
REQUIRED_DEFERRED = [
    "IME / CJK input",
    "AT-SPI for shell chrome",
    "Night light, HDR, VRR UI, color management",
    "Multi-GPU / rotated sampled glass",
    "Direct scanout, cursor, and overlay evidence",
    "Mixed-DPI per-output chrome",
    "Search service",
    "Stage / tiling",
    "Controller remapping",
    "Printer drivers / job manager",
    "Wallpaper rotation",
    "Files column / gallery / preview",
    "Keyboard backlight",
    "Auto-brightness",
    "Audio balance",
    "Smart zoom",
    "Media playback keys",
]


class CheckError(Exception):
    """A register/status problem that must fail the gate."""


def canonical(text: str) -> str:
    """Normalize a cell or label for comparison without changing its words."""
    text = text.replace("\u2013", "-").replace("\u2014", "-")
    text = re.sub(r"[*_`]", "", text)
    return re.sub(r"\s+", " ", text).strip().lower()


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise CheckError(f"cannot read {path}: {exc}") from exc


def section(text: str, heading: re.Pattern[str], name: str) -> str:
    """Return the text between ``heading`` and the next level-2/3 heading."""
    match = heading.search(text)
    if match is None:
        raise CheckError(f"cannot find the {name} heading")
    start = match.end()
    next_match = NEXT_HEADING_RE.search(text, start)
    end = next_match.start() if next_match else len(text)
    return text[start:end]


def table_rows(text: str, name: str) -> list[str]:
    """Return the first Markdown table in ``text`` as raw row lines."""
    lines = text.splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.strip().startswith("|"):
            start = index
            break
    if start is None:
        raise CheckError(f"{name} has no Markdown table")
    rows: list[str] = []
    for line in lines[start:]:
        if line.strip().startswith("|"):
            rows.append(line.strip())
        elif rows:
            break
    if len(rows) < 2:
        raise CheckError(f"{name} table needs a header and a separator")
    return rows


def split_row(line: str) -> list[str]:
    """Split one Markdown table row into stripped cells."""
    line = line.strip()
    if line.startswith("|"):
        line = line[1:]
    if line.endswith("|"):
        line = line[:-1]
    return [cell.strip() for cell in line.split("|")]


def parse_deferred_table(text: str) -> list[dict[str, str]]:
    """Parse design 27 §7 into one dict per deferred row."""
    register = section(text, HEADING_RE, "deferred register (design 27 §7)")
    rows = table_rows(register, "deferred register")
    headers = [canonical(cell) for cell in split_row(rows[0])]
    if headers != HEADERS:
        raise CheckError(
            "deferred register headers must be "
            f"{', '.join(HEADERS)}; got {', '.join(headers) or '<none>'}"
        )

    separator = split_row(rows[1])
    if len(separator) != len(HEADERS) or any(
        not re.fullmatch(r":?-{3,}:?", cell) for cell in separator
    ):
        raise CheckError("deferred register separator row is malformed")

    parsed: list[dict[str, str]] = []
    for line in rows[2:]:
        cells = split_row(line)
        if len(cells) != len(HEADERS):
            raise CheckError(
                f"deferred register row has {len(cells)} cells, expected "
                f"{len(HEADERS)}: {line}"
            )
        parsed.append(dict(zip(HEADERS, cells)))
    if not parsed:
        raise CheckError("deferred register has no rows")
    return parsed


def parse_status_gaps(text: str) -> dict[str, dict[str, str]]:
    """Return the open gap rows in STATUS.md §5, keyed by gap id.

    The review's deferrals include items that STATUS classifies as P0 or P1
    product risk (for example mixed-DPI chrome and multi-GPU glass), so the
    register accepts any gap row in §5, not only the P2 subsection.
    """
    register = section(text, GAP_HEADING_RE, "STATUS gap register")
    gaps: dict[str, dict[str, str]] = {}
    for line in register.splitlines():
        if not line.strip().startswith("|"):
            continue
        cells = split_row(line)
        if not cells or not STATUS_ID_RE.fullmatch(cells[0].strip()):
            continue
        gap_id = cells[0].strip()
        if gap_id in gaps:
            raise CheckError(f"STATUS gap register has a duplicate {gap_id} row")
        gap_text = cells[1] if len(cells) > 1 else ""
        gaps[gap_id] = {
            "gap": gap_text,
            "text": line.strip(),
            "resolved": canonical(gap_text).startswith("resolved"),
        }
    if not gaps:
        raise CheckError("STATUS gap register has no gap rows")
    return gaps


def check(
    design_path: Path = DEFAULT_DESIGN, status_path: Path = DEFAULT_STATUS
) -> tuple[list[dict[str, str]], dict[str, dict[str, str]]]:
    """Parse and validate the register; return the rows and STATUS gaps."""
    design = read_text(design_path)
    status = read_text(status_path)
    rows = parse_deferred_table(design)
    gaps = parse_status_gaps(status)

    errors: list[str] = []
    seen: dict[str, str] = {}
    for index, row in enumerate(rows, start=1):
        label = canonical(row["deferred"])
        if not label:
            errors.append(f"row {index}: `Deferred` is empty")
            continue
        if label in seen:
            errors.append(
                f"row {index}: duplicate deferred item {row['deferred']!r}"
            )
        seen[label] = row["deferred"]

        for field in HEADERS[1:]:
            if not row[field]:
                errors.append(f"{row['deferred']}: `{field}` is empty")

        if len(row["reason"]) < 15:
            errors.append(f"{row['deferred']}: `reason` is too short")
        if len(row["user impact"]) < 15:
            errors.append(f"{row['deferred']}: `user impact` is too short")
        if len(row["promotion trigger"]) < 15:
            errors.append(f"{row['deferred']}: `promotion trigger` is too short")

        owner = row["owner"]
        if owner and not OWNER_RE.search(owner):
            errors.append(
                f"{row['deferred']}: `owner` must name a document (.md), an "
                "ADR, or a task id"
            )
        if owner and PLACEHOLDER_RE.search(owner):
            errors.append(
                f"{row['deferred']}: `owner` must not be a placeholder"
            )

        status_id = row["status"]
        if status_id and not STATUS_ID_RE.fullmatch(status_id):
            errors.append(
                f"{row['deferred']}: `status` must look like G-20, got "
                f"{status_id!r}"
            )
        elif status_id:
            gap = gaps.get(status_id)
            if gap is None:
                errors.append(
                    f"{row['deferred']}: `status` {status_id} is not a gap row "
                    "in STATUS.md"
                )
            elif gap["resolved"]:
                errors.append(
                    f"{row['deferred']}: `status` {status_id} is a resolved "
                    "STATUS row; a deferral must name an open gap"
                )

    missing = [
        item
        for item in REQUIRED_DEFERRED
        if canonical(item) not in seen
    ]
    for item in missing:
        errors.append(f"deferred register is missing the review item {item!r}")

    if errors:
        raise CheckError("\n".join(errors))
    return rows, gaps


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--design",
        type=Path,
        default=DEFAULT_DESIGN,
        help="design 27 document containing §7",
    )
    parser.add_argument(
        "--status",
        type=Path,
        default=DEFAULT_STATUS,
        help="STATUS.md containing the gap register",
    )
    args = parser.parse_args(argv)

    try:
        rows, gaps = check(args.design, args.status)
    except CheckError as exc:
        print(f"deferred-register check failed:\n{exc}", file=sys.stderr)
        return 1

    status_ids = {row["status"] for row in rows}
    print(
        "deferred-register check passed: "
        f"{len(rows)} deferred item(s), every row with reason/impact/owner/"
        f"trigger/STATUS; {len(status_ids)} open STATUS gap row(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
