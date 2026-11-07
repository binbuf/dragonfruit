#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Regression tests for the T219 experience schema and validator.

Run with ``make experience-schema-check`` or directly:

    python3 tests/experience/test_experience_schema.py

The test proves the validator *rejects* the failures the contract names:
missing axes, an unknown surface/state, a bad capture hash, an OPEN row
without a reason, a verification-kind/status mismatch, a dimension the surface
does not measure, and a threshold profile that drops a dimension. It also
validates the committed sample artifact and sample OPEN row.
"""
from __future__ import annotations

import copy
import importlib.util
import io
import json
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
SCRIPT = REPO / "scripts" / "t219-experience-schema.py"
FIXTURES = REPO / "tests" / "experience" / "fixtures"

spec = importlib.util.spec_from_file_location("t219_experience_schema", SCRIPT)
t219 = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(t219)


class ExperienceSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = t219.load_json(t219.SCHEMA)
        cls.manifest = t219.load_json(t219.MANIFEST)
        cls.thresholds = t219.load_json(t219.THRESHOLDS)
        cls.sample = t219.load_json(FIXTURES / "sample-artifact.json")
        cls.sample_open = t219.load_json(FIXTURES / "sample-open-artifact.json")

    def test_thresholds_are_complete(self):
        self.assertEqual(t219.check_thresholds(self.thresholds), [])

    def test_manifest_is_valid(self):
        errors, _ = t219.check_manifest(self.manifest, self.schema, self.thresholds)
        self.assertEqual(errors, [])

    def test_every_surface_state_has_an_entry(self):
        errors, data = t219.check_manifest(self.manifest, self.schema, self.thresholds)
        self.assertEqual(errors, [])
        for surface_id, surface in data["surfaces"].items():
            states = {state["id"] for state in surface["states"]}
            self.assertEqual(states, data["covered"][surface_id])

    def test_sample_captured_artifact_validates(self):
        self.assertEqual(
            t219.check_artifact(self.sample, self.manifest, self.schema, self.thresholds),
            [],
        )

    def test_sample_open_artifact_validates(self):
        self.assertEqual(
            t219.check_artifact(self.sample_open, self.manifest, self.schema, self.thresholds),
            [],
        )

    def test_missing_axes_is_rejected(self):
        artifact = copy.deepcopy(self.sample)
        del artifact["axes"]
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(errors)

    def test_unknown_surface_is_rejected(self):
        artifact = copy.deepcopy(self.sample)
        artifact["surface"] = "S-NOT-REAL"
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(errors)

    def test_bad_hash_is_rejected(self):
        artifact = copy.deepcopy(self.sample)
        artifact["verification"]["sha256"] = "0" * 64
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(any("hash mismatch" in error for error in errors))

    def test_open_row_without_reason_is_rejected(self):
        artifact = copy.deepcopy(self.sample_open)
        del artifact["verification"]["reason"]
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(errors)

    def test_status_verification_mismatch_is_rejected(self):
        artifact = copy.deepcopy(self.sample)
        artifact["status"] = "open"
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(errors)

    def test_non_applicable_dimension_is_rejected(self):
        artifact = copy.deepcopy(self.sample_open)
        artifact["id"] = "A-MENU-01"
        artifact["surface"] = "A-MENU"
        artifact["state"] = "open"
        artifact["acceptance"]["dimensions"] = ["material"]
        errors = t219.check_artifact(artifact, self.manifest, self.schema, self.thresholds)
        self.assertTrue(any("non-applicable" in error for error in errors))

    def test_unknown_threshold_profile_is_rejected(self):
        thresholds = copy.deepcopy(self.thresholds)
        thresholds["surfaces"]["S-MB"]["profile"] = "does-not-exist"
        errors = t219.check_thresholds(thresholds)
        self.assertTrue(any("unknown profile" in error for error in errors))

    def test_profile_missing_dimension_is_rejected(self):
        thresholds = copy.deepcopy(self.thresholds)
        del thresholds["profiles"]["glass-transient"]["performance"]
        errors = t219.check_thresholds(thresholds)
        self.assertTrue(any("performance" in error for error in errors))

    def test_axis_vocabulary_is_enforced(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["axis_sets"]["light-1x"]["appearance"] = "sepia"
        errors, _ = t219.check_manifest(manifest, self.schema, self.thresholds)
        self.assertTrue(any("not in the vocabulary" in error for error in errors))

    def test_cli_check_passes(self):
        stdout = io.StringIO()
        stderr = io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = t219.main(["--check"])
        self.assertEqual(code, 0, stderr.getvalue())
        self.assertIn("Covered (captured + verified)", stdout.getvalue())

    def test_cli_sample_artifact_passes(self):
        stdout = io.StringIO()
        stderr = io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            code = t219.main(["--validate-artifact", str(FIXTURES / "sample-artifact.json")])
        self.assertEqual(code, 0, stderr.getvalue())
        self.assertIn("PASS: artifact", stdout.getvalue())


if __name__ == "__main__":
    unittest.main(verbosity=2)
