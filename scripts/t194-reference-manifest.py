#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Pin and verify the T194 reference-evidence manifest.

The full macOS reference checkout is optional.  This script records the small
subset of source PNGs, layout/AX dumps, JSON sections, analysis scripts and
derived crops that the Dragonfruit comparison workflow uses, together with
their SHA-256 hashes and the sRGB/ICC assumptions.

Commands:

    python3 scripts/t194-reference-manifest.py --write
    python3 scripts/t194-reference-manifest.py --verify
    python3 scripts/t194-reference-manifest.py --check-manifest

The reference root is resolved in this order:

1. ``--reference-root PATH``
2. ``DRAGONFRUIT_REFERENCE_ROOT``
3. the sibling checkout ``../macos-liquid-glass`` next to this repository

A reference run (``--write`` or ``--verify``) fails clearly when the root is
missing, the checkout revision does not match the manifest, or any recorded
source/crop/script hash differs.  ``--check-manifest`` is the structural check
that runs without the checkout (it is part of ``make glass-selftest``).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Iterable

REPO = Path(__file__).resolve().parent.parent
MANIFEST = REPO / "docs" / "captures" / "glass-reference-manifest.json"
DEFAULT_ROOT = REPO.parent / "macos-liquid-glass"
ENV_ROOT = "DRAGONFRUIT_REFERENCE_ROOT"
REFERENCE_REVISION = "063a473cf06d0dba5cb83bd56c40b6fc88df2931"

sys.path.insert(0, str(REPO / "scripts"))
import glass_compare  # noqa: E402


SOURCES = [
    {
        "id": "background_neutral_mid",
        "path": "backgrounds/neutral_mid.png",
        "role": "neutral mid-grey backdrop used by the Dock/menu/Control Center fits",
    },
    {
        "id": "background_neutral_white",
        "path": "backgrounds/neutral_white.png",
        "role": "white backdrop used by menu/banner fits",
    },
    {
        "id": "background_checker_16",
        "path": "backgrounds/checker_16.png",
        "role": "16 px checker backdrop used by the Dock MTF fit",
    },
    {
        "id": "dock_rest_neutral_mid",
        "path": "captures/raw/neutral_mid__dock_rest.png",
        "role": "Dock plate material, rim, geometry and MTF source",
    },
    {
        "id": "dock_rest_checker_16",
        "path": "captures/raw/checker_16__dock_rest.png",
        "role": "Dock MTF source",
    },
    {
        "id": "menu_file_neutral_white",
        "path": "captures/raw/neutral_white__menu_file.png",
        "role": "light attached menu (File) material and geometry source",
    },
    {
        "id": "control_center_neutral_white",
        "path": "captures/raw/neutral_white__control_center.png",
        "role": "Control Center module/outer-panel material source",
    },
    {
        "id": "control_center_panel_neutral_white",
        "path": "captures/coverage/control_center_panel__neutral_white.png",
        "role": "later controlled Control Center outer-panel fit",
    },
    {
        "id": "notification_banner_neutral_white",
        "path": "captures/coverage/notification_banner__neutral_white.png",
        "role": "notification banner material source",
    },
    {
        "id": "notification_center_neutral_white",
        "path": "captures/coverage/notification_center__neutral_white.png",
        "role": "Notification Center stack/lower-panel evidence and gap source",
    },
    {
        "id": "context_desktop_neutral_white",
        "path": "captures/coverage/context_desktop__neutral_white.png",
        "role": "detached desktop context-menu material source",
    },
    {
        "id": "status_wifi_neutral_white",
        "path": "captures/coverage/menu_wifi__neutral_white.png",
        "role": "detached status-menu material source",
    },
    {
        "id": "dock_layout_checker_16",
        "path": "captures/raw/checker_16__dock_layout.json",
        "role": "Dock tile layout dump for geometry/magnification provenance",
    },
    {
        "id": "control_center_ax",
        "path": "captures/coverage/control_center_panel.ui.txt",
        "role": "Control Center accessibility/layout dump",
    },
    {
        "id": "context_desktop_ax",
        "path": "captures/coverage/context_desktop_finder.ui.txt",
        "role": "detached menu accessibility/layout dump",
    },
]

JSON_SECTIONS = [
    ("docs/measurements.json", "system"),
    ("docs/measurements.json", "capture"),
    ("docs/measurements.json", "dock.geometry"),
    ("docs/measurements.json", "dock.material"),
    ("docs/measurements.json", "dock.blur"),
    ("docs/measurements.json", "dock.rim"),
    ("docs/measurements.json", "dock.lens"),
    ("docs/measurements.json", "menu_bar"),
    ("docs/measurements.json", "popover.linear_srgb"),
    ("docs/measurements.json", "popover.mtf"),
    ("docs/measurements.json", "popover.geometry"),
    ("docs/measurements.json", "popover.shadow"),
    ("docs/measurements.json", "control_center.module"),
    ("docs/measurements.json", "control_center.outer_panel"),
    ("docs/measurements.json", "animation.magnification"),
    ("docs/measurements.json", "animation.temporal"),
    ("docs/measurements.json", "animation.menu"),
    ("docs/coverage_measurements.json", "context_menus"),
    ("docs/coverage_measurements.json", "status_menus"),
    ("docs/coverage_measurements.json", "control_center"),
    ("docs/coverage_measurements.json", "notification"),
    ("docs/coverage_measurements.json", "slider"),
    ("docs/coverage_measurements.json", "dark"),
    ("docs/coverage_measurements.json", "dark_extra"),
    ("docs/coverage_measurements.json", "accessibility"),
    ("docs/material-schema.json", "color"),
    ("docs/material-schema.json", "axes"),
    ("docs/material-schema.json", "surfaces"),
    ("docs/material-schema.json", "geometry"),
    ("docs/material-schema.json", "motion"),
    ("docs/material-schema.json", "acceptance"),
    ("docs/material-schema.json", "implementation_notes"),
    ("analysis/out/validation.json", ""),
]

SCRIPTS = [
    "analysis/measure.py",
    "analysis/coverage_measure.py",
    "analysis/glasslib.py",
    "analysis/compare.py",
    "analysis/render_reference.py",
    "tools/build_material_schema.py",
]

# Crop rectangles are in reference capture pixels (the workstation is 2x).
# They mirror the exact bands the analysis scripts score; the manifest records
# both the rect and the hash of the ICC-converted sRGB crop.
CROPS = [
    {
        "id": "dock_interior",
        "source": "dock_rest_neutral_mid",
        "rect_px": [737, 2178, 2862, 2306],
        "note": "measure.py dock_material interior band, clear of icons",
    },
    {
        "id": "dock_rim",
        "source": "dock_rest_neutral_mid",
        "rect_px": [837, 2166, 2762, 2190],
        "note": "measure.py dock_edge_profile top rim band",
    },
    {
        "id": "dock_mtf_band",
        "source": "dock_rest_checker_16",
        "rect_px": [737, 2180, 2862, 2192],
        "note": "measure.py dock_mtf row band at checker period 16",
    },
    {
        "id": "menu_file_interior",
        "source": "menu_file_neutral_white",
        "rect_px": [700, 300, 740, 1200],
        "note": "measure.py popover_material left padding band",
    },
    {
        "id": "menu_file_mtf_band",
        "source": "menu_file_neutral_white",
        "rect_px": [250, 1290, 700, 1320],
        "note": "measure.py popover_material checker MTF band",
    },
    {
        "id": "control_center_module",
        "source": "control_center_neutral_white",
        "rect_px": [3450, 130, 3530, 190],
        "note": "measure.py control_center_material module band",
    },
    {
        "id": "control_center_outer",
        "source": "control_center_neutral_white",
        "rect_px": [3545, 300, 3560, 500],
        "note": "measure.py control_center_material outer-panel band",
    },
    {
        "id": "control_center_outer_controlled",
        "source": "control_center_panel_neutral_white",
        "rect_px": [3000, 550, 3040, 650],
        "note": "coverage_measure.py later controlled outer-panel band",
    },
    {
        "id": "notification_banner",
        "source": "notification_banner_neutral_white",
        "rect_px": [2895, 150, 2915, 230],
        "note": "coverage_measure.py banner material band",
    },
    {
        "id": "notification_stack",
        "source": "notification_center_neutral_white",
        "rect_px": [3200, 240, 3250, 270],
        "note": "coverage_measure.py Notification Center stack band",
    },
    {
        "id": "notification_lower_panel",
        "source": "notification_center_neutral_white",
        "rect_px": [3200, 450, 3250, 550],
        "note": "coverage_measure.py Notification Center lower-panel band",
    },
    {
        "id": "context_desktop_left_padding",
        "source": "context_desktop_neutral_white",
        "rect_px": [1805, 1250, 1815, 1600],
        "note": "coverage_measure.py detached context-menu band",
    },
    {
        "id": "status_wifi_band",
        "source": "status_wifi_neutral_white",
        "rect_px": [2960, 150, 2980, 350],
        "note": "coverage_measure.py detached status-menu band",
    },
]


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _sha256(path: Path) -> str:
    return _sha256_bytes(path.read_bytes())


def _canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def _resolve_root(explicit: str | None) -> Path | None:
    if explicit:
        return Path(explicit).expanduser().resolve()
    env = os.environ.get(ENV_ROOT)
    if env:
        return Path(env).expanduser().resolve()
    if DEFAULT_ROOT.exists():
        return DEFAULT_ROOT.resolve()
    return None


def _git_revision(root: Path) -> str | None:
    try:
        out = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
        return out.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def _section(data, path: str):
    if not path:
        return data
    value = data
    for part in path.split("."):
        if isinstance(value, list):
            value = value[int(part)]
        else:
            value = value[part]
    return value


def _image_meta(path: Path) -> dict:
    from PIL import Image

    image = Image.open(path)
    return {
        "size": list(image.size),
        "mode": image.mode,
        "icc": glass_compare.icc_info(path),
    }


def _crop_hash(source: Path, rect: list[int]) -> dict:
    image = glass_compare.load_srgb(source)
    x0, y0, x1, y1 = rect
    if x0 < 0 or y0 < 0 or x1 > image.width or y1 > image.height or x1 <= x0 or y1 <= y0:
        raise ValueError(f"crop {rect} is outside {source} ({image.width}x{image.height})")
    crop = image.crop((x0, y0, x1, y1))
    return {
        "sha256_srgb": _sha256_bytes(crop.tobytes()),
        "size_px": [crop.width, crop.height],
        "mean_srgb": [round(v / 255.0, 6) for v in _mean_rgb(crop)],
    }


def _mean_rgb(image) -> list[float]:
    from PIL import ImageStat

    return list(ImageStat.Stat(image).mean[:3])


def _derived_targets(root: Path) -> dict:
    measurements = json.loads((root / "docs/measurements.json").read_text())
    coverage = json.loads((root / "docs/coverage_measurements.json").read_text())
    schema = json.loads((root / "docs/material-schema.json").read_text())

    dock = measurements["dock"]
    menu_bar = measurements["menu_bar"]
    popover = measurements["popover"]
    cc = measurements["control_center"]
    return {
        "dock": {
            "geometry": dock["geometry"],
            "alpha": dock["material"]["linear_srgb"]["mean_alpha"],
            "tint": dock["material"]["linear_srgb"]["mean_tint"],
            "blur": dock["blur"],
            "rim": {k: v for k, v in dock["rim"].items() if k != "profile"},
            "lens": {k: v for k, v in dock["lens"].items() if k != "measured_vertical_line"},
        },
        "menu_bar": {
            "fit": menu_bar["fit"],
            "bar_height_pt": menu_bar["bar_height_pt"],
            "veil_extent_pt": menu_bar["veil_extent_pt"],
            "blur": menu_bar["blur"],
        },
        "menus": {
            "light_attached_alpha": popover["linear_srgb"]["mean_alpha"],
            "light_attached_tint": popover["linear_srgb"]["mean_tint"],
            "mtf": popover["mtf"],
            "geometry": popover["geometry"],
            "shadow": popover["shadow"],
            "detached_context": coverage["context_menus"],
            "detached_status": coverage["status_menus"],
        },
        "control_center": {
            "module_alpha": cc["module"]["linear_srgb"]["mean_alpha"],
            "module_tint": cc["module"]["linear_srgb"]["mean_tint"],
            "outer_alpha": cc["outer_panel"]["linear_srgb"]["mean_alpha"],
            "outer_tint": cc["outer_panel"]["linear_srgb"]["mean_tint"],
            "controlled_outer": coverage["control_center"]["outer_panel"],
        },
        "notification": coverage["notification"],
        "accessibility": coverage["accessibility"],
        "material_schema": {
            "schema_version": schema.get("schema_version"),
            "color": schema.get("color"),
            "acceptance": schema.get("acceptance"),
        },
    }


def _gaps() -> list[dict]:
    return [
        {
            "id": "notification_center_unmapped",
            "status": "unmapped",
            "reason": "the shell consumes history and the compositor declares the input action, "
            "but no Notification Center surface is mapped; the reference stack/lower-panel "
            "fits remain evidence only",
            "owner": "T204",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md#what-to-carry-forward",
        },
        {
            "id": "settings_appearance_transparency",
            "status": "missing",
            "reason": "appearance.transparency is absent from the settingsd schema",
            "owner": "T196",
            "evidence": "services/settingsd/src/schema.rs",
        },
        {
            "id": "settings_reduce_transparency",
            "status": "missing",
            "reason": "accessibility.reduceTransparency is absent from the settingsd schema",
            "owner": "T196",
            "evidence": "services/settingsd/src/schema.rs",
        },
        {
            "id": "settings_increase_contrast",
            "status": "missing",
            "reason": "accessibility.increaseContrast is absent from the settingsd schema",
            "owner": "T196",
            "evidence": "services/settingsd/src/schema.rs",
        },
        {
            "id": "dark_attached_menu_rejected",
            "status": "rejected",
            "reason": "the published Dark File alpha/tint fit is a Light File crop; use the "
            "confirmed dark mid/black captures as appearance checks and recapture a controlled "
            "dark attached menu",
            "owner": "T202",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md#how-to-resolve-conflicting-evidence",
        },
        {
            "id": "dark_banner_lower_panel",
            "status": "missing",
            "reason": "dark banner and dark Notification Center lower panel are uncovered as "
            "independent fits",
            "owner": "T205",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md",
        },
        {
            "id": "interactive_auto_hide",
            "status": "missing",
            "reason": "the recorded Dock restart cut supplies no interactive auto-hide timing",
            "owner": "T199",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md",
        },
        {
            "id": "multi_output_fractional_hdr",
            "status": "missing",
            "reason": "multi-output, fractional DPI and HDR/P3 behavior are not measured",
            "owner": "T208",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md",
        },
    ]


def build_manifest(root: Path) -> dict:
    revision = _git_revision(root)
    if revision and revision != REFERENCE_REVISION:
        raise SystemExit(
            f"reference revision mismatch: {root} is {revision}, "
            f"manifest pins {REFERENCE_REVISION}"
        )
    sources = []
    source_paths = {}
    for entry in SOURCES:
        path = root / entry["path"]
        if not path.exists():
            raise SystemExit(f"reference source missing: {path}")
        source_paths[entry["id"]] = path
        suffix = path.suffix.lower()
        if suffix == ".png":
            kind = "image"
            meta = _image_meta(path)
        elif suffix == ".json":
            kind = "json"
            meta = {"icc": {"present": False, "assumed": "not an image"}}
        else:
            kind = "text"
            meta = {"icc": {"present": False, "assumed": "not an image"}}
        sources.append(
            {
                **entry,
                "kind": kind,
                "sha256": _sha256(path),
                "bytes": path.stat().st_size,
                **meta,
            }
        )
    json_sections = []
    for rel, section in JSON_SECTIONS:
        path = root / rel
        if not path.exists():
            raise SystemExit(f"reference JSON missing: {path}")
        data = json.loads(path.read_text())
        try:
            value = _section(data, section)
        except (KeyError, IndexError) as exc:
            raise SystemExit(f"reference JSON section missing: {rel}#{section}: {exc}")
        json_sections.append(
            {
                "path": rel,
                "section": section or "(root)",
                "sha256": _sha256_bytes(_canonical(value)),
                "bytes": len(_canonical(value)),
            }
        )
    scripts = []
    for rel in SCRIPTS:
        path = root / rel
        if not path.exists():
            raise SystemExit(f"reference analysis script missing: {path}")
        scripts.append({"path": rel, "sha256": _sha256(path), "bytes": path.stat().st_size})
    crops = []
    for entry in CROPS:
        source = source_paths[entry["source"]]
        try:
            derived = _crop_hash(source, entry["rect_px"])
        except ValueError as exc:
            raise SystemExit(f"reference crop {entry['id']}: {exc}")
        crops.append(
            {
                **entry,
                "rect_pt": [round(v / 2.0, 1) for v in entry["rect_px"]],
                **derived,
            }
        )
    return {
        "schema": 1,
        "generated_by": "scripts/t194-reference-manifest.py",
        "reference": {
            "revision": revision or REFERENCE_REVISION,
            "root_override": ENV_ROOT,
            "cli_override": "--reference-root PATH",
            "default_root": "../macos-liquid-glass",
            "checkout_optional": True,
        },
        "workstation": {
            "os": "macOS 27.0.1 (build 26A434)",
            "model": "Mac17,2 (Apple M5)",
            "display": "Built-in Liquid Retina XDR, 3024x1964 native, 1800x1169 pt @2x, Display P3",
            "display_scale": 2,
            "capture_pixels": [3600, 2338],
            "logical_points": [1800, 1169],
        },
        "icc": {
            "source": "embedded Display P3 profile where present; no profile is treated as sRGB",
            "scoring": "sRGB",
            "conversion": "PIL.ImageCms.profileToProfile(source, embedded, sRGB)",
            "note": "keep source pixels and sRGB-derived targets separate from the GPU's internal working space",
        },
        "sources": sources,
        "json_sections": json_sections,
        "scripts": scripts,
        "crops": crops,
        "derived_targets": _derived_targets(root),
        "gaps": _gaps(),
        "commands": {
            "write": "python3 scripts/t194-reference-manifest.py --write",
            "verify": "python3 scripts/t194-reference-manifest.py --verify",
            "structural": "python3 scripts/t194-reference-manifest.py --check-manifest",
        },
    }


def _check_manifest_data(manifest: dict) -> list[str]:
    problems = []
    if manifest.get("schema") != 1:
        problems.append("schema must be 1")
    for key in ("reference", "workstation", "icc", "sources", "json_sections", "scripts", "crops", "derived_targets", "gaps"):
        if key not in manifest:
            problems.append(f"missing key {key}")
    source_ids = {entry.get("id") for entry in manifest.get("sources", [])}
    for entry in manifest.get("sources", []):
        if not entry.get("sha256") or len(entry["sha256"]) != 64:
            problems.append(f"source {entry.get('id')}: bad sha256")
        if not entry.get("path"):
            problems.append(f"source {entry.get('id')}: missing path")
        if entry.get("kind") == "image" and not entry.get("icc"):
            problems.append(f"source {entry.get('id')}: missing ICC assumption")
    for entry in manifest.get("crops", []):
        if entry.get("source") not in source_ids:
            problems.append(f"crop {entry.get('id')}: unknown source {entry.get('source')}")
        if not entry.get("sha256_srgb"):
            problems.append(f"crop {entry.get('id')}: missing sRGB hash")
        if len(entry.get("rect_px", [])) != 4:
            problems.append(f"crop {entry.get('id')}: bad rect")
    for entry in manifest.get("scripts", []):
        if not entry.get("sha256") or len(entry["sha256"]) != 64:
            problems.append(f"script {entry.get('path')}: bad sha256")
    for entry in manifest.get("gaps", []):
        if entry.get("status") not in ("missing", "unmapped", "rejected", "open"):
            problems.append(f"gap {entry.get('id')}: bad status {entry.get('status')}")
        if not entry.get("owner"):
            problems.append(f"gap {entry.get('id')}: missing owner")
    return problems


def verify_manifest(manifest: dict, root: Path) -> int:
    problems = []
    revision = _git_revision(root)
    if revision and revision != manifest["reference"]["revision"]:
        problems.append(
            f"revision mismatch: checkout {revision} != manifest {manifest['reference']['revision']}"
        )
    for entry in manifest["sources"]:
        path = root / entry["path"]
        if not path.exists():
            problems.append(f"source missing: {entry['path']}")
            continue
        if _sha256(path) != entry["sha256"]:
            problems.append(f"source hash mismatch: {entry['path']}")
        if entry.get("kind") == "image":
            meta = _image_meta(path)
            if meta["size"] != entry.get("size"):
                problems.append(
                    f"source size mismatch: {entry['path']} {meta['size']} != {entry.get('size')}"
                )
    for entry in manifest["scripts"]:
        path = root / entry["path"]
        if not path.exists():
            problems.append(f"script missing: {entry['path']}")
        elif _sha256(path) != entry["sha256"]:
            problems.append(f"script hash mismatch: {entry['path']}")
    for entry in manifest["json_sections"]:
        path = root / entry["path"]
        if not path.exists():
            problems.append(f"JSON missing: {entry['path']}")
            continue
        data = json.loads(path.read_text())
        section = "" if entry["section"] == "(root)" else entry["section"]
        try:
            value = _section(data, section)
        except (KeyError, IndexError):
            problems.append(f"JSON section missing: {entry['path']}#{entry['section']}")
            continue
        if _sha256_bytes(_canonical(value)) != entry["sha256"]:
            problems.append(f"JSON section hash mismatch: {entry['path']}#{entry['section']}")
    source_paths = {entry["id"]: root / entry["path"] for entry in manifest["sources"]}
    for entry in manifest["crops"]:
        source = source_paths[entry["source"]]
        if not source.exists():
            problems.append(f"crop source missing: {entry['source']}")
            continue
        try:
            derived = _crop_hash(source, entry["rect_px"])
        except ValueError as exc:
            problems.append(f"crop {entry['id']}: {exc}")
            continue
        if derived["sha256_srgb"] != entry["sha256_srgb"]:
            problems.append(f"crop hash mismatch: {entry['id']}")
    if problems:
        print("t194-reference-manifest: REFERENCE FIXTURE MISMATCH", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        print(
            f"  root: {root}\n"
            f"  override: {ENV_ROOT}=/path/to/macos-liquid-glass or --reference-root",
            file=sys.stderr,
        )
        return 2
    print(f"t194-reference-manifest: verify PASS ({root}, revision {revision or 'unknown'})")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", action="store_true")
    group.add_argument("--verify", action="store_true")
    group.add_argument("--check-manifest", action="store_true")
    parser.add_argument("--reference-root")
    parser.add_argument("--out", type=Path, default=MANIFEST)
    args = parser.parse_args()

    if args.check_manifest:
        if not args.out.exists():
            print(f"t194-reference-manifest: manifest missing: {args.out}", file=sys.stderr)
            return 2
        manifest = json.loads(args.out.read_text())
        problems = _check_manifest_data(manifest)
        if problems:
            print("t194-reference-manifest: STRUCTURE FAIL", file=sys.stderr)
            for problem in problems:
                print(f"  {problem}", file=sys.stderr)
            return 2
        print(
            f"t194-reference-manifest: structure PASS "
            f"({len(manifest['sources'])} sources, {len(manifest['crops'])} crops, "
            f"{len(manifest['gaps'])} gaps)"
        )
        return 0

    root = _resolve_root(args.reference_root)
    if root is None:
        print(
            "t194-reference-manifest: reference root not found\n"
            f"  set {ENV_ROOT}=/path/to/macos-liquid-glass or pass --reference-root\n"
            f"  default: {DEFAULT_ROOT}",
            file=sys.stderr,
        )
        return 2
    if not root.is_dir():
        print(
            f"t194-reference-manifest: reference root is not a directory: {root}\n"
            f"  set {ENV_ROOT} or pass --reference-root",
            file=sys.stderr,
        )
        return 2

    if args.write:
        manifest = build_manifest(root)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        print(
            f"t194-reference-manifest: wrote {args.out} "
            f"({len(manifest['sources'])} sources, {len(manifest['json_sections'])} "
            f"JSON sections, {len(manifest['crops'])} crops)"
        )
        return 0

    manifest = json.loads(args.out.read_text())
    problems = _check_manifest_data(manifest)
    if problems:
        print("t194-reference-manifest: STRUCTURE FAIL", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 2
    return verify_manifest(manifest, root)


if __name__ == "__main__":
    sys.exit(main())
