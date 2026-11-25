#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T349 regression tests for the chrome stability fixture gate.

The gate is deliberately small: it must fail closed on an unexpected pass
(the owning task landed but the manifest was not flipped), an unexpected fail
(a green fixture regressed), a missing fixture, or an unlisted fixture.
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tests" / "chrome-stability" / "run_fixtures.py"
MANIFEST = ROOT / "tests" / "chrome-stability" / "expected-failures.json"
QML = ROOT / "shell" / "tests" / "tst_chromestability.qml"

spec = importlib.util.spec_from_file_location("chrome_stability_fixtures", SCRIPT)
assert spec.loader is not None
fixtures = importlib.util.module_from_spec(spec)
sys.modules["chrome_stability_fixtures"] = fixtures
spec.loader.exec_module(fixtures)


class ManifestTests(unittest.TestCase):
    def test_manifest_lists_every_qml_fixture(self) -> None:
        cases = fixtures.load_manifest(MANIFEST)
        qml_tests = set(
            re.findall(r"function\s+(test_[A-Za-z0-9_]+)\s*\(", QML.read_text())
        )
        self.assertEqual({case.test for case in cases}, qml_tests)

    def test_pre_fix_reproducers_name_their_owner_and_setup(self) -> None:
        cases = fixtures.load_manifest(MANIFEST)
        expected_failures = {case.test: case for case in cases
                             if case.expected == "fail"}
        self.assertEqual(
            set(expected_failures),
            {
                "test_chooser_arrow_is_inside_the_committed_crop",
                "test_chooser_material_region_excludes_the_arrow_gaps",
            },
        )
        for case in expected_failures.values():
            self.assertIn(case.owner, {"T351"})
            self.assertTrue(case.setup)
            self.assertTrue(case.symptom)

    def test_status_anchor_rows_cover_one_and_a_half_and_two_x(self) -> None:
        cases = fixtures.load_manifest(MANIFEST)
        status_rows = [case for case in cases
                       if case.test.startswith("test_status_anchor_")]
        self.assertTrue(status_rows)
        for case in status_rows:
            self.assertEqual(case.expected, "pass")
            self.assertEqual(case.scales, (1.0, 1.5, 2.0))

    def test_default_scale_is_one_x(self) -> None:
        cases = fixtures.load_manifest(MANIFEST)
        by_test = {case.test: case for case in cases}
        self.assertEqual(
            by_test["test_attached_menu_has_square_top_corners_on_a_patterned_underlay"].scales,
            (1.0,),
        )

    def test_invalid_scale_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "manifest.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "cases": [
                            {
                                "test": "test_one",
                                "expected": "pass",
                                "owner": "T349",
                                "setup": "one",
                                "symptom": "one",
                                "scales": [0],
                            },
                        ],
                    }
                )
            )
            with self.assertRaises(ValueError):
                fixtures.load_manifest(path)

    def test_duplicate_case_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "manifest.json"
            path.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "cases": [
                            {
                                "test": "test_one",
                                "expected": "pass",
                                "owner": "T349",
                                "setup": "one",
                                "symptom": "one",
                            },
                            {
                                "test": "test_one",
                                "expected": "pass",
                                "owner": "T349",
                                "setup": "one",
                                "symptom": "one",
                            },
                        ],
                    }
                )
            )
            with self.assertRaises(ValueError):
                fixtures.load_manifest(path)


class EvaluationTests(unittest.TestCase):
    def case(self, test: str, expected: str = "pass",
             owner: str = "T349") -> fixtures.ManifestCase:
        return fixtures.ManifestCase(test, expected, owner, "setup", "symptom")

    def test_expected_pass_and_expected_fail_are_green(self) -> None:
        problems = fixtures.evaluate(
            [self.case("test_pass"), self.case("test_fail", "fail", "T350")],
            {"test_pass": "pass", "test_fail": "fail"},
        )
        self.assertEqual(problems, [])

    def test_unexpected_fail_is_red(self) -> None:
        problems = fixtures.evaluate(
            [self.case("test_pass")], {"test_pass": "fail"}
        )
        self.assertEqual(len(problems), 1)
        self.assertIn("expected pass", str(problems[0]))

    def test_unexpected_pass_is_red_and_names_the_owner(self) -> None:
        problems = fixtures.evaluate(
            [self.case("test_fail", "fail", "T350")], {"test_fail": "pass"}
        )
        self.assertEqual(len(problems), 1)
        self.assertIn("T350", str(problems[0]))
        self.assertIn("flip", str(problems[0]))

    def test_missing_fixture_is_red(self) -> None:
        problems = fixtures.evaluate([self.case("test_pass")], {})
        self.assertEqual(len(problems), 1)
        self.assertIn("did not run", str(problems[0]))

    def test_unlisted_fixture_is_red(self) -> None:
        problems = fixtures.evaluate([], {"test_new": "pass"})
        self.assertEqual(len(problems), 1)
        self.assertIn("no manifest row", str(problems[0]))

    def test_parse_results_reads_qmltest_output(self) -> None:
        output = "\n".join(
            [
                "PASS   : tst_chromestability::ChromeStability::test_a()",
                "FAIL!  : tst_chromestability::ChromeStability::test_b() nope",
                "SKIP   : tst_chromestability::ChromeStability::test_c() later",
                "Totals: 1 passed, 1 failed, 1 skipped",
            ]
        )
        self.assertEqual(
            fixtures.parse_results(output),
            {"test_a": "pass", "test_b": "fail", "test_c": "skip"},
        )


if __name__ == "__main__":
    unittest.main()
