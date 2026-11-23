#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T304 regression tests for the settings-consumer gate.

The gate itself must fail closed: a new live key without a table entry, a
stored-only key in the table, and a known-gap entry whose task has landed are
all errors. These tests drive the parser with small temporary schemas so they
do not depend on the size of the real ``schema.rs``.
"""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "check-settings-consumers.py"

spec = importlib.util.spec_from_file_location("check_settings_consumers", SCRIPT)
gate = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(gate)

SCHEMA = """
pub const KEYS: &[KeySpec] = &[
    KeySpec {
        key: "test.live",
        consumer: "test/consumer",
        stored_only: false,
    },
    KeySpec {
        key: "test.stored",
        consumer: "test/consumer",
        stored_only: true,
    },
];
"""


class SettingsConsumerGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.dir = Path(self.temp.name)
        self.schema = self.dir / "schema.rs"
        self.schema.write_text(SCHEMA, encoding="utf-8")
        self.table = self.dir / "settings-consumers.json"
        self.roadmap = self.dir / "ROADMAP.md"
        self.roadmap.write_text("- [ ] T315 — not landed\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_table(self, entries: dict) -> None:
        self.table.write_text(
            json.dumps({"schema": 1, "keys": entries}), encoding="utf-8"
        )

    def entry(self, **overrides) -> dict:
        entry = {
            "consumer": "test/consumer",
            "test": "tests/settings/test_settings_consumers.py",
            "state": "verified",
            "task": "T304",
        }
        entry.update(overrides)
        return entry

    def test_current_tree_passes(self) -> None:
        specs, _table, gaps = gate.check(
            ROOT / "services/settingsd/src/schema.rs",
            ROOT / "scripts/settings-consumers.json",
            ROOT / "docs/tasks/stability/plan/ROADMAP.md",
        )
        live = [row for row in specs if not row["stored_only"]]
        # T320 closed the last pointer/backlight known gaps and made the
        # libinput-appliable pointer family keys live.
        self.assertEqual(len(live), 174)
        self.assertEqual(len(gaps), 0)

    def test_live_key_without_entry_fails(self) -> None:
        self.write_table({})
        with self.assertRaisesRegex(gate.CheckError, "test.live"):
            gate.check(self.schema, self.table, self.roadmap)

    def test_stored_only_key_in_table_fails(self) -> None:
        self.write_table(
            {
                "test.live": self.entry(),
                "test.stored": self.entry(),
            }
        )
        with self.assertRaisesRegex(gate.CheckError, "stored-only"):
            gate.check(self.schema, self.table, self.roadmap)

    def test_known_gap_with_landed_task_fails(self) -> None:
        self.write_table(
            {
                "test.live": self.entry(state="known-gap", task="T315"),
            }
        )
        self.roadmap.write_text("- [x] T315 — landed\n", encoding="utf-8")
        with self.assertRaisesRegex(gate.CheckError, "marked landed"):
            gate.check(self.schema, self.table, self.roadmap)

    def test_known_gap_with_pending_task_passes(self) -> None:
        self.write_table(
            {
                "test.live": self.entry(state="known-gap", task="T315"),
            }
        )
        _specs, _table, gaps = gate.check(self.schema, self.table, self.roadmap)
        self.assertEqual([gap["key"] for gap in gaps], ["test.live"])

    def test_verified_entry_needs_an_existing_test(self) -> None:
        self.write_table(
            {
                "test.live": self.entry(test="tests/does-not-exist.py"),
            }
        )
        with self.assertRaisesRegex(gate.CheckError, "does not exist"):
            gate.check(self.schema, self.table, self.roadmap)


if __name__ == "__main__":
    unittest.main()
