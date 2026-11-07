#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T206 first-party Settings/Files chrome check, capture scorer and report.

The first-party apps draw their own sidebar and toolbar on a transparent
window surface. T206 gives those two surfaces app-local optical presets:

* Settings sidebar Light 0.986/0.857, toolbar Light 0.961/0.965;
* Files sidebar Light 0.985/0.858, toolbar Light 0.994/0.922;
* Files sidebar Dark 0.935/0.218, toolbar Dark 0.900/0.247;
* Settings Dark is adapted from the measured Finder pair because System
  Settings Dark has no independent fit.

The presets are near-opaque scalar tint/alpha. The reference's small sidebar
chroma pickup is *not* representable by the current token transport, so the
capture report records the measured output chroma separately instead of
claiming a colour match.

``--check`` is part of ``make gates``. ``wallpaper`` and ``score`` are used by
``make first-party-chrome-capture``.
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

OUT = REPO / "docs" / "captures" / "t206-first-party-chrome.json"
MD = REPO / "docs" / "captures" / "t206-first-party-chrome.md"
CAPTURES = REPO / "docs" / "captures"
TOKENS = REPO / "design-system" / "tokens" / "tokens.json"
MATERIAL = REPO / "design-system" / "Material.qml"
GLASS = REPO / "design-system" / "Glass.qml"
APPWINDOW = REPO / "design-system" / "components" / "AppWindow.qml"
SIDEBAR = REPO / "design-system" / "components" / "Sidebar.qml"
TOOLBAR = REPO / "design-system" / "components" / "Toolbar.qml"
CHROMESHEET = REPO / "design-system" / "components" / "ChromeSheet.qml"
SETTINGS_SHELL = REPO / "apps" / "settings" / "SettingsShell.qml"
FILES_SHELL = REPO / "apps" / "files" / "FilesShell.qml"
FILES_PATHBAR = REPO / "apps" / "files" / "PathBar.qml"
FILES_SETTINGS = REPO / "apps" / "files" / "FilesSettings.cpp"
FILES_CMAKE = REPO / "apps" / "files" / "CMakeLists.txt"

# REFERENCE.md "Settings and Files" row plus the T206 Settings-Dark adaptation.
TARGETS = {
    ("settingsSidebar", "light"): (0.986, 0.857, "measured"),
    ("settingsToolbar", "light"): (0.961, 0.965, "measured"),
    ("settingsSidebar", "dark"): (0.935, 0.218, "adapted"),
    ("settingsToolbar", "dark"): (0.900, 0.247, "adapted"),
    ("filesSidebar", "light"): (0.985, 0.858, "measured"),
    ("filesToolbar", "light"): (0.994, 0.922, "measured"),
    ("filesSidebar", "dark"): (0.935, 0.218, "measured"),
    ("filesToolbar", "dark"): (0.900, 0.247, "measured"),
}
APP_SURFACES = {
    "settings": {"sidebar": "settingsSidebar", "toolbar": "settingsToolbar",
                 "width": 800, "height": 640, "sidebar_width": 192},
    "files": {"sidebar": "filesSidebar", "toolbar": "filesToolbar",
              "width": 980, "height": 640, "sidebar_width": 240},
}
TITLEBAR_HEIGHT = 40
TOOLBAR_HEIGHT = 52
TEXT = {
    "light": {"primary": (0.114, 0.090, 0.137),
              "secondary": (0.357, 0.306, 0.400)},
    "dark": {"primary": (0.973, 0.965, 0.980),
             "secondary": (0.765, 0.722, 0.800)},
}
# The content sheet and path bar stay opaque; these are the semantic tokens
# the app shells paint (`surface` and `chrome`).
OPAQUE = {
    "surface": {"light": (1.0, 1.0, 1.0), "dark": (0.114, 0.090, 0.137)},
    "chrome": {"light": (1.0, 1.0, 1.0), "dark": (0.176, 0.145, 0.204)},
}
FALLBACK = {
    ("settings", "light", "sidebar"): (0.973, 0.965, 0.980),  # surfaceMuted
    ("settings", "light", "toolbar"): (1.0, 1.0, 1.0),        # chrome
    ("settings", "dark", "sidebar"): (0.176, 0.145, 0.204),
    ("settings", "dark", "toolbar"): (0.176, 0.145, 0.204),
    ("files", "light", "sidebar"): (0.973, 0.965, 0.980),
    ("files", "light", "toolbar"): (1.0, 1.0, 1.0),
    ("files", "dark", "sidebar"): (0.176, 0.145, 0.204),
    ("files", "dark", "toolbar"): (0.176, 0.145, 0.204),
}


def _resolve(data: dict, name: str, scheme: str, **kwargs) -> dict:
    return presets.resolve(data, name, scheme=scheme, rollout="replacement",
                           **kwargs)


def check(data: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    report: dict = {"targets": {}, "resolved": {}, "files": {}, "chroma": {}}

    def fail(message: str) -> None:
        errors.append(message)

    def close(actual: float, expected: float, tolerance: float, label: str) -> None:
        if abs(actual - expected) > tolerance:
            fail(f"{label}: {actual:.6f} is outside {expected:.6f} ± {tolerance}")

    for (name, scheme), (alpha, tint, confidence) in TARGETS.items():
        result = _resolve(data, name, scheme)
        report["resolved"][f"{name}_{scheme}"] = {
            "alpha": round(result["alpha"], 6),
            "tint": round(result["tint"], 6),
            "renderMode": result["renderMode"],
            "confidence": result["confidence"],
        }
        report["targets"][f"{name}_{scheme}"] = {
            "alpha": alpha, "tint": tint, "confidence": confidence,
        }
        close(result["alpha"], alpha, 1e-6, f"{name} {scheme} alpha")
        close(result["tint"], tint, 1e-6, f"{name} {scheme} tint")
        if result["renderMode"] != "solid":
            fail(f"{name} {scheme} must be an app-local solid preset")
        if result["blur"] != 0.0:
            fail(f"{name} {scheme} must not request a sampled blur")
        if result["confidence"] != confidence:
            fail(f"{name} {scheme} confidence must be {confidence}")
        if result["alpha"] < 0.9:
            fail(f"{name} {scheme} must stay near-opaque")
        if result["fallbackReason"] != "none":
            fail(f"{name} {scheme} must not name a fallback on the normal path")
        reduced = _resolve(data, name, scheme, reduce_transparency=True)
        if reduced["renderMode"] != "solid" or reduced["alpha"] < 0.9:
            fail(f"{name} {scheme} Reduce Transparency must use the opaque plate")
        legacy = presets.resolve(data, name, scheme=scheme)
        if legacy["look"] != "legacy":
            fail(f"{name} {scheme} must keep the legacy look by default")

    # Files keeps its toolbar and sidebar independent in both schemes.
    for scheme in ("light", "dark"):
        sidebar = _resolve(data, "filesSidebar", scheme)
        toolbar = _resolve(data, "filesToolbar", scheme)
        if (sidebar["alpha"], sidebar["tint"]) == (toolbar["alpha"], toolbar["tint"]):
            fail(f"Files {scheme} sidebar and toolbar collapsed to one preset")

    # The app-local presets do not follow the light attached-menu slider.
    for name in presets.APP_CHROME_PRESETS:
        low = _resolve(data, name, "light", slider=0.0)
        high = _resolve(data, name, "light", slider=1.0)
        if low["sliderApplied"] or high["sliderApplied"]:
            fail(f"{name} must not follow the menu slider")
        close(low["alpha"], high["alpha"], 1e-9, f"{name} slider alpha")
        close(low["tint"], high["tint"], 1e-9, f"{name} slider tint")

    # The QML complement and the two app shells must consume the one helper,
    # not re-derive a second alpha curve.
    material = MATERIAL.read_text()
    for needle in ("appChromeResolution", "appChromeFill", "appChromeBorder",
                   "Glass.appChromeSupported"):
        if needle not in material:
            fail(f"Material.qml is missing {needle}")
    glass = GLASS.read_text()
    for needle in ("appChromeSupported", "settingsSidebar", "filesToolbar"):
        if needle not in glass:
            fail(f"Glass.qml is missing {needle}")
    appwindow = APPWINDOW.read_text()
    for needle in ("chromeTransparent", "titleBarBacking", "ChromeSheet"):
        if needle not in appwindow:
            fail(f"AppWindow.qml is missing {needle}")
    sidebar = SIDEBAR.read_text()
    for needle in ("fillColor", "bottomLeftRadius", "ChromeSheet"):
        if needle not in sidebar:
            fail(f"Sidebar.qml is missing {needle}")
    toolbar = TOOLBAR.read_text()
    for needle in ("fillColor", "borderColor"):
        if needle not in toolbar:
            fail(f"Toolbar.qml is missing {needle}")
    sheet = CHROMESHEET.read_text()
    for needle in ("topLeftRadius", "topRightRadius", "bottomLeftRadius",
                   "bottomRightRadius", "ShapePath"):
        if needle not in sheet:
            fail(f"ChromeSheet.qml is missing {needle}")
    pathbar = FILES_PATHBAR.read_text()
    for needle in ("fillColor", "bottomRightRadius", "ChromeSheet"):
        if needle not in pathbar:
            fail(f"PathBar.qml is missing {needle}")
    for shell, app, preset in ((SETTINGS_SHELL, "Settings", "settingsSidebar"),
                               (FILES_SHELL, "Files", "filesSidebar")):
        text = shell.read_text()
        for needle in (preset, "chromeTransparent", "Material.appChromeFill",
                       "Material.appChromeBorder", "appChromeSupported",
                       "Settings.values[\"appearance.colorScheme\"]",
                       "Settings.values[\"accessibility.reduceTransparency\"]",
                       "Settings.values[\"accessibility.increaseContrast\"]"):
            if needle not in text:
                fail(f"{app} shell is missing {needle}")
    files_settings = FILES_SETTINGS.read_text()
    for needle in ("DbusSettingsClient", "MockSettingsClient", "glassRollout",
                   "appChromeSupported"):
        if needle not in files_settings:
            fail(f"FilesSettings.cpp is missing {needle}")
    cmake = FILES_CMAKE.read_text()
    if "dragonfruit-settings-client" not in cmake:
        fail("Files must link the shared settings client")
    if "FilesSettings.cpp" not in cmake:
        fail("FilesSettings.cpp must be in the Files QML module")

    # The scalar preset has no chroma transport. Keep that explicit rather
    # than silently claiming the measured sidebar pickup is represented.
    report["chroma"] = {
        "transport": "scalar-neutral-tint",
        "measured_sidebar_pickup": "small, observed in REFERENCE.md",
        "unresolved": True,
        "note": "The token shape stores alpha/tint only; the measured chroma "
                "component is not carried. Captures report the rendered "
                "channel spread separately.",
    }
    return errors, report


def wallpaper(kind: str, path: Path, width: int, height: int) -> None:
    from PIL import Image

    if kind == "neutral":
        color = (128, 128, 128)
    elif kind == "saturated":
        color = (224, 32, 64)
    else:
        raise SystemExit(f"unknown wallpaper kind {kind!r}")
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.new("RGB", (width, height), color).save(path)


def _parse_windows(query: str) -> tuple[dict[int, dict], dict[int, dict]]:
    decorations: dict[int, dict] = {}
    physical: dict[int, dict] = {}
    decoration_re = re.compile(
        r"^decoration (\d+) (\d+) (-?\d+) (-?\d+) (\d+) (\d+) "
        r"(-?\d+) (-?\d+) (\d+) (\d+) (\S+) (-?\d+)$", re.MULTILINE)
    for match in decoration_re.finditer(query):
        window_id = int(match.group(1))
        decorations[window_id] = {
            "id": window_id,
            "ssd": match.group(2) == "1",
            "x": int(match.group(7)),
            "y": int(match.group(8)),
            "w": int(match.group(9)),
            "h": int(match.group(10)),
            "state": match.group(11),
        }
    scale_re = re.compile(
        r"^scale window (\d+) logical=(-?\d+) (-?\d+) (\d+) (\d+) "
        r"physical=(-?\d+) (-?\d+) (\d+) (\d+)$", re.MULTILINE)
    for match in scale_re.finditer(query):
        window_id = int(match.group(1))
        physical[window_id] = {
            "x": int(match.group(6)),
            "y": int(match.group(7)),
            "w": int(match.group(8)),
            "h": int(match.group(9)),
        }
    return decorations, physical


def _find_app_window(query: str, app: str) -> dict | None:
    decorations, physical = _parse_windows(query)
    candidates = [entry for entry in decorations.values() if not entry["ssd"]]
    if not candidates:
        return None
    expected = APP_SURFACES[app]
    candidates.sort(
        key=lambda entry: abs(entry["w"] - expected["width"])
        + abs(entry["h"] - expected["height"]))
    window = candidates[0]
    rect = physical.get(window["id"], window)
    scale = rect["w"] / window["w"] if window["w"] else 1.0
    return {
        "id": window["id"],
        "x": rect["x"],
        "y": rect["y"],
        "w": rect["w"],
        "h": rect["h"],
        "scale": scale,
        "logical": window,
    }


def _mean_rgb(image, rect: tuple[int, int, int, int]) -> tuple[float, float, float]:
    from PIL import ImageStat

    crop = image.crop(rect).convert("RGB")
    stat = ImageStat.Stat(crop)
    return tuple(value / 255.0 for value in stat.mean)


def _sample(window: dict, app: str, surface: str) -> tuple[int, int, int, int]:
    scale = window["scale"]
    width = max(3, int(4 * scale))
    if surface == "sidebar":
        x = window["x"] + int(2 * scale)
        y = window["y"] + int((TITLEBAR_HEIGHT + 80) * scale)
    else:
        sidebar = APP_SURFACES[app]["sidebar_width"]
        x = window["x"] + int((sidebar + 2) * scale)
        y = window["y"] + int((TITLEBAR_HEIGHT + 20) * scale)
    return (x, y, x + width, y + width)


def _sample_opaque(window: dict, app: str, surface: str) -> tuple[int, int, int, int]:
    scale = window["scale"]
    width = max(3, int(4 * scale))
    x = window["x"] + window["w"] - int(20 * scale) - width
    if surface == "pathbar":
        y = window["y"] + window["h"] - int(18 * scale) - width
    elif app == "files":
        # Above the 36 pt path bar, clear of the listing glyphs at the edge.
        y = window["y"] + window["h"] - int((36 + 20) * scale) - width
    else:
        y = window["y"] + window["h"] - int(20 * scale) - width
    return (x, y, x + width, y + width)


def _expected(app: str, scheme: str, surface: str, variant: str,
              data: dict, bg: tuple[float, float, float]) -> tuple[float, float, float]:
    if variant == "solid":
        return FALLBACK[(app, scheme, surface)]
    preset = APP_SURFACES[app][surface]
    if variant == "reduce":
        entry = data["glass"]["accessibility"]["reduceTransparency"][scheme]
        alpha, tint = float(entry["alpha"]), float(entry["tint"])
    else:
        resolved = _resolve(data, preset, scheme)
        alpha, tint = resolved["alpha"], resolved["tint"]
    return tuple(channel * (1.0 - alpha) + tint * alpha for channel in bg)


def _luminance(rgb: tuple[float, float, float]) -> float:
    def linear(value: float) -> float:
        return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4
    r, g, b = (linear(value) for value in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def _contrast(a: tuple[float, float, float], b: tuple[float, float, float]) -> float:
    la, lb = _luminance(a), _luminance(b)
    lighter, darker = max(la, lb), min(la, lb)
    return (lighter + 0.05) / (darker + 0.05)


def score(manifest_path: Path, write: bool = False) -> int:
    from PIL import Image

    data = presets.load()
    manifest = json.loads(manifest_path.read_text())
    failures: list[str] = []
    rows = []
    for case in manifest["cases"]:
        label = case["label"]
        image_path = REPO / case["capture"] if case.get("capture") else None
        query_path = REPO / case["query"] if case.get("query") else None
        row = dict(case)
        row.update({"present": bool(image_path and image_path.exists()),
                    "fit_error": None, "chroma": None,
                    "contrast": {}, "fallback": case.get("variant") != "sampled"})
        if not image_path or not image_path.exists():
            failures.append(f"{label}: missing capture")
            rows.append(row)
            continue
        query = query_path.read_text() if query_path and query_path.exists() else ""
        window = _find_app_window(query, case["app"])
        if window is None:
            failures.append(f"{label}: no CSD app window in the query dump")
            rows.append(row)
            continue
        image = Image.open(image_path)
        # A point outside the window gives the actual backdrop, independent of
        # the wallpaper generator's nominal colour.
        bx = window["x"] - int(40 * window["scale"])
        if bx < 0:
            bx = window["x"] + window["w"] + int(40 * window["scale"])
        by = window["y"] + window["h"] // 2
        bg = _mean_rgb(image, (bx, by, bx + 4, by + 4))
        row["backdrop"] = [round(value, 6) for value in bg]
        for surface in ("sidebar", "toolbar"):
            rect = _sample(window, case["app"], surface)
            actual = _mean_rgb(image, rect)
            expected = _expected(case["app"], case["scheme"], surface,
                                 case["variant"], data, bg)
            error = max(abs(a - e) for a, e in zip(actual, expected))
            chroma = max(actual) - min(actual)
            contrast = _contrast(actual, TEXT[case["scheme"]]["primary"])
            secondary = _contrast(actual, TEXT[case["scheme"]]["secondary"])
            row[surface] = {
                "rect": list(rect),
                "actual": [round(value, 6) for value in actual],
                "expected": [round(value, 6) for value in expected],
                "fit_error": round(error, 6),
                "chroma_spread": round(chroma, 6),
                "contrast_primary": round(contrast, 3),
                "contrast_secondary": round(secondary, 3),
            }
            row["fit_error"] = max(row["fit_error"] or 0.0, error)
            row["chroma"] = max(row["chroma"] or 0.0, chroma)
            row["contrast"][surface] = {
                "primary": round(contrast, 3),
                "secondary": round(secondary, 3),
            }
            tolerance = 0.035 if case["variant"] == "sampled" else 0.025
            if error > tolerance:
                failures.append(
                    f"{label} {surface}: fit error {error:.4f} > {tolerance}")
            if contrast < 4.5:
                failures.append(
                    f"{label} {surface}: primary contrast {contrast:.2f} < 4.5")
            if secondary < 3.0:
                failures.append(
                    f"{label} {surface}: secondary contrast {secondary:.2f} < 3.0")
        # Main content and unclassified chrome stay opaque. The detail sheet
        # and (in Files) the path bar must match their semantic token exactly
        # over both neutral and saturated backdrops.
        opaque_surfaces = [("detail", "surface")]
        if case["app"] == "files":
            opaque_surfaces.append(("pathbar", "chrome"))
        for name, token in opaque_surfaces:
            rect = _sample_opaque(window, case["app"], name)
            actual = _mean_rgb(image, rect)
            expected = OPAQUE[token][case["scheme"]]
            error = max(abs(a - e) for a, e in zip(actual, expected))
            row[name] = {
                "rect": list(rect),
                "actual": [round(value, 6) for value in actual],
                "expected": [round(value, 6) for value in expected],
                "fit_error": round(error, 6),
            }
            row["fit_error"] = max(row["fit_error"] or 0.0, error)
            if error > 0.02:
                failures.append(
                    f"{label} {name}: opaque fit error {error:.4f} > 0.02")
        rows.append(row)

    summary = {
        "cases": len(rows),
        "present": sum(1 for row in rows if row["present"]),
        "max_fit_error": max((row["fit_error"] or 0.0) for row in rows) if rows else 0.0,
        "max_chroma_spread": max((row["chroma"] or 0.0) for row in rows) if rows else 0.0,
        "failures": failures,
    }
    report = dict(manifest)
    report["scoring"] = summary
    report["rows"] = rows
    report["chroma"] = {
        "transport": "scalar-neutral-tint",
        "unresolved": True,
        "note": "The measured sidebar chroma pickup is not carried by the "
                "alpha/tint preset. The capture rows report the rendered "
                "channel spread; a future token/channel transport is required "
                "to reproduce the pickup itself.",
    }
    if write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2) + "\n")
        lines = [
            "# T206 — first-party Settings/Files chrome",
            "",
            f"Scored {summary['cases']} cases; {summary['present']} captures present.",
            f"Max fit error {summary['max_fit_error']:.4f}; "
            f"max rendered chroma spread {summary['max_chroma_spread']:.4f}.",
            "",
            "The presets store scalar alpha/tint. The reference's small sidebar "
            "chroma pickup is not carried by that transport; the chroma column "
            "is evidence of the rendered neutral result, not a claim that the "
            "pickup was reproduced.",
            "",
            "| Case | App | Scheme | Variant | Backdrop | Fit error | Chroma | Contrast P/S |",
            "|---|---|---|---|---|---|---|---|",
        ]
        for row in rows:
            contrast = row["contrast"].get("sidebar", {})
            lines.append(
                f"| {row['label']} | {row['app']} | {row['scheme']} | "
                f"{row['variant']} | {row.get('wallpaper', '')} | "
                f"{row['fit_error'] if row['fit_error'] is not None else '—'} | "
                f"{row['chroma'] if row['chroma'] is not None else '—'} | "
                f"{contrast.get('primary', '—')}/{contrast.get('secondary', '—')} |")
        if failures:
            lines.extend(["", "## Failures", ""] + [f"- {item}" for item in failures])
        else:
            lines.extend(["", "Verdict: PASS"])
        MD.write_text("\n".join(lines) + "\n")
    for failure in failures:
        print(f"FAIL  {failure}", file=sys.stderr)
    print(json.dumps(summary, indent=2))
    return 0 if not failures else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="validate presets, QML consumers and fallbacks")
    sub = parser.add_subparsers(dest="command")
    wall = sub.add_parser("wallpaper", help="write a deterministic backdrop")
    wall.add_argument("--kind", required=True, choices=("neutral", "saturated"))
    wall.add_argument("--out", required=True, type=Path)
    wall.add_argument("--width", type=int, default=1920)
    wall.add_argument("--height", type=int, default=1200)
    score_parser = sub.add_parser("score", help="score a capture manifest")
    score_parser.add_argument("manifest", type=Path)
    score_parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)

    data = presets.load()
    if args.check:
        errors, report = check(data)
        if errors:
            for error in errors:
                print(f"FAIL  {error}", file=sys.stderr)
            print(f"{len(errors)} first-party chrome checks failed", file=sys.stderr)
            return 1
        print("first-party chrome: presets, QML consumers and fallbacks pass")
        return 0
    if args.command == "wallpaper":
        wallpaper(args.kind, args.out, args.width, args.height)
        return 0
    if args.command == "score":
        return score(args.manifest, args.write)
    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
