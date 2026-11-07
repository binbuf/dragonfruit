#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Regression tests for the T220 naming and asset gates.

Run with ``make check-naming`` or directly:

    python3 tests/naming/test_naming_and_assets.py

The tests prove the gates reject the failures the policy names: an Apple term
in a QML string, a desktop entry, or packaging metadata; a shipped asset with
no provenance; and an unlisted asset file. They also validate the committed
map, manifest, and the current tree.
"""

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
NAMING_SCRIPT = REPO / "scripts" / "check-user-facing-names.py"
ASSET_SCRIPT = REPO / "scripts" / "check-asset-manifest.py"
OPTIONAL_SERVICES = REPO / "packaging" / "optional-services.json"
NAMING_DOC = REPO / "docs" / "design" / "interface-naming-and-licensing.md"
NOTICE = REPO / "NOTICE"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


naming = load_module("check_user_facing_names", NAMING_SCRIPT)
assets = load_module("check_asset_manifest", ASSET_SCRIPT)


class NamingGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.map = naming.load_map(REPO / "docs" / "design" / "naming-gate.json")

    def test_map_rows_are_valid(self):
        self.assertTrue(self.map["forbid"])
        for row in self.map["forbid"]:
            self.assertTrue(row["term"])
            self.assertTrue(row["replacement"])
            self.assertTrue(row["pattern"])

    def test_repository_has_no_forbidden_user_facing_terms(self):
        self.assertEqual(naming.scan(REPO, self.map), [])

    def test_forbidden_qml_string_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "shell").mkdir()
            (root / "shell" / "Sample.qml").write_text(
                'Item { text: qsTr("Mission Control") }\n', encoding="utf-8"
            )
            errors = naming.scan(root, self.map)
            self.assertTrue(any("Mission Control" in error for error in errors))

    def test_forbidden_cpp_literal_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "shell" / "src").mkdir(parents=True)
            (root / "shell" / "src" / "Sample.cpp").write_text(
                'entry(QStringLiteral("App Store"));\n', encoding="utf-8"
            )
            errors = naming.scan(root, self.map)
            self.assertTrue(any("App Store" in error for error in errors))

    def test_internal_key_literal_is_not_user_facing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "shell" / "src").mkdir(parents=True)
            (root / "shell" / "src" / "Sample.cpp").write_text(
                'settingsKey(QStringLiteral("wallpaper.showOnAllSpaces"));\n',
                encoding="utf-8",
            )
            self.assertEqual(naming.scan(root, self.map), [])

    def test_forbidden_desktop_value_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "apps" / "settings").mkdir(parents=True)
            (root / "apps" / "settings" / "Sample.desktop").write_text(
                "[Desktop Entry]\nName=App Store\n", encoding="utf-8"
            )
            errors = naming.scan(root, self.map)
            self.assertTrue(any("App Store" in error for error in errors))

    def test_forbidden_packaging_value_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "packaging" / "debian").mkdir(parents=True)
            (root / "packaging" / "debian" / "control").write_text(
                "Recommends: iCloud\n", encoding="utf-8"
            )
            errors = naming.scan(root, self.map)
            self.assertTrue(any("iCloud" in error for error in errors))

    def test_allow_marker_exempts_a_negative_test(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "shell").mkdir()
            (root / "shell" / "Sample.qml").write_text(
                'Item { text: qsTr("Mission Control") } // df-allow-name\n',
                encoding="utf-8",
            )
            self.assertEqual(naming.scan(root, self.map), [])

    def test_generic_names_pass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "shell").mkdir()
            (root / "shell" / "Sample.qml").write_text(
                'Item { text: qsTr("Overview") }\n'
                'Item { text: qsTr("Show on all Workspaces") }\n',
                encoding="utf-8",
            )
            self.assertEqual(naming.scan(root, self.map), [])


class AssetManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest_path = REPO / "docs" / "design" / "asset-manifest.json"
        cls.manifest = assets.load_manifest(cls.manifest_path)

    def test_categories_are_complete(self):
        self.assertEqual(set(self.manifest["categories"]), assets.CATEGORIES)

    def test_repository_manifest_is_valid(self):
        errors, summary = assets.check(REPO, self.manifest_path)
        self.assertEqual(errors, [])
        self.assertGreater(summary["shipped_files"], 0)
        self.assertEqual(set(summary["categories"]), assets.CATEGORIES)

    def test_unlisted_asset_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "assets").mkdir()
            (root / "assets" / "listed.svg").write_text("<svg/>", encoding="utf-8")
            (root / "assets" / "extra.png").write_bytes(b"png")
            (root / "LICENSES").mkdir()
            (root / "LICENSES" / "MIT.txt").write_text("MIT", encoding="utf-8")
            manifest = {
                "schema": 1,
                "categories": {category: "" for category in assets.CATEGORIES},
                "assets": [
                    {
                        "id": "icon.test",
                        "category": "icon",
                        "name": "Test icons",
                        "status": "shipped",
                        "path": "assets",
                        "include": ["*.svg"],
                        "source": "test",
                        "license": "MIT",
                        "license_file": "LICENSES/MIT.txt",
                        "notice": "NOTICE",
                    }
                ],
            }
            path = root / "manifest.json"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            errors, _ = assets.check(root, path)
            self.assertTrue(any("extra.png" in error for error in errors))

    def test_missing_license_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "assets").mkdir()
            (root / "assets" / "listed.svg").write_text("<svg/>", encoding="utf-8")
            manifest = {
                "schema": 1,
                "categories": {category: "" for category in assets.CATEGORIES},
                "assets": [
                    {
                        "id": "icon.test",
                        "category": "icon",
                        "name": "Test icons",
                        "status": "shipped",
                        "path": "assets",
                        "include": ["*.svg"],
                        "source": "test",
                        "license": "MIT",
                        "license_file": "LICENSES/missing.txt",
                        "notice": "NOTICE",
                    }
                ],
            }
            path = root / "manifest.json"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            errors, _ = assets.check(root, path)
            self.assertTrue(any("license_file" in error for error in errors))

    def test_none_shipped_category_needs_a_policy_and_owner(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = {
                "schema": 1,
                "categories": {category: "" for category in assets.CATEGORIES},
                "assets": [
                    {
                        "id": "sound.test",
                        "category": "sound",
                        "name": "Test sounds",
                        "status": "none-shipped",
                        "path": None,
                        "include": [],
                        "source": "none",
                        "license": "none",
                        "license_file": None,
                        "notice": "NOTICE",
                    }
                ],
            }
            path = root / "manifest.json"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            errors, _ = assets.check(root, path)
            self.assertTrue(any("owner" in error for error in errors))
            self.assertTrue(any("policy" in error for error in errors))


class OptionalServiceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads(OPTIONAL_SERVICES.read_text(encoding="utf-8"))
        cls.doc = NAMING_DOC.read_text(encoding="utf-8")
        cls.notice = NOTICE.read_text(encoding="utf-8")

    def test_schema_and_services(self):
        self.assertEqual(self.data["schema"], 1)
        self.assertTrue(self.data["services"])
        for service in self.data["services"]:
            for field in ("id", "name", "license", "integration", "decision", "absence"):
                self.assertTrue(service.get(field), f"{service.get('id')}: missing {field}")

    def test_markers_are_weak_dependencies_only(self):
        for service in self.data["services"]:
            for distro in ("fedora", "debian"):
                marker = service[distro]["marker"]
                self.assertIn(marker, {"Recommends", "Suggests"})
                self.assertNotIn(marker, {"Requires", "Depends"})

    def test_services_are_linked_from_notice_and_the_third_party_table(self):
        for service in self.data["services"]:
            name = service["name"]
            self.assertIn(name, self.notice, f"{name} missing from NOTICE")
            self.assertIn(name, self.doc, f"{name} missing from the third-party table")

    def test_display_manager_decision_is_not_a_package_dependency(self):
        display = self.data["display_managers"]
        self.assertEqual(
            set(display["supported"]), {"GDM", "SDDM", "LightDM", "greetd"}
        )
        self.assertIn("does not depend", display["packaging_marker"])


if __name__ == "__main__":
    unittest.main()
