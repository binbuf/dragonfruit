#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T199 Dock motion calibration from the controlled reference checkout.

The reference analysis lives outside this repository:

  analysis/magnification_sweep.json
  analysis/temporal_response.json
  docs/material-schema.json

Commands:

  fit      fit the raised-cosine spatial curve and print the calibration
  check    compare design-system/tokens/tokens.json against the reference
  report   write docs/captures/t199-dock-motion.md

The fit uses the sweep's Google Chrome tile (the same probe as the reference
``analysis/measure.py``): the observed tile width divided by the rest tile
width, against the absolute cursor distance in capture pixels.  The capture is
2x, so the influence radius is halved to logical points.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TOKENS = REPO / "design-system" / "tokens" / "tokens.json"
DEFAULT_REFERENCE = REPO.parent / "macos-liquid-glass"
OUT = REPO / "docs" / "captures" / "t199-dock-motion.md"

# The reference's two calibration anchors (REPORT.md §3.2 and §4.6).
DIAGNOSTIC_PEAK = 2.047
DEFAULT_PEAK = 73.0 / 60.0
TARGET_RADIUS_PT = 125.1
TARGET_DWELL_MS = 1400
TARGET_MAGNIFY_MS = 135
TARGET_GENIE_MS = 450
TARGET_LAUNCH_LIFT_PT = 15


def raised_cosine(distance_px: float, amplitude: float, radius_px: float) -> float:
    u = min(distance_px / radius_px, 1.0)
    return 1.0 + amplitude * 0.5 * (1.0 + math.cos(math.pi * u))


def load_sweep(reference_root: Path):
    path = reference_root / "analysis" / "magnification_sweep.json"
    data = json.loads(path.read_text())
    rows = []
    for row in data["rows"]:
        chrome = next((tile for tile in row["layout"] if tile["name"] == "Google Chrome"), None)
        if chrome is not None:
            rows.append((abs(float(row["delta"])), chrome["w"] / float(data["tilesize"])))
    if not rows:
        raise SystemExit(f"{path}: no Google Chrome samples")
    return data, rows


def fit_curve(rows):
    def sse(amplitude, radius):
        return sum(
            (raised_cosine(distance, amplitude, radius) - scale) ** 2
            for distance, scale in rows
        )

    best = (float("inf"), 1.0, 240.0)
    for amplitude_step in range(500, 1501, 5):
        amplitude = amplitude_step / 1000.0
        for radius in range(150, 351):
            error = sse(amplitude, radius)
            if error < best[0]:
                best = (error, amplitude, float(radius))
    error, amplitude, radius = best
    # Coarse-to-fine local refinement; pure stdlib so the script runs on a
    # minimal host (the reference campaign used SciPy).
    for step in (0.005, 0.0005, 0.00005):
        center_a, center_r = amplitude, radius
        local = (error, center_a, center_r)
        for ai in range(-10, 11):
            a = center_a + ai * step
            for ri in range(-10, 11):
                r = center_r + ri * (step * 200.0)
                if r <= 0:
                    continue
                e = sse(a, r)
                if e < local[0]:
                    local = (e, a, r)
        error, amplitude, radius = local
    return {
        "amplitude": amplitude,
        "radius_px": radius,
        "radius_pt": radius / 2.0,
        "rmse": math.sqrt(error / len(rows)),
        "max_scale": max(scale for _, scale in rows),
        "samples": len(rows),
    }


def read_tokens():
    return json.loads(TOKENS.read_text())


def check(tokens, fit):
    dock = tokens["component"]["dock"]
    motion = tokens["motion"]
    tooltip = tokens["component"]["tooltip"]
    checks = []

    def check_item(name, ok, detail):
        checks.append({"name": name, "ok": bool(ok), "detail": detail})

    check_item(
        "influence radius",
        abs(dock["magnifyInfluence"] - fit["radius_pt"]) <= 5.0,
        f'{dock["magnifyInfluence"]:.1f} pt vs fit {fit["radius_pt"]:.1f} pt (±5)',
    )
    check_item(
        "diagnostic peak",
        abs(dock["magnifyPeakMax"] - DIAGNOSTIC_PEAK) / DIAGNOSTIC_PEAK <= 0.03,
        f'{dock["magnifyPeakMax"]:.3f}x vs observed {DIAGNOSTIC_PEAK:.3f}x (±3 %)',
    )
    check_item(
        "default peak",
        abs(dock["magnifyPeak"] - DEFAULT_PEAK) / DEFAULT_PEAK <= 0.03,
        f'{dock["magnifyPeak"]:.3f}x vs 73/60 {DEFAULT_PEAK:.3f}x (±3 %)',
    )
    hover = motion["dockHover"]
    smoothstep = (
        abs(hover["curve"][0] - 1.0 / 3.0) < 1e-6
        and abs(hover["curve"][1]) < 1e-9
        and abs(hover["curve"][2] - 2.0 / 3.0) < 1e-6
        and abs(hover["curve"][3] - 1.0) < 1e-9
    )
    check_item(
        "enter/exit envelope",
        125 <= hover["duration"] <= 145 and smoothstep,
        f'{hover["duration"]} ms, curve {hover["curve"][:4]} (smoothstep control points)',
    )
    check_item(
        "tooltip dwell",
        abs(tooltip["dwell"] - TARGET_DWELL_MS) <= 100,
        f'{tooltip["dwell"]} ms vs {TARGET_DWELL_MS} ms',
    )
    check_item(
        "tooltip offset",
        tooltip["offset"] == 8,
        f'{tooltip["offset"]} pt above the plate',
    )
    check_item(
        "genie timing",
        TARGET_GENIE_MS <= motion["windowGenie"]["duration"] <= 500,
        f'{motion["windowGenie"]["duration"]} ms vs 450-500 ms',
    )
    check_item(
        "launch lift",
        abs(dock["launchLift"] - TARGET_LAUNCH_LIFT_PT) <= 1,
        f'{dock["launchLift"]} pt vs {TARGET_LAUNCH_LIFT_PT} pt',
    )
    return checks


def cmd_fit(args):
    _, rows = load_sweep(Path(args.reference_root))
    fit = fit_curve(rows)
    print(json.dumps(fit, indent=2, sort_keys=True))
    return 0


def cmd_check(args):
    _, rows = load_sweep(Path(args.reference_root))
    fit = fit_curve(rows)
    checks = check(read_tokens(), fit)
    for item in checks:
        print(f'{"PASS" if item["ok"] else "FAIL"}  {item["name"]}: {item["detail"]}')
    return 0 if all(item["ok"] for item in checks) else 1


def cmd_report(args):
    _, rows = load_sweep(Path(args.reference_root))
    fit = fit_curve(rows)
    checks = check(read_tokens(), fit)
    lines = [
        "# T199 — Dock motion calibration",
        "",
        "Generated by `python3 scripts/t199-dock-motion.py report` from the",
        "controlled reference checkout (`macos-liquid-glass`).",
        "",
        "## Spatial fit",
        "",
        "| Metric | Value |",
        "|---|---:|",
        f'| Samples (Google Chrome sweep) | {fit["samples"]} |',
        f'| Fitted amplitude A | {fit["amplitude"]:.6f} |',
        f'| Fitted influence radius | {fit["radius_pt"]:.3f} pt ({fit["radius_px"]:.3f} capture px) |',
        f'| Fitted RMSE | {fit["rmse"]:.5f} |',
        f'| Observed diagnostic peak | {fit["max_scale"]:.4f}x |',
        f'| Fitted peak 1 + A | {1 + fit["amplitude"]:.4f}x |',
        "",
        "Model: `S(d) = 1 + A * 0.5 * (1 + cos(pi * d / R))`.",
        "",
        "## Token checks",
        "",
        "| Check | Result | Detail |",
        "|---|---|---|",
    ]
    for item in checks:
        lines.append(
            f'| {item["name"]} | {"PASS" if item["ok"] else "FAIL"} | {item["detail"]} |'
        )
    lines += [
        "",
        "## Capture limitations",
        "",
        "- The spatial/temporal fit comes from the reference checkout's 2x",
        "  capture; Dragonfruit's live nested sweep remains the visual check.",
        "- `make dock-magnify-sweep-capture` needs a host Wayland session,",
        "  `spectacle`, Pillow and the built tree. When that host is absent the",
        "  QML regression tests are the executable calibration evidence.",
        "- Launch lift is only partially measured in the reference (one 15 pt /",
        "  ~0.3 s lift before the app window covers the Dock); the existing",
        "  multi-hop shape is deliberately not re-invented.",
        "",
    ]
    OUT.write_text("\n".join(lines))
    print(f"wrote {OUT}")
    return 0 if all(item["ok"] for item in checks) else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("fit", "check", "report"))
    parser.add_argument("--reference-root", default=str(DEFAULT_REFERENCE))
    args = parser.parse_args()
    return {"fit": cmd_fit, "check": cmd_check, "report": cmd_report}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
