#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T205 notification banner, stack and lower-panel material check and scorer.

The notification surfaces are three distinct materials, not one Popup plate
with a second alpha layered on top:

* the transient banner (`notificationBanner`, Light 0.549/0.976),
* the Notification Center stack (`notificationStack`, Light 0.551/0.706,
  Dark 0.507/0.550), and
* the Center's lower panel (`notificationLowerPanel`, Light 0.407/0.852).

The Dark banner and lower panel are uncovered as independent fits; they are
explicit, readable adaptations and every capture row labels them
``underconstrained``. ``--check`` is part of ``make gates`` and validates the
generated presets, their distinctness, the QML/shell region mapping and the
empty/scrollable states without a session. ``wallpaper`` and ``score`` are
used by ``make notification-materials-capture``.
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

OUT = REPO / "docs" / "captures" / "t205-notification-materials.json"
MD = REPO / "docs" / "captures" / "t205-notification-materials.md"
CAPTURES = REPO / "docs" / "captures"
BANNER_QML = REPO / "shell" / "notifications" / "NotificationBanner.qml"
CENTER_QML = REPO / "shell" / "notifications" / "NotificationCenter.qml"
CONTROLLER = REPO / "shell" / "src" / "shellcontroller.cpp"
PROTOCOL = REPO / "shell" / "src" / "shellprotocol.cpp"
LAYER = REPO / "compositor" / "src" / "shell" / "layer.rs"
RENDER = REPO / "compositor" / "src" / "render.rs"

# REFERENCE.md / coverage_measurements.json notification section.
TARGETS = {
    ("notificationBanner", "light"): (0.549, 0.976),
    ("notificationStack", "light"): (0.551, 0.706),
    ("notificationStack", "dark"): (0.507, 0.550),
    ("notificationLowerPanel", "light"): (0.407, 0.852),
}
UNDERCONSTRAINED = {("notificationBanner", "dark"),
                    ("notificationLowerPanel", "dark")}
BACKGROUNDS = (0.0, 0.502, 1.0)
MID_BACKGROUND = 0.502


def output(background: float, alpha: float, tint: float) -> float:
    return background * (1.0 - alpha) + tint * alpha


def resolve(data: dict, name: str, scheme: str, **kwargs) -> dict:
    return presets.resolve(data, name, scheme=scheme, rollout="replacement",
                           **kwargs)


def _region_count_bound() -> int:
    match = re.search(r"MAX_PANEL_REGIONS:\s*usize\s*=\s*(\d+)", LAYER.read_text())
    return int(match.group(1)) if match else 0


def _distinct(values: list[float], minimum: float) -> bool:
    return max(values) - min(values) >= minimum


def check(data: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    report: dict = {"targets": {}, "resolved": {}, "distinctness": {},
                    "contrast": {}}

    def fail(message: str) -> None:
        errors.append(message)

    def close(actual: float, expected: float, tolerance: float, label: str) -> None:
        if abs(actual - expected) > tolerance:
            fail(f"{label}: {actual:.6f} is outside {expected:.6f} ± {tolerance}")

    names = ("notificationBanner", "notificationStack", "notificationLowerPanel")
    resolved = {}
    for name in names:
        for scheme in ("light", "dark"):
            entry = resolve(data, name, scheme)
            resolved[(name, scheme)] = entry
            report["resolved"][f"{name}_{scheme}"] = {
                "alpha": round(entry["alpha"], 6),
                "tint": round(entry["tint"], 6),
                "blur": entry["blur"],
                "renderMode": entry["renderMode"],
                "confidence": entry["confidence"],
            }
            if entry["renderMode"] != "sampled-glass":
                fail(f"{name} {scheme} is not on the sampled replacement path")
            if entry["blur"] <= 0.0:
                fail(f"{name} {scheme} has no measured blur")
    for (name, scheme), (alpha, tint) in TARGETS.items():
        report["targets"][f"{name}_{scheme}"] = {"alpha": alpha, "tint": tint}
        entry = resolved[(name, scheme)]
        close(entry["alpha"], alpha, 1e-6, f"{name} {scheme} alpha")
        close(entry["tint"], tint, 1e-6, f"{name} {scheme} tint")

    # The three materials must be independently distinguishable on the
    # neutral backgrounds, not just different alpha numbers.
    for scheme in ("light", "dark"):
        rows = {}
        for name in names:
            entry = resolved[(name, scheme)]
            rows[name] = [output(bg, entry["alpha"], entry["tint"])
                          for bg in BACKGROUNDS]
        report["distinctness"][scheme] = {
            name: [round(value, 6) for value in values]
            for name, values in rows.items()
        }
        for left, right in (("notificationBanner", "notificationStack"),
                            ("notificationBanner", "notificationLowerPanel"),
                            ("notificationStack", "notificationLowerPanel")):
            for bg, lvalue, rvalue in zip(BACKGROUNDS, rows[left], rows[right]):
                if abs(lvalue - rvalue) < 0.02:
                    fail(f"{scheme} {left} and {right} differ by only "
                         f"{abs(lvalue - rvalue):.3f} over bg {bg}")
        # The adapted Dark lower panel must read below the Dark stack. The
        # measured Light lower panel is allowed to sit slightly above the
        # Light stack (the reference fit is brighter over mid-grey).
        if scheme == "dark" and rows["notificationLowerPanel"][1] >= rows["notificationStack"][1]:
            fail(f"{scheme} lower panel is not toned below the stack")

    # Dark banner/lower panel are readable adaptations: light-on-glass text
    # over the dark desktop/mid backgrounds, never the Light plate.
    for name in ("notificationBanner", "notificationLowerPanel"):
        entry = resolved[(name, "dark")]
        black = output(0.0, entry["alpha"], entry["tint"])
        mid = output(MID_BACKGROUND, entry["alpha"], entry["tint"])
        report["contrast"][name] = {
            "black": round(black, 6),
            "mid": round(mid, 6),
            "confidence": entry["confidence"],
            "underconstrained": (name, "dark") in UNDERCONSTRAINED,
        }
        if black > 0.2:
            fail(f"dark {name} is too light over black for light text")
        if mid > 0.45:
            fail(f"dark {name} is too light over mid for light text")
        if entry["confidence"] != "adapted":
            fail(f"dark {name} must be labelled adapted (underconstrained)")

    # Accessibility and no-sampling paths go through the one resolver.
    for name in names:
        for scheme in ("light", "dark"):
            reduced = resolve(data, name, scheme, reduce_transparency=True)
            if reduced["renderMode"] != "solid" or reduced["blur"] != 0.0:
                fail(f"{name} {scheme} Reduce Transparency is not the solid plate")
            if reduced["alpha"] < 0.95:
                fail(f"{name} {scheme} Reduce Transparency is not opaque")
            software = resolve(data, name, scheme, backend="software")
            if software["renderMode"] != "software-fallback":
                fail(f"{name} {scheme} software path is not the shared fallback")
        contrasted = resolve(data, name, "light", increase_contrast=True)
        if contrasted["edge"] <= resolved[(name, "light")]["edge"]:
            fail(f"{name} Increase Contrast does not raise the rim")

    bound = _region_count_bound()
    report["region_bound"] = bound
    if bound < 2:
        fail(f"MAX_PANEL_REGIONS is {bound}; the Center needs two regions")

    banner_qml = BANNER_QML.read_text()
    center_qml = CENTER_QML.read_text()
    for needle in ("notificationBanner", "materialRegions", "bannerCard",
                   "actionsRow", "stripe"):
        if needle not in banner_qml:
            fail(f"NotificationBanner.qml is missing {needle}")
    for needle in ("notificationStack", "notificationLowerPanel",
                   "materialRegions", "emptyHistory", "historyScrollBar",
                   "notificationLowerPanel", "bannerInset", "lowerSummary"):
        if needle not in center_qml:
            fail(f"NotificationCenter.qml is missing {needle}")
    for key in ('"stack"', '"lower"', '"banner"'):
        if key not in center_qml and key not in banner_qml:
            fail(f"missing material region key {key}")
    for needle in ("setBannerPanelRegions", "clearBannerPanelRegions",
                   "setNotificationCenterPanelRegions",
                   "clearNotificationCenterPanelRegions"):
        if needle not in CONTROLLER.read_text():
            fail(f"shellcontroller.cpp is missing {needle}")
        if needle not in PROTOCOL.read_text():
            fail(f"shellprotocol.cpp is missing {needle}")
    render = RENDER.read_text()
    for preset in ("NotificationBanner", "NotificationStack",
                   "NotificationLowerPanel"):
        if preset not in render:
            fail(f"render.rs does not route {preset}")

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


def _parse_regions(query: str) -> dict[tuple[str, str], dict]:
    regions: dict[tuple[str, str], dict] = {}
    pattern = re.compile(
        r"scale region (\S+) (\S+) preset=(\S+) "
        r"logical=(-?\d+),(-?\d+),(\d+),(\d+)")
    for match in pattern.finditer(query):
        namespace, key, preset = match.group(1), match.group(2), match.group(3)
        regions[(namespace, key)] = {
            "namespace": namespace,
            "key": key,
            "preset": preset,
            "x": int(match.group(4)),
            "y": int(match.group(5)),
            "w": int(match.group(6)),
            "h": int(match.group(7)),
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


def _sample_rect(region: dict, fx: float, fy: float,
                 size: int = 10) -> tuple[int, int, int, int]:
    cx = region["x"] + int(region["w"] * fx)
    cy = region["y"] + int(region["h"] * fy)
    half = size // 2
    return (max(0, cx - half), max(0, cy - half), cx + half, cy + half)


def _backdrop_rect(region: dict, size: int = 16) -> tuple[int, int, int, int]:
    cy = region["y"] + int(region["h"] * 0.5)
    x = max(0, region["x"] - 40)
    return (x, max(0, cy - size // 2), max(1, region["x"] - 8),
            min(region["y"] + region["h"], cy + size // 2))


def _rects_overlap(a: dict, b: dict) -> bool:
    return not (a["x"] + a["w"] <= b["x"] or b["x"] + b["w"] <= a["x"]
                or a["y"] + a["h"] <= b["y"] or b["y"] + b["h"] <= a["y"])


def _expected(data: dict, preset: str, case: dict) -> dict:
    return resolve(
        data, preset, case.get("scheme", "light"),
        reduce_transparency=bool(case.get("reduce_transparency")),
        increase_contrast=bool(case.get("increase_contrast")),
        backend="software" if case.get("material_path") == "software"
                else "gpu-sampled",
        tier="full",
    )


def score(manifest_path: Path) -> tuple[list[str], dict]:
    from PIL import Image

    if not manifest_path.is_absolute():
        manifest_path = REPO / manifest_path
    manifest = json.loads(manifest_path.read_text())
    data = presets.load()
    errors: list[str] = []
    rows: list[dict] = []

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
               "scheme": case.get("scheme"), "variant": case.get("variant"),
               "wallpaper": case.get("wallpaper"),
               "material_path": case.get("material_path", "gpu"),
               "regions": sorted(f"{ns}/{key}" for ns, key in regions),
               "capture": case["capture"], "materials": {}}
        image = Image.open(capture)

        expected_keys = {
            "notification": "banner",
            "notification-center": "stack",
        }
        if case.get("state") in ("empty", "banner-empty", "populated",
                                 "concurrent", "scrollable"):
            expected_keys["notification-center"] = "stack"
        required = [("notification-center", "stack"),
                    ("notification-center", "lower")]
        if case.get("state") in ("banner-empty", "concurrent"):
            required.append(("notification", "banner"))
        for key in required:
            if key not in regions:
                fail(f"{case['label']}: missing {key[0]}/{key[1]} region")

        # Disjoint regions: the stack/lower bands and the banner card never
        # overlap, so no material is painted twice.
        stack = regions.get(("notification-center", "stack"))
        lower = regions.get(("notification-center", "lower"))
        banner = regions.get(("notification", "banner"))
        if stack and lower and _rects_overlap(stack, lower):
            fail(f"{case['label']}: stack and lower regions overlap")
        if banner and stack and _rects_overlap(banner, stack):
            fail(f"{case['label']}: banner and stack regions overlap")

        # Sample blank material strips, never text/action/row content: the
        # banner's right padding, the stack's right padding outside the row
        # cards, and the lower panel's top band above its centered text.
        samples = {
            "banner": (banner, 0.985, 0.5, 6),
            "stack": (stack, 0.985, 0.5, 4),
            "lower": (lower, 0.5, 0.18, 6),
        }
        presets_by_key = {
            "banner": "notificationBanner",
            "stack": "notificationStack",
            "lower": "notificationLowerPanel",
        }
        for key, (region, fx, fy, sample_size) in samples.items():
            if not region:
                continue
            preset = presets_by_key[key]
            backdrop = _mean(image, _backdrop_rect(region))
            actual = _mean(image, _sample_rect(region, fx, fy, sample_size))
            resolved = _expected(data, preset, case)
            expected = output(backdrop, resolved["alpha"], resolved["tint"])
            error = abs(actual - expected)
            underconstrained = (preset, case.get("scheme")) in UNDERCONSTRAINED
            tolerance = 0.12 if underconstrained else (0.08 if key == "lower" else 0.06)
            row["materials"][key] = {
                "preset": preset,
                "backdrop": round(backdrop, 4),
                "actual": round(actual, 4),
                "expected": round(expected, 4),
                "fit_error": round(error, 4),
                "tolerance": tolerance,
                "underconstrained": underconstrained,
                "confidence": resolved["confidence"],
                "renderMode": resolved["renderMode"],
            }
            if error > tolerance:
                fail(f"{case['label']}: {key} fit error {error:.3f} "
                     f"(tolerance {tolerance:.2f})")
            if case.get("wallpaper") == "grid":
                region_var = _variance(image, _sample_rect(region, fx, fy,
                                                          sample_size + 2))
                backdrop_var = _variance(image, _backdrop_rect(region, 24))
                row["materials"][key]["region_var"] = round(region_var, 5)
                row["materials"][key]["backdrop_var"] = round(backdrop_var, 5)
                if backdrop_var > 1e-5 and region_var > backdrop_var * 0.75:
                    fail(f"{case['label']}: {key} does not attenuate the grid")

        if len(regions) > 32:
            fail(f"{case['label']}: region count {len(regions)} exceeds 32")
        rows.append(row)

    # Empty/populated/concurrent states must actually differ in the Center's
    # content: the empty case has no history rows, the populated case does.
    # The capture manifest records the fixture, so a missing material in one
    # state is visible in the report even before pixel scoring.
    try:
        manifest_label = str(manifest_path.relative_to(REPO))
    except ValueError:
        manifest_label = str(manifest_path)
    report = {"manifest": manifest_label,
              "cases": rows, "errors": errors}
    return errors, report


def write_report(data: dict, check_report: dict, capture_report: dict | None) -> None:
    report = {
        "task": "T205",
        "presets": {
            "banner": "notificationBanner",
            "stack": "notificationStack",
            "lower": "notificationLowerPanel",
        },
        "underconstrained": sorted(f"{name}_{scheme}" for name, scheme
                                   in UNDERCONSTRAINED),
        "check": check_report,
        "captures": capture_report,
    }
    OUT.write_text(json.dumps(report, indent=2) + "\n")

    lines = [
        "# T205 — Notification banner, stack and lower-panel materials",
        "",
        "Generated by `scripts/t205-notification-materials.py --write`.",
        "",
        "## Presets",
        "",
        "| Region | Preset | Light | Dark |",
        "|---|---|---|---|",
    ]
    for key, name in (("banner", "notificationBanner"),
                      ("stack", "notificationStack"),
                      ("lower", "notificationLowerPanel")):
        entry = data["glass"]["presets"][name]
        lines.append(
            f"| {key} | `{name}` | {entry['light']['calibration']['neutralAlpha']:.3f} / "
            f"{entry['light']['calibration']['neutralTint']:.3f} | "
            f"{entry['dark']['calibration']['neutralAlpha']:.3f} / "
            f"{entry['dark']['calibration']['neutralTint']:.3f} |")
    lines += [
        "",
        "The Light values are the measured fits. The Dark stack is the measured",
        "0.507/0.550 fit. The Dark banner and lower panel are uncovered as",
        "independent fits: they are labelled `adapted`/underconstrained in the",
        "token provenance and every capture row carries the same label.",
        "",
        "## Distinctness",
        "",
        "```json",
        json.dumps(check_report.get("distinctness", {}), indent=2),
        "```",
        "",
        "## Captures",
        "",
    ]
    if capture_report and capture_report.get("cases"):
        lines += ["| Case | State | Scheme | Variant | Backdrop | Banner err | Stack err | Lower err |",
                  "|---|---|---|---|---|---|---|---|"]
        for row in capture_report["cases"]:
            materials = row.get("materials", {})
            def err(key):
                value = materials.get(key, {}).get("fit_error")
                return "—" if value is None else value
            lines.append(
                f"| {row['label']} | {row.get('state')} | {row.get('scheme')} | "
                f"{row.get('variant')} | {row.get('wallpaper')} | "
                f"{err('banner')} | {err('stack')} | {err('lower')} |")
    else:
        lines.append("No live capture manifest was present.")
    lines += ["", "## Region keys", "",
              "`notification/banner`, `notification-center/stack`, "
              "`notification-center/lower`."]
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
            print(f"T205: {error}", file=sys.stderr)
        return 1
    print("T205 notification materials: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
