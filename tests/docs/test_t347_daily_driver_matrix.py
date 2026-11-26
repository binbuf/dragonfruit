#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""T347 regression tests for the daily-driver matrix report.

The matrix is a hardware task, so these tests pin the contract the runner must
keep on any host:

* every native row is PASS or OPEN with a reason, a rerun command, and an
  artifact path;
* a host without a free DRM seat gets OPEN rows, never skipped rows;
* a headless failure is FAIL and beats an intake PASS (fail closed);
* an intake PASS needs both a note and an artifact;
* the report and design 27 §5.2 name the runner and the intake file.
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "t347-daily-driver-matrix.py"
DESIGN = ROOT / "docs" / "design" / "27-stability-and-daily-driver.md"
MAKEFILE = ROOT / "Makefile"

spec = importlib.util.spec_from_file_location("t347_matrix", SCRIPT)
matrix = importlib.util.module_from_spec(spec)
assert spec.loader is not None
sys.modules["t347_matrix"] = matrix
spec.loader.exec_module(matrix)


def check(check_id: str, verdict: str, detail: str = "ok") -> matrix.HeadlessResult:
    return matrix.HeadlessResult(
        id=check_id,
        title=check_id,
        command=f"run {check_id}",
        verdict=verdict,
        detail=detail,
        artifact=f"docs/captures/t347/logs/{check_id}.log",
    )


def all_checks(verdict: str = matrix.PASS) -> dict[str, matrix.HeadlessResult]:
    ids = [check_id for check_id, _, _, _ in matrix._check_defs(3)]
    return {check_id: check(check_id, verdict) for check_id in ids}


def no_seat() -> matrix.SeatState:
    return matrix.SeatState(
        available=False,
        reason="no free DRM seat (/dev/dri/card1 busy (Permission denied))",
        probe={"cards": [{"path": "/dev/dri/card1", "can_set_master": False}]},
    )


def free_seat() -> matrix.SeatState:
    return matrix.SeatState(
        available=True,
        reason="free DRM master on /dev/dri/card0",
        probe={"free_card": "/dev/dri/card0"},
    )


def intake(rows: dict[str, dict] | None = None, profile: str = "reference-A") -> matrix.Intake:
    return matrix.Intake(
        present=True,
        path=ROOT / "docs" / "captures" / "t347-native-evidence.json",
        profile=profile,
        recorded="2026-11-25T10:00:00Z",
        rows=rows or {},
    )


def hardware() -> dict[str, str]:
    return {
        "host": "Linux test",
        "distro": "Test Linux",
        "seat": "no free DRM seat",
        "evidence profile": "host only",
    }


class RowContractTests(unittest.TestCase):
    def test_row_ids_are_unique(self) -> None:
        ids = [row.id for row in matrix.ROW_DEFS]
        self.assertEqual(len(ids), len(set(ids)))

    def test_every_row_has_a_rerun_command_and_artifact(self) -> None:
        for row in matrix.ROW_DEFS:
            self.assertTrue(row.rerun.strip(), row.id)
            self.assertTrue(row.native_artifact.strip(), row.id)
            self.assertTrue(row.requirement.strip(), row.id)

    def test_every_row_check_is_defined(self) -> None:
        defined = {check_id for check_id, _, _, _ in matrix._check_defs(3)}
        # The journal check is appended by run_matrix() because it scans the
        # logs the other checks write.
        defined.add("journal")
        for row in matrix.ROW_DEFS:
            for check_id in row.checks:
                self.assertIn(check_id, defined, f"{row.id} names {check_id}")

    def test_headless_check_ids_are_unique(self) -> None:
        ids = [check_id for check_id, _, _, _ in matrix._check_defs(3)]
        self.assertEqual(len(ids), len(set(ids)))


class SeatAndVerdictTests(unittest.TestCase):
    def test_no_seat_marks_every_native_row_open_with_a_command(self) -> None:
        rows = matrix.resolve_rows(no_seat(), all_checks(), matrix.Intake(present=False, path=ROOT / "x"))
        self.assertEqual(matrix.gate_verdict(rows), matrix.OPEN)
        self.assertEqual(matrix.counts(rows)[matrix.OPEN], len(matrix.ROW_DEFS))
        for row in rows:
            self.assertEqual(row.verdict, matrix.OPEN, row.id)
            self.assertIn("DRM seat", row.reason, row.id)
            self.assertTrue(row.command.strip(), row.id)
            self.assertTrue(row.artifact.strip(), row.id)

    def test_free_seat_without_intake_is_still_open(self) -> None:
        rows = matrix.resolve_rows(free_seat(), all_checks(), matrix.Intake(present=False, path=ROOT / "x"))
        self.assertEqual(matrix.gate_verdict(rows), matrix.OPEN)
        for row in rows:
            self.assertEqual(row.verdict, matrix.OPEN, row.id)
            self.assertIn("free DRM seat is available", row.reason, row.id)

    def test_headless_failure_fails_the_row_and_beats_intake_pass(self) -> None:
        checks = all_checks()
        checks["lock"] = check("lock", matrix.FAIL, "one lock test failed")
        rows = matrix.resolve_rows(
            free_seat(),
            checks,
            intake(
                {
                    "lock-unlock": {
                        "verdict": matrix.PASS,
                        "note": "50 cycles passed",
                        "artifact": "docs/captures/t347-native/lock-unlock.txt",
                    }
                }
            ),
        )
        lock = next(row for row in rows if row.id == "lock-unlock")
        self.assertEqual(lock.verdict, matrix.FAIL)
        self.assertIn("Headless proof failed", lock.reason)
        self.assertEqual(matrix.gate_verdict(rows), matrix.FAIL)

    def test_intake_pass_without_note_or_artifact_stays_open(self) -> None:
        rows = matrix.resolve_rows(
            free_seat(),
            all_checks(),
            intake({"cold-login": {"verdict": matrix.PASS, "note": "4.2 s"}}),
        )
        cold = next(row for row in rows if row.id == "cold-login")
        self.assertEqual(cold.verdict, matrix.OPEN)
        self.assertIn("without a note and artifact", cold.reason)

    def test_intake_pass_with_note_and_artifact_is_pass(self) -> None:
        rows = matrix.resolve_rows(
            free_seat(),
            all_checks(),
            intake(
                {
                    "cold-login": {
                        "verdict": matrix.PASS,
                        "note": "4.2 s from login to usable desktop",
                        "artifact": "docs/captures/t347-native/cold-login.txt",
                    }
                }
            ),
        )
        cold = next(row for row in rows if row.id == "cold-login")
        self.assertEqual(cold.verdict, matrix.PASS)
        self.assertEqual(cold.artifact, "docs/captures/t347-native/cold-login.txt")
        self.assertEqual(matrix.gate_verdict(rows), matrix.OPEN)

    def test_intake_fail_is_a_failure(self) -> None:
        rows = matrix.resolve_rows(
            free_seat(),
            all_checks(),
            intake(
                {
                    "battery": {
                        "verdict": matrix.FAIL,
                        "note": "drain above budget",
                        "artifact": "docs/captures/t347-native/battery.txt",
                    }
                }
            ),
        )
        battery = next(row for row in rows if row.id == "battery")
        self.assertEqual(battery.verdict, matrix.FAIL)
        self.assertEqual(matrix.gate_verdict(rows), matrix.FAIL)

    def test_all_pass_rows_make_the_gate_pass(self) -> None:
        rows = []
        for row in matrix.ROW_DEFS:
            rows.append(
                matrix.RowResult(
                    id=row.id,
                    title=row.title,
                    severity=row.severity,
                    owner=row.owner,
                    requirement=row.requirement,
                    verdict=matrix.PASS,
                    reason="native evidence",
                    command=row.rerun,
                    artifact=row.native_artifact,
                    checks=row.checks,
                )
            )
        self.assertEqual(matrix.gate_verdict(rows), matrix.PASS)


class IntakeTests(unittest.TestCase):
    def test_unknown_row_is_a_warning(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "intake.json"
            path.write_text(
                json.dumps(
                    {
                        "profile": "reference-A",
                        "rows": {
                            "cold-login": {
                                "verdict": "OPEN",
                                "note": "not run",
                            },
                            "not-a-row": {"verdict": "PASS", "note": "x", "artifact": "y"},
                        },
                    }
                )
            )
            loaded = matrix.load_intake(path)
            self.assertIn("cold-login", loaded.rows)
            self.assertEqual(len(loaded.warnings), 1)
            self.assertIn("not-a-row", loaded.warnings[0])

    def test_bad_verdict_raises(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "intake.json"
            path.write_text(json.dumps({"rows": {"cold-login": {"verdict": "MAYBE"}}}))
            with self.assertRaises(ValueError):
                matrix.load_intake(path)

    def test_missing_intake_is_not_present(self) -> None:
        loaded = matrix.load_intake(ROOT / "docs" / "captures" / "does-not-exist.json")
        self.assertFalse(loaded.present)
        self.assertEqual(loaded.rows, {})


class ReportTests(unittest.TestCase):
    def _rows(self) -> list[matrix.RowResult]:
        return matrix.resolve_rows(
            no_seat(),
            all_checks(),
            matrix.Intake(present=False, path=ROOT / "x"),
        )

    def test_report_contains_every_row_and_the_profile(self) -> None:
        rows = self._rows()
        report = matrix.render_report(
            generated="2026-11-25T00:00:00Z",
            hardware=hardware(),
            seat=no_seat(),
            checks=list(all_checks().values()),
            rows=rows,
            intake=matrix.Intake(present=False, path=ROOT / "x"),
            report_path=ROOT / "docs" / "captures" / "t347-daily-driver-matrix.md",
        )
        self.assertIn("## Hardware profile", report)
        self.assertIn("## Daily-driver matrix", report)
        self.assertIn("## Rerun", report)
        self.assertIn("No free DRM seat", report)
        for row in matrix.ROW_DEFS:
            self.assertIn(row.title, report)
        for row in rows:
            self.assertIn(f"**{row.verdict}**", report)

    def test_p0_failure_is_named_in_the_follow_ups(self) -> None:
        rows = matrix.resolve_rows(
            no_seat(),
            all_checks(),
            matrix.Intake(present=False, path=ROOT / "x"),
        )
        rows[0] = matrix.RowResult(
            id=rows[0].id,
            title=rows[0].title,
            severity="P0",
            owner=rows[0].owner,
            requirement=rows[0].requirement,
            verdict=matrix.FAIL,
            reason="native cold login failed",
            command=rows[0].command,
            artifact=rows[0].artifact,
            checks=rows[0].checks,
        )
        report = matrix.render_report(
            generated="2026-11-25T00:00:00Z",
            hardware=hardware(),
            seat=no_seat(),
            checks=list(all_checks().values()),
            rows=rows,
            intake=matrix.Intake(present=False, path=ROOT / "x"),
            report_path=ROOT / "docs" / "captures" / "t347-daily-driver-matrix.md",
        )
        self.assertIn("not waived", report)
        self.assertIn("**P0 `cold-login`**", report)

    def test_markdown_table_cells_escape_pipes(self) -> None:
        self.assertEqual(matrix._cell("a|b"), "a\\|b")
        self.assertEqual(matrix._cell("a\nb"), "a<br>b")


class WiringTests(unittest.TestCase):
    def test_makefile_exposes_the_target(self) -> None:
        text = MAKEFILE.read_text()
        self.assertIn("t347-daily-driver-matrix:", text)
        self.assertIn("scripts/t347-daily-driver-matrix.py", text)
        self.assertIn("check-t347-matrix:", text)
        self.assertIn("test_t347_daily_driver_matrix.py", text)

    def test_design_names_the_runner_and_intake(self) -> None:
        text = DESIGN.read_text()
        self.assertIn("make t347-daily-driver-matrix", text)
        self.assertIn("t347-native-evidence.json", text)
        self.assertIn("docs/captures/t347-daily-driver-matrix.md", text)


if __name__ == "__main__":
    unittest.main(verbosity=2)
