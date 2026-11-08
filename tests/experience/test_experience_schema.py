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

    def test_structural_surfaces_and_states_exist(self):
        surfaces = {surface["id"]: surface for surface in self.manifest["surfaces"]}
        expected = {
            "N-WIN": {
                "chrome-borderless", "chrome-titlebar", "chrome-unified-compact",
                "chrome-expanded", "chrome-unified", "chrome-unified-labelled",
                "chrome-preference", "traffic-lights-titlebar",
                "traffic-lights-unified", "accessory-bars",
                "fullscreen-chrome-hidden", "fullscreen-chrome-revealed",
                "long-title", "rtl-titlebar",
            },
            "SET-GENERAL": {
                "about-panel", "about-page", "about-dark", "about-localized",
            },
            "Q-AD": {
                "spotlight-grid", "spotlight-list", "suggestions",
                "search-focused", "category-chips", "no-results",
                "launch-motion", "spotlight-keyboard", "iphone-apps",
            },
            "S-NC": {
                "panel-layout", "banner-stack", "widget-stack",
                "widget-gallery", "material-ramp", "empty", "reply-actions",
                "focus-active", "widget-variants", "motion-open-close",
                "motion-banner-stack",
            },
            "S-CC": {
                "grid", "module-sizes", "expanded-wifi", "expanded-bluetooth",
                "expanded-focus", "edit-controls", "addable-modules", "pages",
                "resize", "menu-bar-pinning", "motion-open-close",
                "motion-expand", "motion-edit",
            },
            "N-FILES": {
                "chrome", "toolbar-hidden", "view-icon", "view-list",
                "view-column", "view-gallery", "get-info", "search",
                "gallery-preview", "tags", "customization", "dark-matrix",
                "motion",
            },
        }
        for surface_id, states in expected.items():
            self.assertIn(surface_id, surfaces)
            actual = {state["id"] for state in surfaces[surface_id]["states"]}
            self.assertTrue(
                states <= actual,
                f"{surface_id}: missing structural states {sorted(states - actual)}",
            )

    def test_structural_open_ids_are_tracked(self):
        open_ids = (
            "OPEN-CHROME-02", "OPEN-CHROME-03",
            "OPEN-ABOUT-01", "OPEN-ABOUT-02",
            "OPEN-DRAWER-01", "OPEN-DRAWER-02",
            "OPEN-NC-01", "OPEN-NC-02",
            "OPEN-CC-01", "OPEN-CC-02", "OPEN-CC-03",
            "OPEN-FINDER-01", "OPEN-FINDER-02", "OPEN-FINDER-03",
        )
        for open_id in open_ids:
            rows = [
                entry for entry in self.manifest["entries"]
                if open_id in json.dumps(entry, sort_keys=True)
            ]
            self.assertTrue(rows, f"{open_id} is not tracked by any manifest row")
            for row in rows:
                self.assertEqual(row["status"], "open", open_id)
                self.assertTrue(row["verification"].get("reason"), open_id)
                self.assertTrue(row["verification"].get("rerun"), open_id)
                self.assertTrue(row["verification"].get("owner"), open_id)

    def test_structural_thresholds_are_tighten_only(self):
        def geometry(surface_id):
            return t219.resolve_threshold(self.thresholds, surface_id)["geometry"]

        window = geometry("N-WIN")
        self.assertEqual(
            window["chrome_height_pt"],
            [0.0, 32.0, 40.0, 48.0, 52.0, 66.0, 88.0],
        )
        self.assertEqual(window["chrome_height_tolerance_pt"], 1.0)
        self.assertEqual(window["traffic_light_size_pt"], [16.0, 16.0])
        self.assertEqual(window["traffic_light_size_tolerance_pt"], 1.0)
        self.assertEqual(
            window["traffic_light_top_inset_pt"],
            {"titlebar": 8.0, "unified": 18.0},
        )
        self.assertEqual(window["traffic_light_inset_tolerance_pt"], 1.0)
        self.assertEqual(window["traffic_light_spacing_pt"], 23.0)
        self.assertEqual(window["traffic_light_spacing_tolerance_pt"], 1.0)
        self.assertEqual(
            window["accessory_bar_heights_pt"],
            {"tab": 28.0, "path": 30.0, "status": 28.0, "format": 20.0},
        )

        about = geometry("SET-GENERAL")
        self.assertEqual(about["about_panel_pt"], [280.0, 503.0])
        self.assertEqual(about["about_panel_chrome_height_pt"], 32.0)
        self.assertEqual(about["about_page_pt"], [740.0, 820.0])
        self.assertEqual(about["about_page_chrome_height_pt"], 52.0)
        self.assertEqual(about["about_tolerance_pt"], 1.0)

        drawer = geometry("Q-AD")
        self.assertEqual(drawer["drawer_columns"], 7)
        self.assertEqual(drawer["drawer_cell_pt"], [115.0, 92.0])
        self.assertEqual(drawer["drawer_icon_pt"], [77.0, 77.0])
        self.assertEqual(drawer["drawer_column_pitch_pt"], 115.0)
        self.assertEqual(drawer["drawer_row_pitch_pt"], 102.0)
        self.assertEqual(drawer["drawer_tolerance_pt"], 1.0)

        notification = t219.resolve_threshold(self.thresholds, "S-NC")
        self.assertEqual(notification["geometry"]["panel_width_pt"], 344.0)
        self.assertEqual(notification["geometry"]["banner_stack_pt"], [344.0, 75.0])
        self.assertEqual(
            notification["material"]["bands"],
            {
                "banner": {"alpha": 0.549, "tint": 0.976},
                "stack": {"alpha": 0.551, "tint": 0.706},
                "lower_panel": {"alpha": 0.407, "tint": 0.852},
            },
        )

        control_center = geometry("S-CC")
        self.assertEqual(control_center["grid_columns"], 4)
        self.assertEqual(control_center["cell_pt"], [64.0, 64.0])
        self.assertEqual(control_center["gap_pt"], 12.0)
        self.assertEqual(control_center["module_field_width_pt"], 292.0)
        self.assertEqual(control_center["edit_controls_categories"], 22)

        files = geometry("N-FILES")
        self.assertEqual(files["toolbar_height_pt"], 52.0)
        self.assertEqual(files["sidebar_width_pt"], 198.0)
        self.assertEqual(files["tab_bar_height_pt"], 28.0)
        self.assertEqual(files["path_bar_height_pt"], 30.0)
        self.assertEqual(files["status_bar_height_pt"], 28.0)
        self.assertEqual(files["get_info_window_pt"], [328.0, 1034.0])
        self.assertEqual(files["files_tolerance_pt"], 1.0)

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
