#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T203 Control Center material fixture, capture scorer and acceptance check.

The Control Center is three distinct materials, not one panel with a second
alpha layered on top:

* the outer plate (Light 0.265/0.373, Dark 0.489/0.677),
* each module capsule (the Light Wi-Fi fit 0.391/0.317 is an observed
  *effective* stacked response; the preset stores the own-layer value solved
  against the outer plate), and
* an expanded list (Dark near-background-independent luma ~0.13).

``--check`` is part of ``make gates``. It validates the generated presets, the
composite arithmetic, the region bound and the QML/shell mapping without a
session. ``wallpaper`` and ``score`` are used by
``make control-center-materials-capture``.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import material_presets as presets  # noqa: E402

OUT = REPO / "docs" / "captures" / "t203-control-center.json"
MD = REPO / "docs" / "captures" / "t203-control-center.md"
CAPTURES = REPO / "docs" / "captures"
QML = REPO / "shell" / "control-center" / "ControlCenter.qml"
CONTROLLER = REPO / "shell" / "src" / "shellcontroller.cpp"
PROTOCOL = REPO / "shell" / "src" / "shellprotocol.cpp"
LAYER = REPO / "compositor" / "src" / "shell" / "layer.rs"

TARGET_OUTER = {"light": (0.265, 0.373), "dark": (0.489, 0.677)}
TARGET_MODULE_EFFECTIVE = {"light": (0.391, 0.317)}
TARGET_EXPANDED_DARK_LUMA = 0.13
MID_BACKGROUND = 0.502


def output(background: float, alpha: float, tint: float) -> float:
    return background * (1.0 - alpha) + tint * alpha


def resolve(data: dict, name: str, scheme: str, **kwargs) -> dict:
    return presets.resolve(data, name, scheme=scheme, rollout="replacement",
                           **kwargs)


def _region_count_bound() -> int:
    match = re.search(r"MAX_PANEL_REGIONS:\s*usize\s*=\s*(\d+)", LAYER.read_text())
    return int(match.group(1)) if match else 0


def check(data: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    report: dict = {"targets": {}, "resolved": {}, "composite": {}}

    def fail(message: str) -> None:
        errors.append(message)

    def close(actual: float, expected: float, tolerance: float, label: str) -> None:
        if abs(actual - expected) > tolerance:
            fail(f"{label}: {actual:.6f} is outside {expected:.6f} ± {tolerance}")

    outer = {scheme: resolve(data, "controlCenterOuter", scheme)
             for scheme in ("light", "dark")}
    module = {scheme: resolve(data, "controlCenterModule", scheme)
              for scheme in ("light", "dark")}
    expanded = {scheme: resolve(data, "controlCenterExpanded", scheme)
                for scheme in ("light", "dark")}

    for scheme, (alpha, tint) in TARGET_OUTER.items():
        report["targets"][f"outer_{scheme}"] = {"alpha": alpha, "tint": tint}
        report["resolved"][f"outer_{scheme}"] = {
            "alpha": round(outer[scheme]["alpha"], 6),
            "tint": round(outer[scheme]["tint"], 6),
            "blur": outer[scheme]["blur"],
            "edge": outer[scheme]["edge"],
            "corner_peak": outer[scheme]["cornerLensPeak"],
            "corner_distance": outer[scheme]["cornerLensDistance"],
        }
        close(outer[scheme]["alpha"], alpha, 1e-6, f"{scheme} outer alpha")
        close(outer[scheme]["tint"], tint, 1e-6, f"{scheme} outer tint")
        if outer[scheme]["blur"] > 10.0:
            fail(f"{scheme} outer blur is not the measured 8 pt family")
        if outer[scheme]["edge"] < 0.25:
            fail(f"{scheme} outer rim is too weak for the reference capsule")
        if outer[scheme]["cornerLensPeak"] < 5.0:
            fail(f"{scheme} outer capsule-end lens is too weak")
        if not 18.0 <= outer[scheme]["cornerLensDistance"] <= 30.0:
            fail(f"{scheme} outer capsule-end lens distance is off the 24 pt fit")

    light_alpha, light_tint = module["light"]["alpha"], module["light"]["tint"]
    report["targets"]["module_light_effective"] = {
        "alpha": TARGET_MODULE_EFFECTIVE["light"][0],
        "tint": TARGET_MODULE_EFFECTIVE["light"][1],
    }
    report["resolved"]["module_light"] = {
        "alpha": round(module["light"]["alpha"], 6),
        "tint": round(module["light"]["tint"], 6),
    }
    # The compositor's flat-band output is opaque: the module preset is the
    # final composite in its own region, not a second alpha over the outer.
    report["composite"]["module_light"] = {
        "alpha": round(light_alpha, 6),
        "tint": round(light_tint, 6),
    }
    close(light_alpha, TARGET_MODULE_EFFECTIVE["light"][0], 1e-6,
          "light module effective alpha")
    close(light_tint, TARGET_MODULE_EFFECTIVE["light"][1], 1e-6,
          "light module effective tint")

    dark_alpha, dark_tint = module["dark"]["alpha"], module["dark"]["tint"]
    report["composite"]["module_dark"] = {
        "alpha": round(dark_alpha, 6),
        "tint": round(dark_tint, 6),
    }
    if not (dark_alpha > outer["dark"]["alpha"]
            and dark_tint < outer["dark"]["tint"]):
        fail("dark module composite must read darker than the dark outer plate")
    if module["dark"]["alpha"] < 0.55 or module["dark"]["alpha"] > 0.75:
        fail("dark module effective alpha is outside the adapted 0.65 range")

    expanded_dark = expanded["dark"]
    dark_outputs = [output(bg, expanded_dark["alpha"], expanded_dark["tint"])
                    for bg in (0.0, MID_BACKGROUND, 1.0)]
    spread = max(dark_outputs) - min(dark_outputs)
    mean = sum(dark_outputs) / len(dark_outputs)
    report["resolved"]["expanded_dark"] = {
        "alpha": round(expanded_dark["alpha"], 6),
        "tint": round(expanded_dark["tint"], 6),
        "outputs": [round(value, 6) for value in dark_outputs],
        "spread": round(spread, 6),
        "mean": round(mean, 6),
    }
    if spread > 0.05:
        fail(f"expanded dark is not background independent: spread {spread:.3f}")
    close(mean, TARGET_EXPANDED_DARK_LUMA, 0.02, "expanded dark mean luma")
    if expanded_dark["alpha"] < 0.94:
        fail("expanded dark must stay near opaque for the observed luma")

    expanded_light = expanded["light"]
    light_mid = output(MID_BACKGROUND, expanded_light["alpha"],
                       expanded_light["tint"])
    report["resolved"]["expanded_light"] = {
        "alpha": round(expanded_light["alpha"], 6),
        "tint": round(expanded_light["tint"], 6),
        "mid_output": round(light_mid, 6),
    }
    if light_mid < 0.8:
        fail("expanded light plate is not a readable light list")

    for scheme in ("light", "dark"):
        reduced = resolve(data, "controlCenterOuter", scheme,
                          reduce_transparency=True)
        report["resolved"][f"reduce_{scheme}"] = {
            "renderMode": reduced["renderMode"],
            "alpha": round(reduced["alpha"], 6),
            "tint": round(reduced["tint"], 6),
        }
        if reduced["renderMode"] != "solid" or reduced["alpha"] < 0.95:
            fail(f"{scheme} Reduce Transparency is not the opaque plate")
        if reduced["blur"] != 0.0:
            fail(f"{scheme} Reduce Transparency still schedules a blur")

    base_edge = outer["light"]["edge"]
    contrasted = resolve(data, "controlCenterOuter", "light",
                         increase_contrast=True)
    report["resolved"]["contrast_light"] = {
        "edge": round(contrasted["edge"], 6),
        "edgeWidth": round(contrasted["edgeWidth"], 6),
    }
    close(contrasted["edge"], base_edge + 0.35, 1e-6,
          "Increase Contrast outer rim")

    bound = _region_count_bound()
    report["region_bound"] = bound
    if bound < 22:
        fail(f"MAX_PANEL_REGIONS is {bound}; the Control Center needs 22")

    qml = QML.read_text()
    for needle in ("materialRegions", "controlCenterOuter",
                   "controlCenterModule", "controlCenterExpanded",
                   "expanded-wifi", "expanded-bluetooth", "wifiNetworkList",
                   "bluetoothExpandedList"):
        if needle not in qml:
            fail(f"ControlCenter.qml is missing {needle}")
    for needle in ("setControlCenterPanelRegions",
                   "clearControlCenterPanelRegions"):
        if needle not in CONTROLLER.read_text():
            fail(f"shellcontroller.cpp is missing {needle}")
        if needle not in PROTOCOL.read_text():
            fail(f"shellprotocol.cpp is missing {needle}")
    module_keys = set(re.findall(r'"(module-[a-z-]+)"', qml))
    report["module_keys"] = sorted(module_keys)
    if len(module_keys) < 18:
        fail(f"only {len(module_keys)} module region keys declared")
    for key in ("module-wifi", "module-bluetooth", "module-clipboard"):
        if f'"{key}"' not in qml:
            fail(f"missing module region key {key}")

    return errors, report


def wallpaper(kind: str, path: Path, width: int, height: int) -> None:
    from PIL import Image, ImageDraw

    image = Image.new("RGB", (width, height), (128, 128, 128))
    if kind == "grid":
        draw = ImageDraw.Draw(image)
        cell = 16
        for x in range(0, width, cell):
            draw.line((x, 0, x, height), fill=(238, 238, 238), width=1)
            if x + 1 < width:
                draw.line((x + 1, 0, x + 1, height), fill=(24, 24, 24), width=1)
        for y in range(0, height, cell):
            draw.line((0, y, width, y), fill=(238, 238, 238), width=1)
            if y + 1 < height:
                draw.line((0, y + 1, width, y + 1), fill=(24, 24, 24), width=1)
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path)


def _parse_regions(query: str) -> dict[str, dict]:
    regions: dict[str, dict] = {}
    pattern = re.compile(
        r"scale region control-center (\S+) preset=(\S+) "
        r"logical=(-?\d+),(-?\d+),(\d+),(\d+)")
    for match in pattern.finditer(query):
        key, preset = match.group(1), match.group(2)
        regions[key] = {
            "preset": preset,
            "x": int(match.group(3)),
            "y": int(match.group(4)),
            "w": int(match.group(5)),
            "h": int(match.group(6)),
        }
    return regions


def _mean(image, rect: tuple[int, int, int, int]) -> float:
    from PIL import ImageStat

    crop = image.crop(rect).convert("L")
    return ImageStat.Stat(crop).mean[0] / 255.0


def _variance(image, rect: tuple[int, int, int, int]) -> float:
    from PIL import ImageStat

    crop = image.crop(rect).convert("L")
    return ImageStat.Stat(crop).var[0] / (255.0 * 255.0)


def _sample_rect(region: dict, fx: float = 0.72, fy: float = 0.5,
                 size: int = 12) -> tuple[int, int, int, int]:
    cx = region["x"] + int(region["w"] * fx)
    cy = region["y"] + int(region["h"] * fy)
    half = size // 2
    return (max(0, cx - half), max(0, cy - half), cx + half, cy + half)


def score(manifest_path: Path) -> tuple[list[str], dict]:
    from PIL import Image

    if not manifest_path.is_absolute():
        manifest_path = REPO / manifest_path
    manifest = json.loads(manifest_path.read_text())
    errors: list[str] = []
    rows: list[dict] = []
    by_label = {case["label"]: case for case in manifest.get("cases", [])}

    def fail(message: str) -> None:
        errors.append(message)

    for case in manifest.get("cases", []):
        capture = REPO / case["capture"] if case.get("capture") else None
        query_path = REPO / case["query"] if case.get("query") else None
        if not capture or not capture.exists():
            fail(f"{case['label']}: missing capture")
            continue
        query = query_path.read_text() if query_path and query_path.exists() else ""
        regions = _parse_regions(query)
        row = {"label": case["label"], "state": case.get("state"),
               "scheme": case.get("scheme"),
               "wallpaper": case.get("wallpaper"),
               "regions": sorted(regions), "capture": case["capture"]}
        if "outer" not in regions or "module-wifi" not in regions:
            fail(f"{case['label']}: outer/module region metadata missing")
            rows.append(row)
            continue
        image = Image.open(capture)
        outer = regions["outer"]
        module = regions["module-wifi"]
        backdrop = _mean(image, (max(0, outer["x"] - 40), outer["y"] + 40,
                                 max(0, outer["x"] - 20), outer["y"] + 60))
        outer_sample = _mean(image, _sample_rect(outer, fx=0.02, size=6))
        # The module region includes the expanded list when Wi-Fi is open.
        # Sample the header band (above the list) so the score is the module
        # capsule, not the nested list on top of it.
        module_sample = _mean(image, _sample_rect(module, fx=0.75, fy=0.18,
                                                  size=10))
        row["backdrop"] = round(backdrop, 4)
        row["outer"] = round(outer_sample, 4)
        row["module"] = round(module_sample, 4)

        if case.get("wallpaper") == "neutral":
            data = presets.load()
            scheme = case.get("scheme", "light")
            reduce = bool(case.get("reduce_transparency"))
            contrast = bool(case.get("increase_contrast"))
            outer_preset = resolve(data, "controlCenterOuter", scheme,
                                   reduce_transparency=reduce,
                                   increase_contrast=contrast)
            module_preset = resolve(data, "controlCenterModule", scheme,
                                    reduce_transparency=reduce,
                                    increase_contrast=contrast)
            expected_outer = output(backdrop, outer_preset["alpha"],
                                    outer_preset["tint"])
            expected_module = output(backdrop, module_preset["alpha"],
                                     module_preset["tint"])
            row["outer_expected"] = round(expected_outer, 4)
            row["module_expected"] = round(expected_module, 4)
            row["outer_error"] = round(abs(outer_sample - expected_outer), 4)
            row["module_error"] = round(abs(module_sample - expected_module), 4)
            if row["outer_error"] > 0.06:
                fail(f"{case['label']}: outer error {row['outer_error']}")
            if row["module_error"] > 0.06:
                fail(f"{case['label']}: nested error {row['module_error']}")
            if "expanded-wifi" in regions:
                expanded = regions["expanded-wifi"]
                expanded_sample = _mean(image, _sample_rect(expanded))
                row["expanded"] = round(expanded_sample, 4)
                if scheme == "dark" and expanded_sample > 0.22:
                    fail(f"{case['label']}: expanded dark list is too bright")
                if scheme == "light" and expanded_sample < 0.75:
                    fail(f"{case['label']}: expanded light list is too dark")
        else:
            backdrop_var = _variance(image, (max(0, outer["x"] - 80),
                                             outer["y"] + 40,
                                             max(0, outer["x"] - 20),
                                             outer["y"] + 80))
            outer_var = _variance(image, _sample_rect(outer, fx=0.02, size=6))
            module_var = _variance(image, _sample_rect(module, fx=0.75, fy=0.18,
                                                       size=16))
            row["backdrop_var"] = round(backdrop_var, 5)
            row["outer_var"] = round(outer_var, 5)
            row["module_var"] = round(module_var, 5)
            if backdrop_var > 1e-5 and module_var > backdrop_var * 0.75:
                fail(f"{case['label']}: module does not attenuate the grid")
            if backdrop_var > 1e-5 and outer_var > backdrop_var * 0.75:
                fail(f"{case['label']}: outer plate does not attenuate the grid")
        rows.append(row)

    # Stale-region check: the expanded list's own rect must change between the
    # collapsed and expanded captures of the same scheme/backdrop. A stale
    # material region would make the two samples nearly equal.
    for scheme in ("light", "dark"):
        collapsed = by_label.get(f"collapsed-{scheme}-neutral")
        expanded = by_label.get(f"expanded-{scheme}-neutral")
        if not collapsed or not expanded:
            continue
        expanded_regions = _parse_regions(
            (REPO / expanded["query"]).read_text())
        collapsed_regions = _parse_regions(
            (REPO / collapsed["query"]).read_text())
        rect = expanded_regions.get("expanded-wifi")
        if not rect:
            fail(f"expanded-{scheme}-neutral: expanded region missing")
            continue
        expanded_image = Image.open(REPO / expanded["capture"])
        collapsed_image = Image.open(REPO / collapsed["capture"])
        sample = _sample_rect(rect)
        expanded_value = _mean(expanded_image, sample)
        collapsed_value = _mean(collapsed_image, sample)
        delta = abs(expanded_value - collapsed_value)
        if delta < 0.05:
            fail(f"{scheme}: expanded list did not move the visible material "
                 f"(delta {delta:.3f})")
        if "expanded-wifi" in collapsed_regions:
            fail(f"{scheme}: collapsed capture still declares expanded-wifi")

    report = {"manifest": str(manifest_path.relative_to(REPO)),
              "cases": rows, "errors": errors}
    return errors, report


def write_report(data: dict, check_report: dict, capture_report: dict | None) -> None:
    report = {
        "task": "T203",
        "presets": {
            "outer": "controlCenterOuter",
            "module": "controlCenterModule",
            "expanded": "controlCenterExpanded",
        },
        "check": check_report,
        "captures": capture_report,
    }
    OUT.write_text(json.dumps(report, indent=2) + "\n")

    lines = [
        "# T203 — Control Center panel and module materials",
        "",
        "Generated by `scripts/t203-control-center.py --write`.",
        "",
        "## Presets",
        "",
        "| Region | Preset | Light | Dark |",
        "|---|---|---|---|",
    ]
    for key, name in (("outer", "controlCenterOuter"),
                      ("module", "controlCenterModule"),
                      ("expanded", "controlCenterExpanded")):
        entry = data["glass"]["presets"][name]
        lines.append(
            f"| {key} | `{name}` | {entry['light']['calibration']['neutralAlpha']:.3f} / "
            f"{entry['light']['calibration']['neutralTint']:.3f} | "
            f"{entry['dark']['calibration']['neutralAlpha']:.3f} / "
            f"{entry['dark']['calibration']['neutralTint']:.3f} |")
    lines += [
        "",
        "The Light module value is the measured 0.391/0.317 effective fit.",
        "The compositor's flat-band output is opaque, so the module preset is the",
        "final composite in its own region; it is not a second alpha layered over",
        "the outer plate. The outer plate remains visible around the capsules.",
        "",
        "## Composite",
        "",
        "```json",
        json.dumps(check_report.get("composite", {}), indent=2),
        "```",
        "",
        "## Captures",
        "",
    ]
    if capture_report and capture_report.get("cases"):
        lines += ["| Case | State | Scheme | Backdrop | Outer error | Nested error |",
                  "|---|---|---|---|---|---|"]
        for row in capture_report["cases"]:
            lines.append(
                f"| {row['label']} | {row.get('state')} | {row.get('scheme')} | "
                f"{row.get('wallpaper')} | {row.get('outer_error', '—')} | "
                f"{row.get('module_error', '—')} |")
    else:
        lines.append("No live capture manifest was present.")
    if check_report.get("module_keys"):
        lines += ["", "## Region keys", "",
                  ", ".join(f"`{key}`" for key in check_report["module_keys"])]
    MD.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--score", type=Path)
    sub = parser.add_subparsers(dest="command")
    wall = sub.add_parser("wallpaper")
    wall.add_argument("--kind", choices=("neutral", "grid"), required=True)
    wall.add_argument("--out", type=Path, required=True)
    wall.add_argument("--width", type=int, default=1920)
    wall.add_argument("--height", type=int, default=1200)
    args = parser.parse_args()

    if args.command == "wallpaper":
        wallpaper(args.kind, args.out, args.width, args.height)
        return 0

    data = presets.load()
    errors, check_report = check(data)
    capture_report = None
    if args.score:
        capture_errors, capture_report = score(args.score)
        errors.extend(capture_errors)
    if args.write:
        write_report(data, check_report, capture_report)
    if errors:
        for error in errors:
            print(f"T203: {error}", file=sys.stderr)
        return 1
    print("T203 control center materials: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
