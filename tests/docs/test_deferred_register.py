#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T346 regression tests for the deferred-register gate.

The gate must fail closed: a missing field, a placeholder owner, an unknown or
resolved STATUS row, a duplicate deferral, and a missing review item are all
errors. Most cases mutate a small generated register so the tests do not depend
on the current size of the real table.
"""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "check-deferred-register.py"
DESIGN = ROOT / "docs" / "design" / "27-stability-and-daily-driver.md"
STATUS = ROOT / "docs" / "design" / "STATUS.md"

spec = importlib.util.spec_from_file_location("check_deferred_register", SCRIPT)
gate = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(gate)

STATUS_BY_ITEM = {
    "IME / CJK input": "G-20",
    "AT-SPI for shell chrome": "G-19",
    "Night light, HDR, VRR UI, color management": "G-13",
    "Multi-GPU / rotated sampled glass": "G-08",
    "Direct scanout, cursor, and overlay evidence": "G-03",
    "Mixed-DPI per-output chrome": "G-02",
    "Search service": "G-17",
    "Stage / tiling": "G-23",
    "Controller remapping": "G-17",
    "Printer drivers / job manager": "G-17",
    "Wallpaper rotation": "G-24",
    "Files column / gallery / preview": "G-25",
    "Keyboard backlight": "G-26",
    "Auto-brightness": "G-13",
    "Audio balance": "G-27",
    "Smart zoom": "G-28",
    "Media playback keys": "G-29",
}


def fixture_rows() -> list[dict[str, str]]:
    return [
        {
            "deferred": item,
            "reason": "A concrete reason this item is outside the gate.",
            "user impact": "A concrete statement of the user-visible impact.",
            "owner": "docs/design/27-stability-and-daily-driver.md",
            "promotion trigger": "A concrete hardware, user, or release trigger.",
            "status": STATUS_BY_ITEM[item],
        }
        for item in gate.REQUIRED_DEFERRED
    ]


def render_design(rows: list[dict[str, str]], headers: list[str] | None = None) -> str:
    headers = headers or gate.HEADERS
    lines = [
        "# Test design",
        "",
        "## 7. Deferred register",
        "",
        "| " + " | ".join(headers) + " |",
        "|" + "---|" * len(headers),
    ]
    for row in rows:
        lines.append("| " + " | ".join(row.get(h, "") for h in gate.HEADERS) + " |")
    return "\n".join(lines) + "\n"


def render_status(
    ids: list[str],
    resolved: set[str] | None = None,
    p0: list[str] | None = None,
) -> str:
    resolved = resolved or set()
    p0 = p0 or []
    lines = ["# Test status", "", "## 5. Gap register", ""]
    if p0:
        lines += [
            "### P0 — blockers",
            "",
            "| ID | Gap | Evidence | Proposed action |",
            "|---|---|---|---|",
        ]
        for gap_id in p0:
            lines.append(f"| {gap_id} | Open blocker. | test | fix |")
        lines.append("")
    lines += [
        "### P2 — deliberate deferrals or polish gaps",
        "",
        "| ID | Gap | Evidence | Proposed action |",
        "|---|---|---|---|",
    ]
    for gap_id in ids:
        text = "Resolved (T000)." if gap_id in resolved else "Deferred fixture row."
        lines.append(f"| {gap_id} | {text} | test | keep deferred |")
    return "\n".join(lines) + "\n"


def status_ids() -> list[str]:
    return sorted(set(STATUS_BY_ITEM.values()))


class DeferredRegisterGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.dir = Path(self.temp.name)
        self.design = self.dir / "27.md"
        self.status = self.dir / "STATUS.md"
        self.rows = fixture_rows()
        self.write_status(status_ids())

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_design(
        self,
        rows: list[dict[str, str]] | None = None,
        headers: list[str] | None = None,
    ) -> None:
        self.design.write_text(
            render_design(rows if rows is not None else self.rows, headers),
            encoding="utf-8",
        )

    def write_status(
        self,
        ids: list[str],
        resolved: set[str] | None = None,
        p0: list[str] | None = None,
    ) -> None:
        self.status.write_text(
            render_status(ids, resolved, p0), encoding="utf-8"
        )

    def test_current_tree_passes(self) -> None:
        rows, gaps = gate.check(DESIGN, STATUS)
        self.assertGreaterEqual(len(rows), len(gate.REQUIRED_DEFERRED))
        labels = {gate.canonical(row["deferred"]) for row in rows}
        for item in gate.REQUIRED_DEFERRED:
            self.assertIn(gate.canonical(item), labels)
        self.assertTrue(gaps)

    def test_fixture_passes(self) -> None:
        self.write_design()
        rows, _gaps = gate.check(self.design, self.status)
        self.assertEqual(len(rows), len(gate.REQUIRED_DEFERRED))

    def test_missing_field_fails(self) -> None:
        self.rows[0]["owner"] = ""
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "`owner` is empty"):
            gate.check(self.design, self.status)

    def test_placeholder_owner_fails(self) -> None:
        self.rows[0]["owner"] = "someone (docs/design/27-stability-and-daily-driver.md)"
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "placeholder"):
            gate.check(self.design, self.status)

    def test_owner_without_a_document_or_task_fails(self) -> None:
        self.rows[0]["owner"] = "the platform team"
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "must name a document"):
            gate.check(self.design, self.status)

    def test_unknown_status_fails(self) -> None:
        self.rows[0]["status"] = "G-99"
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "not a gap row"):
            gate.check(self.design, self.status)

    def test_status_must_exist_in_the_register(self) -> None:
        # G-01 is not in the fixture register.
        self.rows[0]["status"] = "G-01"
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "not a gap row"):
            gate.check(self.design, self.status)

    def test_p0_status_row_is_accepted(self) -> None:
        self.write_status(status_ids(), p0=["G-01"])
        self.rows[0]["status"] = "G-01"
        self.write_design()
        rows, _gaps = gate.check(self.design, self.status)
        self.assertEqual(rows[0]["status"], "G-01")

    def test_resolved_status_fails(self) -> None:
        self.write_status(status_ids() + ["G-16"], resolved={"G-16"})
        self.rows[0]["status"] = "G-16"
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "resolved STATUS row"):
            gate.check(self.design, self.status)

    def test_missing_review_item_fails(self) -> None:
        self.rows.pop(0)
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "missing the review item"):
            gate.check(self.design, self.status)

    def test_duplicate_item_fails(self) -> None:
        self.rows.append(dict(self.rows[0]))
        self.write_design()
        with self.assertRaisesRegex(gate.CheckError, "duplicate deferred item"):
            gate.check(self.design, self.status)

    def test_malformed_row_fails(self) -> None:
        self.write_design()
        text = self.design.read_text(encoding="utf-8")
        lines = text.splitlines()
        for index in range(len(lines) - 1, -1, -1):
            if lines[index].startswith("|"):
                lines.insert(index + 1, "| short | row |")
                break
        self.design.write_text("\n".join(lines) + "\n", encoding="utf-8")
        with self.assertRaisesRegex(gate.CheckError, "cells, expected"):
            gate.check(self.design, self.status)

    def test_wrong_headers_fail(self) -> None:
        self.write_design(headers=gate.HEADERS[:-1])
        with self.assertRaisesRegex(gate.CheckError, "headers must be"):
            gate.check(self.design, self.status)

    def test_missing_section_fails(self) -> None:
        self.design.write_text("# No register here\n", encoding="utf-8")
        with self.assertRaisesRegex(gate.CheckError, "cannot find"):
            gate.check(self.design, self.status)


if __name__ == "__main__":
    unittest.main()
