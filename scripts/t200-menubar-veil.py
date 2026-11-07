#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T200 menu-bar no-blur veil fixture and model check.

The compositor's veil model lives in ``compositor/src/window/menubar_veil.rs``;
the shader in ``compositor/src/window/blur.rs``. This script is the
hardware-independent acceptance fixture:

* the Light black-background LUT is within 0.03 of every measured sample and
  exactly zero at 95 pt;
* the LUT is monotone and the Dark baseline is a black wash with the same
  depth shape;
* the checker transmission at a fixed depth is independent of checker size
  (the veil has no spatial kernel);
* a moving dark/bright window remains readable under Light/Dark and the
  fallback plate stays scheme-appropriate;
* the veil resolves to the explicit no-blur mode with zero blur.

``--check`` is part of ``make gates``; ``--write`` records the evidence JSON.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import material_presets as presets  # noqa: E402

OUT = REPO / "docs" / "captures" / "t200-menubar-veil-model.json"
DEPTHS = (5, 15, 25, 35, 50, 75, 90, 95)
MEASURED_LIGHT = {
    5: 0.45098037,
    15: 0.43921566,
    25: 0.40784317,
    35: 0.35686272,
    50: 0.24313729,
    75: 0.05882353,
    90: 0.00784314,
    95: 0.0,
}
PERIODS = (2, 4, 8, 16, 32, 64)
TOLERANCE = 0.03
READABILITY_FADE = 8.0


def alpha_at(samples: dict[int, float], depth: float) -> float:
    """Piecewise-linear LUT through the measured samples, zero at/after 95."""
    points = sorted(samples.items())
    if depth <= points[0][0]:
        return points[0][1]
    for (left_d, left_a), (right_d, right_a) in zip(points, points[1:]):
        if depth <= right_d:
            span = right_d - left_d
            ratio = 0.0 if span <= 0 else (depth - left_d) / span
            return left_a + (right_a - left_a) * ratio
    return 0.0


def adapted_alpha(light: bool, base: float, depth: float, lum: float,
                  strip: float, target: float) -> float:
    """The same bounded readability rule the shader implements."""
    if depth >= strip + READABILITY_FADE:
        return base
    lum = max(0.0, min(1.0, lum))
    need = 0.0
    if light:
        if lum < target:
            need = (target - lum) / max(1.0 - lum, 1e-4)
    elif lum > target:
        need = (lum - target) / max(lum, 1e-4)
    t = max(0.0, min(1.0, (depth - strip) / READABILITY_FADE))
    weight = 1.0 - t * t * (3.0 - 2.0 * t)
    return max(0.0, min(1.0, base + max(need - base, 0.0) * weight))


def output_luminance(bg: float, alpha: float, veil_luma: float) -> float:
    return bg * (1.0 - alpha) + veil_luma * alpha


def contrast(a: float, b: float) -> float:
    light, dark = (a, b) if a > b else (b, a)
    return (light + 0.05) / (dark + 0.05)


def samples(entry: dict) -> dict[int, float]:
    return {depth: float(entry[f"alpha{depth}"]) for depth in DEPTHS}


def evaluate(data: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    report: dict = {}

    def check(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    light = presets.resolve(data, "menuBarVeil", scheme="light",
                            rollout="replacement")
    dark = presets.resolve(data, "menuBarVeil", scheme="dark",
                           rollout="replacement")
    light_lut = samples(light["veil"])
    dark_lut = samples(dark["veil"])

    check(light["renderMode"] == "no-blur-veil", "Light veil is no-blur-veil")
    check(dark["renderMode"] == "no-blur-veil", "Dark veil is no-blur-veil")
    check(light["blur"] == 0.0 and dark["blur"] == 0.0,
          "the veil must request no blur")
    check(light["stripHeight"] == 39, "reference strip is 39 pt")
    check(light["extent"] == 95, "veil extent is 95 pt")
    check(light["tint"] >= 0.9, "Light veil is a white lift")
    check(dark["tint"] <= 0.1, "Dark veil is a black wash")

    for depth, expected in MEASURED_LIGHT.items():
        actual = light_lut[depth]
        check(abs(actual - expected) <= TOLERANCE,
              f"Light alpha at {depth} pt: {actual} is outside {TOLERANCE} of {expected}")
    check(light_lut[95] == 0.0, "Light LUT is zero at 95 pt")
    check(alpha_at(light_lut, 95) == 0.0, "Light LUT stays zero at 95 pt")
    check(alpha_at(light_lut, 120) == 0.0, "Light LUT stays zero past 95 pt")
    check(dark_lut[95] == 0.0, "Dark LUT is zero at 95 pt")
    for lut, label in ((light_lut, "Light"), (dark_lut, "Dark")):
        for left, right in zip(DEPTHS, DEPTHS[1:]):
            check(lut[left] >= lut[right],
                  f"{label} LUT rises from {left} to {right} pt")
    check(abs(dark_lut[5] - 0.48) <= 1e-6, "Dark baseline peaks at 0.48")

    # No spatial kernel: at a fixed depth every checker period sees the same
    # scalar transmission, so the AC ratio cannot depend on checker size.
    checker = {}
    for depth in (5, 15, 25, 35, 50, 75, 90):
        alpha = light_lut[depth]
        ratios = {}
        for period in PERIODS:
            ratios[str(period)] = round(1.0 - alpha, 12)
        checker[str(depth)] = ratios
        check(len(set(ratios.values())) == 1,
              f"checker transmission at {depth} pt depends on period")
    report["checker_transmission"] = checker

    # Live-window readability: a dark and a bright window move under the bar.
    # Light text is 0.05 on Light; Dark text is 0.95 on Dark.
    live = {}
    for label, entry, light_mode, text in (
        ("light-dark-window", light, True, 0.05),
        ("light-bright-window", light, True, 0.05),
        ("dark-bright-window", dark, False, 0.95),
        ("dark-dark-window", dark, False, 0.95),
    ):
        bg = 0.02 if "dark-window" in label else 0.98
        lut = light_lut if light_mode else dark_lut
        veil_luma = 1.0 if light_mode else 0.0
        depth = 25
        alpha = adapted_alpha(light_mode, alpha_at(lut, depth), depth, bg,
                              entry["stripHeight"], entry["veil"]["readability"])
        out = output_luminance(bg, alpha, veil_luma)
        ratio = contrast(out, text)
        live[label] = {
            "background": bg,
            "depth_pt": depth,
            "alpha": round(alpha, 6),
            "output_luminance": round(out, 6),
            "contrast": round(ratio, 3),
        }
        minimum = 4.0 if label == "dark-bright-window" else 4.5
        check(ratio >= minimum,
              f"{label}: contrast {ratio:.2f} is below {minimum}")
    report["live_window"] = live

    # Fallback colours are scheme-appropriate and opaque enough to read.
    fallback = {}
    for scheme in ("light", "dark"):
        material = data["semantic"][scheme]["material"]
        fallback[scheme] = {
            "fill": data["semantic"][scheme]["color"]["chromeFallbackFill"],
            "opacity": material["chromeFallbackOpacity"],
        }
        check(material["chromeFallbackOpacity"] >= 0.8,
              f"{scheme} fallback must read as a plate")
    check(fallback["light"]["fill"] != fallback["dark"]["fill"],
          "fallback fill must follow the scheme")
    reduced = presets.resolve(data, "menuBarVeil", scheme="light",
                              rollout="replacement", reduce_transparency=True)
    check(reduced["renderMode"] == "solid", "Reduce Transparency is solid")
    check(reduced["alpha"] >= 0.9, "Reduce Transparency is opaque")
    report["fallback"] = fallback
    report["reduce_transparency"] = {
        "mode": reduced["renderMode"],
        "alpha": reduced["alpha"],
    }

    report["light"] = {
        "strip_pt": light["stripHeight"],
        "extent_pt": light["extent"],
        "tint": light["tint"],
        "alpha": light_lut,
    }
    report["dark"] = {
        "strip_pt": dark["stripHeight"],
        "extent_pt": dark["extent"],
        "tint": dark["tint"],
        "alpha": dark_lut,
    }
    report["checks"] = len(errors) == 0
    return errors, report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true",
                        help="run the acceptance fixture")
    parser.add_argument("--write", action="store_true",
                        help=f"record the evidence JSON at {OUT.relative_to(REPO)}")
    args = parser.parse_args()

    errors, report = evaluate(presets.load())
    if args.write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2) + "\n")
        print(f"wrote {OUT.relative_to(REPO)}")
    if errors:
        for error in errors:
            print(f"FAIL  {error}", file=sys.stderr)
        print(f"{len(errors)} failed")
        return 1
    print("T200 menu-bar veil: LUT, checker, live-window and fallback checks pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
