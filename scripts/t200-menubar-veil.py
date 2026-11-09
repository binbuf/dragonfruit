#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T200 menu-bar no-blur veil fixture and model check.

The compositor's veil model lives in ``compositor/src/window/menubar_veil.rs``;
the shader in ``compositor/src/window/blur.rs``. This script is the
hardware-independent acceptance fixture:

* the Light black-background LUT is within 0.03 of every measured sample and
  exactly zero at 95 pt;
* the Light background-luminance adaptation matches the measured
  0.451/0.477/0.409/0.218/0.0 table over L=0/0.251/0.502/0.784/1.0, so the
  veil is invisible over white;
* the LUT is monotone and the Dark baseline matches the coverage captures:
  ~0.125 alpha over mid-grey and invisible over white;
* the checker transmission at a fixed depth is independent of checker size
  (the veil has no spatial kernel);
* the text/glyph foreground flips to black over a light rendered bar and
  stays light over the dark/mid Dark backgrounds;
* the fallback plate stays scheme-appropriate;
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


def adaptation(lum: float, adapt_luma: list[float],
               adapt_scale: list[float]) -> float:
    """Piecewise-linear background-luminance response through the anchors."""
    lum = max(0.0, min(1.0, lum))
    if lum <= adapt_luma[0]:
        return adapt_scale[0]
    for left_l, right_l, left_s, right_s in zip(
            adapt_luma, adapt_luma[1:], adapt_scale, adapt_scale[1:]):
        if lum <= right_l:
            span = right_l - left_l
            ratio = 0.0 if span <= 0 else (lum - left_l) / span
            return left_s + (right_s - left_s) * ratio
    return adapt_scale[-1]


def adapted_alpha(base: float, lum: float, adapt_luma: list[float],
                  adapt_scale: list[float]) -> float:
    """The same measured adaptation the shader uses."""
    return max(0.0, min(1.0, base * adaptation(lum, adapt_luma, adapt_scale)))


def output_luminance(bg: float, alpha: float, veil_luma: float) -> float:
    return bg * (1.0 - alpha) + veil_luma * alpha


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

    # Background-luminance adaptation: the fixed depth LUT alone over-veils a
    # light wallpaper. The measured Light curve is 0.451/0.477/0.409/0.218/0.0
    # over L=0/0.251/0.502/0.784/1.0 at the top of the bar.
    backgrounds = {}
    for bg, expected in ((0.0, 0.451), (0.251, 0.477), (0.502, 0.409),
                         (0.784, 0.218), (1.0, 0.0)):
        alpha = adapted_alpha(alpha_at(light_lut, 5), bg,
                              light["veil"]["adaptLuma"],
                              light["veil"]["adaptScale"])
        backgrounds[str(bg)] = {
            "alpha": round(alpha, 6),
            "measured": expected,
            "delta": round(alpha - expected, 6),
        }
        check(abs(alpha - expected) <= TOLERANCE,
              f"background {bg}: alpha {alpha} is outside {TOLERANCE} of {expected}")
    check(light["veil"]["adaptScale"][0] == 1.0,
          "Light adaptation starts at 1.0 over black")
    check(light["veil"]["adaptScale"][-1] == 0.0,
          "Light adaptation ends at 0.0 over white")
    report["background_adaptation"] = backgrounds

    # Dark-mode coverage: over mid-grey the top alpha is ~0.125; over white
    # the veil is invisible (the old fixed 0.48 baseline turned white grey).
    dark_backgrounds = {}
    for bg, expected in ((0.0, 0.0), (0.502, 0.439), (1.0, 1.0)):
        alpha = adapted_alpha(alpha_at(dark_lut, 5), bg,
                              dark["veil"]["adaptLuma"],
                              dark["veil"]["adaptScale"])
        out = output_luminance(bg, alpha, 0.0)
        dark_backgrounds[str(bg)] = {
            "alpha": round(alpha, 6),
            "output": round(out, 6),
            "measured": expected,
            "delta": round(out - expected, 6),
        }
        check(abs(out - expected) <= 0.02,
              f"Dark background {bg}: output {out} is outside 0.02 of {expected}")
    check(abs(dark["veil"]["adaptScale"][2] - 0.26026) <= 1e-6,
          "Dark adaptation keeps the coverage mid-grey fit")
    check(dark["veil"]["adaptScale"][-1] == 0.0,
          "Dark adaptation ends at 0.0 over white")
    report["dark_background_adaptation"] = dark_backgrounds

    # Wallpaper polarity selects the pair independently of app appearance.
    # Retain the measured optics above; white is preferred until very bright.
    menu_bar = data["component"]["menuBar"]
    foreground = {}
    for scheme in ("light", "dark"):
        for bg, expected in ((0.0, "light"), (0.502, "light"),
                             (0.77, "light"), (0.8, "dark"), (1.0, "dark")):
            threshold = menu_bar["foregroundFlipThresholdLight"]
            chosen = "light" if bg < threshold else "dark"
            entry = dark if chosen == "light" else light
            lut = dark_lut if chosen == "light" else light_lut
            alpha = adapted_alpha(alpha_at(lut, 5), bg,
                                  entry["veil"]["adaptLuma"], entry["veil"]["adaptScale"])
            label = f"{scheme}-{bg}"
            foreground[label] = {
                "rendered_luma": round(output_luminance(bg, alpha, entry["tint"]), 6),
                "threshold": threshold,
                "foreground": chosen,
            }
            check(chosen == expected, f"{label}: foreground {chosen} != {expected}")
    report["foreground"] = foreground

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
        "adapt_luma": light["veil"]["adaptLuma"],
        "adapt_scale": light["veil"]["adaptScale"],
    }
    report["dark"] = {
        "strip_pt": dark["stripHeight"],
        "extent_pt": dark["extent"],
        "tint": dark["tint"],
        "alpha": dark_lut,
        "adapt_luma": dark["veil"]["adaptLuma"],
        "adapt_scale": dark["veil"]["adaptScale"],
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
    print("T200 menu-bar veil: LUT, adaptation, foreground and fallback checks pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
