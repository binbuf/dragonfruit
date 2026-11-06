#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Generate the deterministic T194 comparison fixtures and their targets.

The fixtures are deliberately small (160x120), synthetic and checked in.  They
exercise the four required material families:

* ``neutral``    — a flat glass card over a neutral backdrop;
* ``checker``    — a checkerboard inside the card for MTF;
* ``line_grid``  — a vertical line grid inside the card for MTF;
* ``saturated``  — saturated red/green/blue/yellow bands for colour error.

A separate ``motion_before`` / ``motion_after`` pair moves a red square by
32 px so the motion metric has a real reference displacement.

Run ``python3 tests/glass/generate_fixtures.py`` to regenerate the fixtures,
specs, ``targets.json`` and ``provenance.json``.  ``--check`` regenerates into a
temporary directory and fails if the committed pixel data or derived targets
drift.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageCms

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
FIXTURES = HERE / "fixtures"
SPECS = HERE / "specs"

sys.path.insert(0, str(REPO / "scripts"))
import glass_compare  # noqa: E402

WIDTH, HEIGHT = 160, 120
BACKGROUND = (24, 24, 28)
PANEL = (20, 16, 140, 100)
INTERIOR = (28, 24, 132, 92)
RIM = (20, 16, 140, 23)
SHADOW = (18, 102, 142, 114)
SRGB_ICC = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical_sha(value) -> str:
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def _new_canvas() -> Image.Image:
    return Image.new("RGB", (WIDTH, HEIGHT), BACKGROUND)


def _card(draw: ImageDraw.ImageDraw, fill, outline) -> None:
    # A soft-ish shadow is part of the card's visual extent; the geometry band
    # includes it so a missing shadow is a geometry regression.
    draw.rounded_rectangle((22, 20, 138, 108), radius=14, fill=(12, 12, 15))
    draw.rounded_rectangle(PANEL, radius=12, fill=fill, outline=outline, width=2)


def neutral() -> Image.Image:
    image = _new_canvas()
    draw = ImageDraw.Draw(image)
    _card(draw, (96, 96, 104), (158, 158, 166))
    return image


def checker() -> Image.Image:
    image = _new_canvas()
    draw = ImageDraw.Draw(image)
    _card(draw, (96, 96, 104), (158, 158, 166))
    square = 8
    for y in range(INTERIOR[1], INTERIOR[3], square):
        for x in range(INTERIOR[0], INTERIOR[2], square):
            index = (x - INTERIOR[0]) // square + (y - INTERIOR[1]) // square
            colour = (58, 58, 64) if index % 2 == 0 else (178, 178, 186)
            draw.rectangle(
                (x, y, min(x + square - 1, INTERIOR[2] - 1),
                 min(y + square - 1, INTERIOR[3] - 1)),
                fill=colour,
            )
    return image


def line_grid() -> Image.Image:
    image = _new_canvas()
    draw = ImageDraw.Draw(image)
    _card(draw, (70, 70, 78), (158, 158, 166))
    for x in range(INTERIOR[0], INTERIOR[2], 8):
        draw.rectangle((x, INTERIOR[1], x + 3, INTERIOR[3] - 1), fill=(210, 210, 218))
    return image


def saturated() -> Image.Image:
    image = _new_canvas()
    draw = ImageDraw.Draw(image)
    _card(draw, (96, 96, 104), (158, 158, 166))
    bands = [
        (INTERIOR[0], (220, 30, 40)),
        (INTERIOR[0] + 26, (30, 200, 60)),
        (INTERIOR[0] + 52, (30, 70, 220)),
        (INTERIOR[0] + 78, (230, 210, 30)),
    ]
    for x, colour in bands:
        draw.rectangle((x, INTERIOR[1], min(x + 25, INTERIOR[2] - 1), INTERIOR[3] - 1),
                       fill=colour)
    return image


def motion_frames() -> tuple[Image.Image, Image.Image]:
    before = _new_canvas()
    after = _new_canvas()
    for image, x in ((before, 30), (after, 62)):
        draw = ImageDraw.Draw(image)
        draw.rectangle((x, 48, x + 23, 71), fill=(220, 30, 40))
    return before, after


def _common_spec(name: str) -> dict:
    return {
        "name": name,
        "scale": 1,
        "regions": {
            "interior": list(INTERIOR),
            "rim": list(RIM),
            "shadow": list(SHADOW),
            "geometry": [0, 0, WIDTH, HEIGHT],
        },
        "registration_region": list(PANEL),
        "colour_regions": [
            {"name": "interior", "rect": list(INTERIOR)},
            {"name": "rim", "rect": list(RIM)},
            {"name": "shadow", "rect": list(SHADOW)},
        ],
        "geometry": {
            "rect": [0, 0, WIDTH, HEIGHT],
            "background": "#18181c",
            "tolerance": 0.05,
        },
    }


def specs() -> dict[str, dict]:
    result = {}
    for name in ("neutral", "saturated"):
        result[name] = _common_spec(name)
    for name, periods in (("checker", [16, 32]), ("line_grid", [8, 16])):
        spec = _common_spec(name)
        spec["mtf"] = {
            "rect": list(INTERIOR),
            "axis": "x",
            "periods_px": periods,
        }
        result[name] = spec
    result["motion"] = {
        "name": "motion",
        "scale": 1,
        "region": [0, 0, WIDTH, HEIGHT],
        "background": "#18181c",
        "tolerance": 0.05,
        "min_displacement_px": 20.0,
    }
    return result


def _targets(directory: Path) -> dict:
    fixtures = directory / "fixtures"
    specs_dir = directory / "specs"
    out = {"fixtures": {}, "motion": {}}
    for name in ("neutral", "checker", "line_grid", "saturated"):
        spec = json.loads((specs_dir / f"{name}.json").read_text())
        report = glass_compare.compare_case(
            fixtures / f"{name}.png", fixtures / f"{name}.png", spec
        )
        metrics = report["metrics"]
        out["fixtures"][name] = {
            "verdict": report["verdict"],
            "colour": {
                band: {
                    "coverage": entry["coverage"],
                    "mean_abs_srgb": entry["mean_abs_srgb"],
                    "p95_abs_srgb": entry["p95_abs_srgb"],
                    "ssim": entry["ssim"],
                    "delta_e76_mean": entry["delta_e76_mean"],
                }
                for band, entry in metrics["colour"]["bands"].items()
            },
            "geometry": {
                "coverage": metrics["geometry"]["coverage"],
                "bbox": metrics["geometry"]["reference_bbox"],
                "bbox_delta_px": metrics["geometry"]["bbox_delta_px"],
                "iou": metrics["geometry"]["iou"],
            },
            "mtf": metrics["mtf"],
        }
    motion_spec = json.loads((specs_dir / "motion.json").read_text())
    report = glass_compare.compare_motion(
        fixtures / "motion_before.png",
        fixtures / "motion_after.png",
        fixtures / "motion_before.png",
        fixtures / "motion_after.png",
        motion_spec,
    )
    out["motion"] = {
        "verdict": report["verdict"],
        "reference_displacement": report["reference_displacement"],
        "reference_distance": report["reference_distance"],
        "displacement_ratio": report["displacement_ratio"],
        "direction_error_deg": report["direction_error_deg"],
    }
    return out


def write_all(directory: Path) -> None:
    fixtures = directory / "fixtures"
    specs_dir = directory / "specs"
    fixtures.mkdir(parents=True, exist_ok=True)
    specs_dir.mkdir(parents=True, exist_ok=True)

    images = {
        "neutral": neutral(),
        "checker": checker(),
        "line_grid": line_grid(),
        "saturated": saturated(),
    }
    before, after = motion_frames()
    images["motion_before"] = before
    images["motion_after"] = after
    for name, image in images.items():
        image.save(fixtures / f"{name}.png", icc_profile=SRGB_ICC, optimize=False)

    spec_data = specs()
    for name, spec in spec_data.items():
        (specs_dir / f"{name}.json").write_text(
            json.dumps(spec, indent=2, sort_keys=True) + "\n"
        )


def _load_pixels(path: Path) -> bytes:
    return Image.open(path).convert("RGB").tobytes()


def _canonical(value) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def check() -> int:
    with tempfile.TemporaryDirectory(prefix="glass-fixtures-") as tmp:
        tmp_path = Path(tmp)
        write_all(tmp_path)
        problems = []
        for name in ("neutral", "checker", "line_grid", "saturated",
                     "motion_before", "motion_after"):
            committed = FIXTURES / f"{name}.png"
            generated = tmp_path / "fixtures" / f"{name}.png"
            if not committed.exists():
                problems.append(f"missing fixture {committed.relative_to(REPO)}")
                continue
            if _load_pixels(committed) != _load_pixels(generated):
                problems.append(f"pixel drift: {committed.relative_to(REPO)}")
        for name in specs():
            committed = SPECS / f"{name}.json"
            generated = tmp_path / "specs" / f"{name}.json"
            if not committed.exists():
                problems.append(f"missing spec {committed.relative_to(REPO)}")
                continue
            if _canonical(json.loads(committed.read_text())) != _canonical(
                json.loads(generated.read_text())
            ):
                problems.append(f"spec drift: {committed.relative_to(REPO)}")
        if problems:
            print("glass fixtures: DRIFT", file=sys.stderr)
            for problem in problems:
                print(f"  {problem}", file=sys.stderr)
            return 1
    print("glass fixtures: deterministic pixel and spec check PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--out", type=Path, default=HERE)
    args = parser.parse_args()
    if args.check:
        return check()
    write_all(args.out)
    # Targets/provenance are always written into the committed test tree so a
    # regenerate run keeps them next to the fixtures.
    targets = _targets(args.out)
    (args.out / "targets.json").write_text(
        json.dumps(targets, indent=2, sort_keys=True) + "\n"
    )
    fixture_hashes = {
        path.name: sha256(path)
        for path in sorted((args.out / "fixtures").glob("*.png"))
    }
    spec_hashes = {
        path.name: sha256(path)
        for path in sorted((args.out / "specs").glob("*.json"))
    }
    provenance = {
        "schema": 1,
        "generated_by": "tests/glass/generate_fixtures.py",
        "generator_sha256": sha256(Path(__file__)),
        "fixtures_sha256": fixture_hashes,
        "specs_sha256": spec_hashes,
        "targets_sha256": sha256(args.out / "targets.json"),
        "icc": {
            "embedded": "sRGB",
            "assumption": "synthetic fixtures embed an sRGB profile; the "
            "comparison converts embedded profiles to sRGB",
        },
        "synthetic": True,
        "display_scale": 1,
        "seed": 194,
        "command": "python3 tests/glass/generate_fixtures.py",
    }
    (args.out / "provenance.json").write_text(
        json.dumps(provenance, indent=2, sort_keys=True) + "\n"
    )
    print(f"wrote fixtures, specs, targets and provenance under {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
