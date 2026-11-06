#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Deterministic self-test for the T194 glass comparison workflow.

Run with ``make glass-selftest`` (or ``python3 tests/glass/run_selftest.py``).

The test proves that the comparator *detects* the regressions the acceptance
criteria name, rather than only proving that identical images compare equal:

* colour — a deliberate RGB offset in the material band fails;
* crop/geometry — a shrunken card and a size-mismatched capture fail;
* MTF — a blurred checker loses contrast and fails;
* motion — a stationary candidate where the reference moves fails;
* registration — a one-pixel alignment shift is corrected, a shift beyond the
  bound is rejected instead of being hidden;
* timing — a doubled counter fails the timing comparison;
* coverage — an empty/out-of-image band is reported missing, not passed.

It also regenerates the fixtures into a temporary directory and verifies that
the committed pixels/specs and derived targets are current.
"""
from __future__ import annotations

import json
import shutil
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
FIXTURES = HERE / "fixtures"
SPECS = HERE / "specs"

sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(HERE))

import glass_compare  # noqa: E402
import generate_fixtures  # noqa: E402


class Checks:
    def __init__(self) -> None:
        self.passed = 0
        self.failed = 0

    def check(self, condition: bool, message: str) -> None:
        if condition:
            self.passed += 1
            print(f"PASS  {message}")
        else:
            self.failed += 1
            print(f"FAIL  {message}", file=sys.stderr)

    def report(self) -> int:
        print(f"\n{self.passed} passed, {self.failed} failed")
        return 0 if self.failed == 0 else 1


def _spec(name: str) -> dict:
    return json.loads((SPECS / f"{name}.json").read_text())


def _save(image: Image.Image, directory: Path, name: str) -> Path:
    path = directory / name
    image.save(path)
    return path


def _shift(image: Image.Image, dx: int, dy: int) -> Image.Image:
    out = Image.new("RGB", image.size, (24, 24, 28))
    out.paste(image, (dx, dy))
    return out


def _targets_match(report: dict, expected: dict, checks: Checks, name: str) -> None:
    actual = report["metrics"]
    checks.check(report["verdict"] == expected["verdict"], f"{name}: verdict is {expected['verdict']}")
    for band, values in expected["colour"].items():
        got = actual["colour"]["bands"][band]
        for key in ("coverage", "mean_abs_srgb", "p95_abs_srgb", "ssim", "delta_e76_mean"):
            checks.check(
                abs(float(got[key]) - float(values[key])) <= 1e-9,
                f"{name}: target colour {band}.{key}",
            )
    checks.check(
        abs(actual["geometry"]["bbox_delta_px"] - expected["geometry"]["bbox_delta_px"]) <= 1e-9,
        f"{name}: target geometry bbox_delta",
    )
    checks.check(
        abs(actual["geometry"]["iou"] - expected["geometry"]["iou"]) <= 1e-9,
        f"{name}: target geometry iou",
    )
    if expected["mtf"].get("status") == "pass":
        for period, row in expected["mtf"]["periods"].items():
            got = actual["mtf"]["periods"][period]
            checks.check(
                abs(got["reference_amplitude"] - row["reference_amplitude"]) <= 1e-9,
                f"{name}: target mtf amplitude at {period}px",
            )


def main() -> int:
    checks = Checks()
    provenance = json.loads((HERE / "provenance.json").read_text())
    targets = json.loads((HERE / "targets.json").read_text())

    # 1. Committed fixtures/specs are current and match their recorded hashes.
    checks.check(generate_fixtures.check() == 0, "fixtures/specs regenerate deterministically")
    import hashlib

    for name, expected in provenance["fixtures_sha256"].items():
        path = FIXTURES / name
        got = hashlib.sha256(path.read_bytes()).hexdigest()
        checks.check(got == expected, f"fixture hash {name}")
    for name, expected in provenance["specs_sha256"].items():
        path = SPECS / name
        got = hashlib.sha256(path.read_bytes()).hexdigest()
        checks.check(got == expected, f"spec hash {name}")
    checks.check(
        hashlib.sha256((HERE / "targets.json").read_bytes()).hexdigest()
        == provenance["targets_sha256"],
        "targets hash matches provenance",
    )
    checks.check(
        hashlib.sha256((HERE / "generate_fixtures.py").read_bytes()).hexdigest()
        == provenance["generator_sha256"],
        "generator hash matches provenance",
    )

    # 2. The committed derived targets still describe the committed fixtures.
    for name in ("neutral", "checker", "line_grid", "saturated"):
        report = glass_compare.compare_case(
            FIXTURES / f"{name}.png", FIXTURES / f"{name}.png", _spec(name)
        )
        checks.check(report["verdict"] == "pass", f"{name}: self-comparison passes")
        checks.check(
            all(entry["coverage"] > 0 for entry in report["metrics"]["colour"]["bands"].values()),
            f"{name}: every colour band has coverage",
        )
        checks.check(
            report["metrics"]["geometry"]["coverage"] > 0,
            f"{name}: geometry band has coverage",
        )
        _targets_match(report, targets["fixtures"][name], checks, name)

    motion_spec = _spec("motion")
    motion = glass_compare.compare_motion(
        FIXTURES / "motion_before.png",
        FIXTURES / "motion_after.png",
        FIXTURES / "motion_before.png",
        FIXTURES / "motion_after.png",
        motion_spec,
    )
    checks.check(motion["verdict"] == "pass", "motion: reference sequence passes")
    checks.check(
        abs(motion["reference_distance"] - targets["motion"]["reference_distance"]) <= 1e-9,
        "motion: target reference displacement",
    )

    with tempfile.TemporaryDirectory(prefix="glass-selftest-") as tmp:
        tmp_path = Path(tmp)
        neutral = Image.open(FIXTURES / "neutral.png").convert("RGB")
        checker = Image.open(FIXTURES / "checker.png").convert("RGB")

        # 3. Colour regression: a deliberate RGB offset in the interior band.
        colour = neutral.copy()
        draw = ImageDraw.Draw(colour)
        draw.rectangle((28, 24, 132, 92), fill=(116, 116, 124))
        colour_path = _save(colour, tmp_path, "colour-regression.png")
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", colour_path, _spec("neutral")
        )
        checks.check(report["verdict"] == "fail", "colour: RGB offset is detected")
        checks.check(
            report["metrics"]["colour"]["status"] == "fail",
            "colour: failure is attributed to the colour metric",
        )

        # 4. Geometry regression: shrink the card's right edge by 4 px.
        geometry = neutral.copy()
        draw = ImageDraw.Draw(geometry)
        draw.rectangle((136, 16, 140, 100), fill=(24, 24, 28))
        geometry_path = _save(geometry, tmp_path, "geometry-regression.png")
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", geometry_path, _spec("neutral")
        )
        checks.check(report["verdict"] == "fail", "geometry: a 4 px crop is detected")
        checks.check(
            report["metrics"]["geometry"]["status"] == "fail",
            "geometry: failure is attributed to the geometry metric",
        )

        # 4b. A differently sized capture is a geometry failure, not a resize.
        cropped = neutral.crop((0, 0, 150, 120))
        cropped_path = _save(cropped, tmp_path, "size-mismatch.png")
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", cropped_path, _spec("neutral")
        )
        checks.check(report["verdict"] == "fail", "geometry: a size mismatch fails")
        checks.check(
            "crop" in report.get("error", "").lower() or "size" in report.get("error", "").lower(),
            "geometry: size mismatch explains the crop",
        )

        # 5. MTF regression: blur the checker and require the contrast drop.
        blurred = checker.filter(ImageFilter.GaussianBlur(radius=1.2))
        blurred_path = _save(blurred, tmp_path, "mtf-regression.png")
        report = glass_compare.compare_case(
            FIXTURES / "checker.png", blurred_path, _spec("checker")
        )
        checks.check(report["verdict"] == "fail", "MTF: blur is detected")
        checks.check(
            report["metrics"]["mtf"]["status"] == "fail",
            "MTF: failure is attributed to the MTF metric",
        )

        # 6. Motion regression: the candidate frames do not move.
        still_before = FIXTURES / "motion_before.png"
        still_after = FIXTURES / "motion_before.png"
        report = glass_compare.compare_motion(
            FIXTURES / "motion_before.png",
            FIXTURES / "motion_after.png",
            still_before,
            still_after,
            motion_spec,
        )
        checks.check(report["verdict"] == "fail", "motion: a stationary candidate is detected")

        # 7. Registration: a 1 px shift is corrected; a 6 px shift is rejected.
        shifted = _shift(neutral, 1, 0)
        shifted_path = _save(shifted, tmp_path, "shift-1px.png")
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", shifted_path, _spec("neutral"),
            max_shift_logical=2.0,
        )
        checks.check(report["verdict"] == "pass", "registration: 1 px alignment is corrected")
        checks.check(
            report["registration"]["shift_px"] == [1, 0],
            "registration: reports the corrected shift",
        )
        far = _shift(neutral, 6, 0)
        far_path = _save(far, tmp_path, "shift-6px.png")
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", far_path, _spec("neutral"),
            max_shift_logical=2.0,
        )
        checks.check(report["verdict"] == "fail", "registration: a 6 px shift is rejected")

        # 8. Missing coverage is reported, never passed.
        missing = _spec("neutral")
        missing["colour_regions"] = [
            {"name": "outside", "rect": [200, 200, 220, 220]}
        ]
        report = glass_compare.compare_case(
            FIXTURES / "neutral.png", FIXTURES / "neutral.png", missing
        )
        checks.check(
            report["metrics"]["colour"]["status"] in ("missing", "fail"),
            "coverage: an out-of-image band is not a pass",
        )

    # 9. Timing: a doubled counter fails the comparison.
    reference = {"frames_rendered": 100, "material_allocations": 2}
    candidate = {"frames_rendered": 200, "material_allocations": 2}
    timing = glass_compare.compare_timing(reference, candidate)
    checks.check(timing["status"] == "fail", "timing: a doubled counter is detected")
    same = glass_compare.compare_timing(reference, reference)
    checks.check(same["status"] == "pass", "timing: identical counters pass")

    # 10. The committed reference manifest is structurally valid without the
    # optional checkout; the reference verify itself is a separate command.
    import subprocess

    result = subprocess.run(
        [sys.executable, str(REPO / "scripts" / "t194-reference-manifest.py"),
         "--check-manifest"],
        capture_output=True,
        text=True,
    )
    checks.check(result.returncode == 0, "reference manifest structure check")
    if result.returncode != 0:
        print(result.stderr, file=sys.stderr)

    return checks.report()


if __name__ == "__main__":
    sys.exit(main())
