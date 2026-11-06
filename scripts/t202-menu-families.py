#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T202 menu-family fixture and acceptance check.

The menu families share the generated glass presets: the attached menu-bar
dropdown, detached context/tray menus and the status menus. This script pins
the values the QML and compositor consumers resolve:

* Light attached File alpha 0.552 (target 0.55, ±0.03), blur 10 pt (±1),
  shadow extent 20 pt (±3) and bottom corner 12.5 pt.
* The measured light slider anchors on the attached preset, and the documented
  relative adjustment on the light detached/status presets (their default fits
  are preserved).
* Dark attached is calibrated against the valid dark mid/black context/status
  samples; the contaminated white row stays open for recapture.
* App-owned menus resolve the opaque local fallback; trusted shell menus use
  the sampled preset; Reduce Transparency is always solid.
* Menu close motion is 183 ms with a 0.95 terminal scale, collapsing under
  Reduce Motion.

``--check`` is part of ``make gates``. ``--write`` records the evidence JSON and
markdown report under ``docs/captures/``. Live captures from
``make menu-families-capture`` are listed when present; the optical fit rows
are model checks because the nested capture has no controlled backdrop.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

import material_presets as presets  # noqa: E402

OUT = REPO / "docs" / "captures" / "t202-menu-families.json"
MD = REPO / "docs" / "captures" / "t202-menu-families.md"
CAPTURES = REPO / "docs" / "captures"

TARGET_LIGHT_ALPHA = 0.55
TARGET_LIGHT_BLUR = 10.0
TARGET_SHADOW_EXTENT = 20.0
TARGET_BOTTOM_CORNER = 12.5
TARGET_CLOSE_MS = 183
TARGET_CLOSE_SCALE = 0.95
DARK_SAMPLE_ALPHA = 0.66
DARK_SAMPLE_TINT = 0.26


def output(bg: float, alpha: float, tint: float) -> float:
    return bg * (1.0 - alpha) + tint * alpha


def evaluate(data: dict) -> tuple[list[str], dict]:
    errors: list[str] = []
    report: dict = {}

    def check(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    attached_light = presets.resolve(
        data, "menuAttached", scheme="light", rollout="replacement"
    )
    detached_light = presets.resolve(
        data, "menuDetached", scheme="light", rollout="replacement"
    )
    status_light = presets.resolve(
        data, "statusMenu", scheme="light", rollout="replacement"
    )
    attached_dark = presets.resolve(
        data, "menuAttached", scheme="dark", rollout="replacement"
    )
    detached_dark = presets.resolve(
        data, "menuDetached", scheme="dark", rollout="replacement"
    )
    status_dark = presets.resolve(
        data, "statusMenu", scheme="dark", rollout="replacement"
    )

    components = data["component"]
    # The elevation token is a number in primitive.elevation.
    shadow_extent = float(data["primitive"]["elevation"][
        components["popup"]["shadowBlur"]["$ref"].split(".")[-1]
    ])
    bottom_corner = float(components["contextMenu"]["attachedRadius"])
    close_scale = float(components["contextMenu"]["closeScale"])
    close_ms = int(data["motion"]["menuClose"]["duration"])

    report["presets"] = {
        "attached": "menuAttached",
        "detached": "menuDetached",
        "status": "statusMenu",
        "submenu": "menuDetached",
    }
    report["light_attached"] = {
        "alpha": round(attached_light["alpha"], 6),
        "target_alpha": TARGET_LIGHT_ALPHA,
        "alpha_delta": round(attached_light["alpha"] - TARGET_LIGHT_ALPHA, 6),
        "tint": round(attached_light["tint"], 6),
        "blur": attached_light["blur"],
        "target_blur": TARGET_LIGHT_BLUR,
        "blur_delta": attached_light["blur"] - TARGET_LIGHT_BLUR,
        "shadow_extent": shadow_extent,
        "target_shadow_extent": TARGET_SHADOW_EXTENT,
        "shadow_delta": shadow_extent - TARGET_SHADOW_EXTENT,
        "bottom_corner": bottom_corner,
        "target_bottom_corner": TARGET_BOTTOM_CORNER,
        "render_mode": attached_light["renderMode"],
        "confidence": attached_light["confidence"],
    }
    check(abs(attached_light["alpha"] - TARGET_LIGHT_ALPHA) <= 0.03,
          "light attached alpha is outside the ±0.03 File tolerance")
    check(abs(attached_light["blur"] - TARGET_LIGHT_BLUR) <= 1.0,
          "light attached blur is outside the ±1 pt File tolerance")
    check(abs(shadow_extent - TARGET_SHADOW_EXTENT) <= 3.0,
          "menu shadow extent is outside the ±3 pt File tolerance")
    check(abs(bottom_corner - TARGET_BOTTOM_CORNER) <= 0.5,
          "attached bottom corner is not the measured 12.5 pt")
    check(attached_light["renderMode"] == "sampled-glass",
          "light attached menu is sampled glass")

    anchors = data["glass"]["slider"]["lightAnchors"]
    slider_rows = {}
    for name, position in (("low", 0.0), ("mid", anchors["mid"]["position"]),
                           ("high", 1.0)):
        resolved = presets.resolve(data, "menuAttached", scheme="light",
                                   slider=position, rollout="replacement")
        expected = anchors[name]
        slider_rows[name] = {
            "position": position,
            "alpha": round(resolved["alpha"], 6),
            "tint": round(resolved["tint"], 6),
            "expected_alpha": expected["alpha"],
            "expected_tint": expected["tint"],
        }
        check(abs(resolved["alpha"] - expected["alpha"]) <= 1e-6,
              f"attached slider alpha at {position} moved off the measured anchor")
        check(abs(resolved["tint"] - expected["tint"]) <= 1e-6,
              f"attached slider tint at {position} moved off the measured anchor")
    report["light_slider"] = slider_rows

    relative_rows = {}
    for name, default in (("menuDetached", detached_light),
                          ("statusMenu", status_light)):
        low = presets.resolve(data, name, scheme="light", slider=0.0,
                              rollout="replacement")
        high = presets.resolve(data, name, scheme="light", slider=1.0,
                               rollout="replacement")
        relative_rows[name] = {
            "default_alpha": round(default["alpha"], 6),
            "low_alpha": round(low["alpha"], 6),
            "high_alpha": round(high["alpha"], 6),
            "default_tint": round(default["tint"], 6),
            "low_tint": round(low["tint"], 6),
            "high_tint": round(high["tint"], 6),
        }
        check(low["alpha"] > default["alpha"] > high["alpha"],
              f"{name}: relative light alpha does not follow the File curve")
        check(low["tint"] < default["tint"] < high["tint"],
              f"{name}: relative light tint does not follow the File curve")
        check(low["sliderApplied"] and high["sliderApplied"],
              f"{name}: relative slider response is not recorded")
        dark_low = presets.resolve(data, name, scheme="dark", slider=0.0,
                                   rollout="replacement")
        dark_high = presets.resolve(data, name, scheme="dark", slider=1.0,
                                    rollout="replacement")
        check(not dark_low["sliderApplied"] and not dark_high["sliderApplied"],
              f"{name}: dark must not follow the light slider")
    report["light_relative"] = relative_rows

    dark_rows = {}
    for label, resolved in (("attached", attached_dark),
                            ("detached", detached_dark),
                            ("status", status_dark)):
        mid = output(0.502, resolved["alpha"], resolved["tint"])
        black = output(0.0, resolved["alpha"], resolved["tint"])
        reference_mid = output(0.502, DARK_SAMPLE_ALPHA, DARK_SAMPLE_TINT)
        reference_black = output(0.0, DARK_SAMPLE_ALPHA, DARK_SAMPLE_TINT)
        dark_rows[label] = {
            "alpha": round(resolved["alpha"], 6),
            "tint": round(resolved["tint"], 6),
            "mid_output": round(mid, 6),
            "black_output": round(black, 6),
            "reference_mid": round(reference_mid, 6),
            "reference_black": round(reference_black, 6),
            "mid_delta": round(mid - reference_mid, 6),
            "black_delta": round(black - reference_black, 6),
            "confidence": resolved["confidence"],
            "white_row": "open",
        }
    check((attached_dark["alpha"], attached_dark["tint"])
          != (detached_dark["alpha"], detached_dark["tint"]),
          "dark attached and detached presets collapsed to one value")
    check(abs(dark_rows["attached"]["mid_delta"]) <= 0.06
          and abs(dark_rows["attached"]["black_delta"]) <= 0.06,
          "dark attached is not within the valid mid/black sample tolerance")
    report["dark_menus"] = dark_rows
    report["dark_file_decision"] = {
        "rejected_crop": "captures/coverage/dark_menu_file__neutral_white.png "
                         "(pixel-identical to the Light File crop)",
        "valid_rows": ["mid", "black"],
        "white_row": "open",
        "action": "calibrated the attached baseline against the valid dark "
                  "context/status family; a controlled white-row recapture "
                  "remains open",
    }

    trusted = presets.resolve(data, "menuDetached", scheme="light",
                              rollout="replacement", backend="gpu-sampled")
    local = presets.resolve(data, "menuDetached", scheme="light",
                            rollout="replacement", backend="software")
    reduced = presets.resolve(data, "menuAttached", scheme="light",
                              rollout="replacement", reduce_transparency=True)
    report["fallback"] = {
        "trusted": {
            "render_mode": trusted["renderMode"],
            "alpha": round(trusted["alpha"], 6),
            "blur": trusted["blur"],
        },
        "app_local": {
            "render_mode": local["renderMode"],
            "alpha": round(local["alpha"], 6),
            "blur": local["blur"],
        },
        "reduce_transparency": {
            "render_mode": reduced["renderMode"],
            "alpha": round(reduced["alpha"], 6),
            "blur": reduced["blur"],
        },
    }
    check(trusted["renderMode"] == "sampled-glass",
          "trusted detached menus must sample")
    check(local["renderMode"] == "software-fallback" and local["alpha"] >= 0.9,
          "app-owned menus must use the opaque local fallback")
    check(reduced["renderMode"] == "solid" and reduced["blur"] == 0.0,
          "Reduce Transparency must be a solid plate")

    report["motion"] = {
        "close_ms": close_ms,
        "target_close_ms": TARGET_CLOSE_MS,
        "terminal_scale": close_scale,
        "target_terminal_scale": TARGET_CLOSE_SCALE,
        "reduced_motion_ms": int(data["motion"]["menuClose"].get("reducedDuration", 0)),
        "anchor_translation": "toward the anchor point; interaction closes immediately",
    }
    check(close_ms == TARGET_CLOSE_MS,
          "menu close duration is not the measured 183 ms")
    check(abs(close_scale - TARGET_CLOSE_SCALE) <= 1e-6,
          "menu terminal scale is not the measured 0.95")
    check(int(data["motion"]["menuClose"].get("reducedDuration", 0)) == 0,
          "Reduce Motion must collapse the menu close duration")

    captures = []
    for path in sorted(CAPTURES.glob("t202-*.png")):
        captures.append(str(path.relative_to(REPO)))
    report["captures"] = captures
    report["capture_note"] = (
        "Live captures are produced by `make menu-families-capture`; optical "
        "fit rows above are model checks because the nested desktop has no "
        "controlled backdrop."
    )
    return errors, report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    data = presets.load()
    errors, report = evaluate(data)
    if errors:
        for error in errors:
            print(f"FAIL  {error}")
        return 1
    if args.write:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2) + "\n")
        light = report["light_attached"]
        MD.write_text(
            "# T202 — menu families\n\n"
            "Model and capture evidence for the attached/detached/status menu "
            "presets. Live stills are produced by `make menu-families-capture`.\n\n"
            "## Light attached File\n\n"
            "| metric | resolved | target | delta |\n|---|---|---|---|\n"
            f"| alpha | {light['alpha']:.3f} | {light['target_alpha']:.3f} | "
            f"{light['alpha_delta']:+.3f} |\n"
            f"| blur | {light['blur']:.1f} pt | {light['target_blur']:.1f} pt | "
            f"{light['blur_delta']:+.1f} pt |\n"
            f"| shadow extent | {light['shadow_extent']:.0f} pt | "
            f"{light['target_shadow_extent']:.0f} pt | "
            f"{light['shadow_delta']:+.0f} pt |\n"
            f"| bottom corner | {light['bottom_corner']:.1f} pt | "
            f"{light['target_bottom_corner']:.1f} pt | — |\n\n"
            "## Dark File decision\n\n"
            f"- Rejected crop: {report['dark_file_decision']['rejected_crop']}\n"
            f"- Valid rows: {', '.join(report['dark_file_decision']['valid_rows'])}\n"
            f"- White row: {report['dark_file_decision']['white_row']}\n"
            f"- {report['dark_file_decision']['action']}\n\n"
            "## Fallback\n\n"
            f"- Trusted shell menu: {report['fallback']['trusted']['render_mode']}\n"
            f"- App-owned menu: {report['fallback']['app_local']['render_mode']} "
            f"(alpha {report['fallback']['app_local']['alpha']:.3f})\n"
            f"- Reduce Transparency: {report['fallback']['reduce_transparency']['render_mode']}\n\n"
            "## Motion\n\n"
            f"- Close {report['motion']['close_ms']} ms, terminal scale "
            f"{report['motion']['terminal_scale']:.2f}; Reduce Motion collapses it.\n\n"
            "## Captures\n\n"
            + ("\n".join(f"- `{path}`" for path in report["captures"]) or
               "- None yet; run `make menu-families-capture` on a host session.")
            + "\n"
        )
        print(f"wrote {OUT.relative_to(REPO)} and {MD.relative_to(REPO)}")
    else:
        print("menu families: model checks pass")
        if args.check:
            print(json.dumps(report["light_attached"], indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
