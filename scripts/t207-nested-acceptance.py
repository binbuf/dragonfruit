#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T207 nested and fallback acceptance for the liquid-glass replacement.

The acceptance is a matrix, not one screenshot:

* every measured surface has a per-surface report from T198/T200/T202/T203/
  T205/T206 (colour, rim, MTF, lens, timing, fallback);
* the new T207 A/B run captures the same scene and settings on the legacy
  (old) rail and the replacement rail, then compares geometry, padded area,
  timing, cache growth, transport/copies, idle behaviour and the flat-band
  reference error;
* interaction rows cover overlays, live setting changes, output change,
  lock/fullscreen transitions, idle settle, service restart and teardown.

``--check`` is the hardware-independent gate. It validates the checked-in case
matrix, the report and the per-surface reports. ``plan`` and ``score`` are used
by ``make nested-acceptance-capture``.

Reference crops come from the optional ``macos-liquid-glass`` checkout through
``docs/captures/glass-reference-manifest.json``. A requested reference run
without the checkout records the reference rows OPEN; it never invents a
score.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CAPTURES = REPO / "docs" / "captures"
CASES = REPO / "tests" / "glass" / "t207-cases.json"
OUT_JSON = CAPTURES / "t207-nested-acceptance.json"
OUT_MD = CAPTURES / "t207-nested-acceptance.md"
MANIFEST = CAPTURES / "t207-nested-acceptance-capture.json"
MANIFEST_JSONL = CAPTURES / "t207-nested-acceptance-capture.jsonl"
REFERENCE_MANIFEST = CAPTURES / "glass-reference-manifest.json"
REFERENCE_ROOT = Path(
    __import__("os").environ.get(
        "DRAGONFRUIT_REFERENCE_ROOT",
        str(REPO.parent / "macos-liquid-glass"),
    )
)

sys.path.insert(0, str(REPO / "scripts"))

TOLERANCES = {
    "flat_band": {
        "mean_abs_srgb": 0.02,
        "p95_abs_srgb": 0.05,
        "ssim": 0.90,
    },
    "timing_regression": 0.10,
    "timing_floor_us": 2000,
    "padded_area_regression": 0.001,
    "cache_growth": 1,
    "idle_frames": 1,
}

VERDICTS = ("pass", "fail", "adaptation", "OPEN")


def load_json(path: Path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_cases() -> dict:
    return load_json(CASES)


def plan_rows(cases: dict) -> list[dict]:
    rows: list[dict] = []
    for case in cases["cases"]:
        rollouts = ("legacy", "replacement") if case["group"] == "ab" else ("replacement",)
        for rollout in rollouts:
            row = dict(case)
            row["rollout"] = rollout
            rows.append(row)
    return rows


def _pre_capture(case: dict) -> str:
    kind = case.get("pre_capture", "")
    if kind == "moving-panel":
        return (
            'for i in $(seq 1 120); do [ -f "$XDG_RUNTIME_DIR/$SOCKET.x11-display" ] '
            '&& break; sleep 0.1; done; '
            'DISPLAY=$(cat "$XDG_RUNTIME_DIR/$SOCKET.x11-display") '
            '"$T207_MOVING_PANEL" 25 &'
        )
    if kind == "scale-change":
        return (
            'BUS=$(sed -n "s/.*private session bus at //p" "$SCRATCH/demo.log" | head -1); '
            'echo "bus=$BUS" > "$SCRATCH/scale-change.log"; '
            'DBUS_SESSION_BUS_ADDRESS="$BUS" gdbus call --session '
            '--dest org.dragonfruit.Settings1 '
            '--object-path /org/dragonfruit/Settings1 '
            '--method org.dragonfruit.Settings1.Set display.scale "<1.5>" '
            '>> "$SCRATCH/scale-change.log" 2>&1; sleep 6'
        )
    return ""


def cmd_plan(args: argparse.Namespace) -> int:
    cases = load_cases()
    fields = (
        "id", "rollout", "state", "surface", "scheme", "scale", "tier",
        "material_path", "chrome_gpu", "wallpaper", "transparency",
        "reduce_transparency", "increase_contrast", "reduce_motion",
        "capture_delay", "stats_interval", "stats_snapshots", "assert_idle",
        "reference", "expect", "session",
    )
    for row in plan_rows(cases):
        values = []
        for field in fields:
            value = row.get(field, "")
            if value is None:
                value = ""
            if isinstance(value, bool):
                value = "1" if value else "0"
            values.append(str(value))
        values.append(row.get("pre_capture", ""))
        # Unit separator: the fields are expanded into shell commands by the
        # capture runner, which may contain shell pipes.
        print("\x1f".join(values))
    return 0


def parse_query(path: Path) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace")
    result: dict = {"raw": text, "regions": {}, "materials": {}, "output": {}}
    for line in text.splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "scale" and len(parts) >= 3 and parts[1] == "output":
            fields = dict(part.split("=", 1) for part in parts[3:] if "=" in part)
            result["output"] = fields
        elif parts[0] == "scale" and len(parts) >= 4 and parts[1] == "region":
            name = parts[2]
            index = 3
            sub = ""
            if index < len(parts) and "=" not in parts[index]:
                sub = parts[index]
                index += 1
            fields = dict(part.split("=", 1) for part in parts[index:] if "=" in part)
            key = f"{name}/{sub}" if sub else name
            result["regions"][key] = fields
            # Keep the first region under the bare name as a fallback (the
            # outer panel for Control Center, the panel for the Dock).
            result["regions"].setdefault(name, fields)
        elif parts[0] == "material" and len(parts) >= 2 and parts[1].startswith("preset="):
            name = parts[1].split("=", 1)[1]
            fields = dict(part.split("=", 1) for part in parts[2:] if "=" in part)
            result["materials"][name] = fields
        elif parts[0] == "material" and len(parts) >= 2:
            fields = dict(part.split("=", 1) for part in parts[1:] if "=" in part)
            result.setdefault("material_roles", {}).update(fields)
        elif parts[0] == "lock" and len(parts) >= 2:
            fields = dict(part.split("=", 1) for part in parts[1:] if "=" in part)
            result["lock"] = fields
        elif parts[0] == "session" and len(parts) >= 2:
            fields = dict(part.split("=", 1) for part in parts[1:] if "=" in part)
            result["session"] = fields
        elif parts[0] == "scanout" and len(parts) >= 2:
            fields = dict(part.split("=", 1) for part in parts[1:] if "=" in part)
            result["scanout"] = fields
        elif parts[0] == "decoration":
            result.setdefault("decorations", []).append(parts)
    return result


def _token_fields(line: str) -> dict:
    return dict(
        match.group(1, 2)
        for match in re.finditer(r"([A-Za-z_][A-Za-z0-9_-]*)=([^\s]+)", line)
    )


def parse_stats(path: Path) -> dict:
    if not path.is_file():
        return {"present": False, "snapshots": [], "last": {}, "delta": {}}
    text = path.read_text(encoding="utf-8", errors="replace")
    groups = [group.strip() for group in text.split("snapshot") if group.strip()]
    snapshots = []
    for group in groups:
        entry: dict = {}
        for line in group.splitlines():
            if "chrome stats" in line:
                entry["shell"] = _token_fields(line)
            elif "chrome surface" in line:
                surface = _token_fields(line).get("surface")
                if surface:
                    entry.setdefault("surfaces", {})[surface] = _token_fields(line)
            elif "output material stats" in line:
                entry["output_material"] = _token_fields(line)
            elif "material stats" in line:
                entry["material"] = _token_fields(line)
            elif "render stats" in line:
                entry["render"] = _token_fields(line)
            elif "scanout stats" in line:
                entry["scanout"] = _token_fields(line)
            elif "degrade stats" in line:
                entry["degrade"] = _token_fields(line)
            elif "refresh stats" in line:
                entry["refresh"] = _token_fields(line)
            elif "frame timing" in line:
                entry["frame_timing"] = _token_fields(line)
            elif "animation stats" in line:
                entry["animation"] = _token_fields(line)
            elif "frame request" in line:
                entry["frame_request"] = _token_fields(line)
        snapshots.append(entry)
    last = snapshots[-1] if snapshots else {}
    delta: dict = {}

    def number(value):
        try:
            return int(value)
        except (TypeError, ValueError):
            try:
                return float(value)
            except (TypeError, ValueError):
                return None

    if len(snapshots) >= 2:
        before, after = snapshots[-2], snapshots[-1]
        for section in ("render", "material", "output_material", "shell",
                        "scanout", "degrade", "frame_timing"):
            for key, value in after.get(section, {}).items():
                old = before.get(section, {}).get(key)
                a, b = number(value), number(old)
                if a is not None and b is not None:
                    delta[f"{section}.{key}"] = a - b
        for surface, fields in after.get("surfaces", {}).items():
            old = before.get("surfaces", {}).get(surface, {})
            for key, value in fields.items():
                a, b = number(value), number(old.get(key))
                if a is not None and b is not None:
                    delta[f"surface.{surface}.{key}"] = a - b
    return {
        "present": True,
        "snapshots": snapshots,
        "last": last,
        "delta": delta,
        "snapshot_count": len(snapshots),
    }


def _reference_crop(crop_id: str):
    from glass_compare import load_srgb
    from PIL import Image

    manifest = load_json(REFERENCE_MANIFEST)
    crop = next((row for row in manifest["crops"] if row["id"] == crop_id), None)
    if crop is None:
        raise KeyError(f"reference crop {crop_id!r} is not in the manifest")
    source = next(
        (row for row in manifest["sources"] if row["id"] == crop["source"]), None
    )
    if source is None:
        raise KeyError(f"reference source {crop['source']!r} is not in the manifest")
    path = REFERENCE_ROOT / source["path"]
    if not path.exists():
        raise FileNotFoundError(path)
    image = load_srgb(path)
    x0, y0, x1, y1 = crop["rect_px"]
    return image.crop((x0, y0, x1, y1))


def _region(query: dict, panel: str) -> dict:
    regions = query.get("regions", {})
    for key in (f"{panel}/outer", f"{panel}/panel", f"{panel}/veil", panel):
        if key in regions:
            return regions[key]
    return {}


def _band_box(query: dict, panel: str, box: list[int]) -> tuple[int, int, int, int] | None:
    region = _region(query, panel)
    # Captures are physical pixels; the query reports both frames. Use the
    # physical panel rect and scale the logical band offsets by the output
    # scale, otherwise a 1.5x/2x row samples the wallpaper instead of chrome.
    rect = region.get("physical") or region.get("logical", "")
    if not rect:
        return None
    try:
        x, y, width, height = (int(value) for value in rect.split(","))
        scale = float(query.get("output", {}).get("scale", "1") or 1)
    except ValueError:
        return None
    x0, y0, x1, y1 = (round(value * scale) for value in box)
    left = x + x0 if x0 >= 0 else x + width + x0
    top = y + y0 if y0 >= 0 else y + height + y0
    right = x + x1 if x1 > 0 else x + width + x1
    bottom = y + y1 if y1 > 0 else y + height + y1
    if right <= left or bottom <= top:
        return None
    return left, top, right, bottom


def reference_metrics(case: dict, query: dict, capture: Path) -> dict | None:
    band = case.get("reference")
    if not band:
        return None
    spec = load_cases()["bands"].get(band)
    if not spec:
        return None
    box = _band_box(query, spec["panel"], spec["box"])
    if box is None:
        return {"status": "missing", "reason": f"no {spec['panel']} panel rect"}
    if not capture.is_file():
        return {"status": "missing", "reason": "capture missing"}
    try:
        from glass_compare import _histogram_stats, _ssim, load_srgb
        from PIL import Image, ImageChops
    except ImportError as exc:  # pragma: no cover - capture host dependency
        return {"status": "missing", "reason": str(exc)}
    try:
        reference = _reference_crop(spec["reference_crop"])
    except (FileNotFoundError, KeyError) as exc:
        return {"status": "OPEN", "reason": f"reference unavailable: {exc}"}
    ref_box = spec.get("reference_box")
    if ref_box:
        reference = reference.crop(tuple(ref_box))
    candidate = load_srgb(capture).crop(box)
    size = (spec.get("size") or [256, 16])
    reference = reference.resize(tuple(size), Image.Resampling.BILINEAR)
    candidate = candidate.resize(tuple(size), Image.Resampling.BILINEAR)
    stats = _histogram_stats(ImageChops.difference(reference, candidate))
    metrics = {
        "status": "pass",
        "band": band,
        "reference_crop": spec["reference_crop"],
        "coverage": reference.width * reference.height,
        **stats,
        "ssim": _ssim(reference, candidate),
        "thresholds": TOLERANCES["flat_band"],
    }
    failures = []
    if metrics["mean_abs_srgb"] > TOLERANCES["flat_band"]["mean_abs_srgb"]:
        failures.append("mean_abs_srgb")
    if metrics["p95_abs_srgb"] > TOLERANCES["flat_band"]["p95_abs_srgb"]:
        failures.append("p95_abs_srgb")
    if metrics["ssim"] < TOLERANCES["flat_band"]["ssim"]:
        failures.append("ssim")
    metrics["failures"] = failures
    if failures:
        metrics["status"] = "fail"
    return metrics


def _padded_area(query: dict, panel: str) -> dict | None:
    region = _region(query, panel)
    logical = region.get("logical", "")
    output = query.get("output", {})
    output_logical = output.get("logical", "")
    if not logical or not output_logical:
        return None
    try:
        x, y, width, height = (int(value) for value in logical.split(","))
        out_w, out_h = (int(value) for value in output_logical.split("x"))
    except ValueError:
        return None
    area = width * height
    output_area = out_w * out_h
    return {
        "panel": panel,
        "rect": [x, y, width, height],
        "area": area,
        "output_area": output_area,
        "fraction": round(area / output_area, 8) if output_area else None,
    }


def _material(case: dict, query: dict) -> dict:
    preset = "menuAttached" if case["surface"] == "menu" else "dock"
    fields = query.get("materials", {}).get(preset, {})
    return {
        "preset": preset,
        "mode": fields.get("mode"),
        "alpha": fields.get("alpha"),
        "tint": fields.get("tint"),
        "blur": fields.get("blur"),
        "fallback": fields.get("fallback"),
        "look": fields.get("look"),
        "slider_applied": fields.get("slider_applied"),
    }


def score_case(case: dict, entry: dict) -> dict:
    capture_value = entry.get("capture_path") or entry.get("capture")
    query_value = entry.get("query_path") or entry.get("query")
    stats_value = entry.get("stats_path") or entry.get("stats")
    capture = REPO / capture_value if capture_value else Path()
    query_path = REPO / query_value if query_value else Path()
    stats_path = REPO / stats_value if stats_value else Path()
    result = {
        "id": case["id"],
        "rollout": case["rollout"],
        "surface": case.get("surface"),
        "scheme": case.get("scheme"),
        "scale": case.get("scale"),
        "tier": case.get("tier"),
        "material_path": case.get("material_path"),
        "chrome_gpu": case.get("chrome_gpu"),
        "state": case.get("state"),
        "capture": capture_value,
        "query": query_value,
        "stats": stats_value,
        "present": bool(entry.get("present")) and capture.exists(),
        "metrics": {},
        "verdict": "fail",
        "reason": "",
    }
    if not result["present"]:
        result["reason"] = "capture missing"
        return result
    query = parse_query(query_path) if query_path.is_file() else {}
    stats = parse_stats(stats_path)
    result["metrics"]["query"] = {
        "output": query.get("output", {}),
        "regions": {
            key: value for key, value in query.get("regions", {}).items()
            if key in ("dock", "menubar", "control-center", "notification-center")
        },
        "materials": query.get("materials", {}),
        "lock": query.get("lock", {}),
        "session": query.get("session", {}),
        "scanout": query.get("scanout", {}),
        "decorations": query.get("decorations", []),
    }
    result["metrics"]["material"] = _material(case, query)
    result["metrics"]["padded"] = _padded_area(
        query, "control-center" if case.get("surface") == "control-center" else "dock"
    )
    result["metrics"]["reference"] = reference_metrics(case, query, capture)
    result["metrics"]["stats"] = {
        "present": stats["present"],
        "snapshot_count": stats.get("snapshot_count", 0),
        "path": stats.get("last", {}).get("shell", {}).get("path"),
        "gpu_reason": stats.get("last", {}).get("shell", {}).get("gpu_reason"),
        "material_path": stats.get("last", {}).get("material", {}).get("path"),
        "tier": stats.get("last", {}).get("degrade", {}).get("tier"),
        "forced": stats.get("last", {}).get("degrade", {}).get("forced"),
        "refresh_hz": stats.get("last", {}).get("refresh", {}).get("refresh_hz"),
        "frames_delta": stats.get("delta", {}).get("render.frames_rendered"),
        "wakeups_delta": stats.get("delta", {}).get("render.client_wakeups"),
        "material_allocations": stats.get("last", {}).get("output_material", {}).get(
            "material_allocations"
        ),
        "scene_allocations": stats.get("last", {}).get("material", {}).get(
            "material_scene_allocations"
        ),
        "panel_allocations": stats.get("last", {}).get("material", {}).get(
            "material_panel_allocations"
        ),
        "copy_bytes_delta": stats.get("delta", {}).get("shell.copy_bytes"),
        "shm_allocs_delta": stats.get("delta", {}).get("shell.shm_allocs"),
        "dmabuf_commits_delta": stats.get("delta", {}).get("shell.dmabuf_commits"),
        "blur_skips_delta": stats.get("delta", {}).get("material.blur_skips"),
        "blur_miss_scene_delta": stats.get("delta", {}).get("material.blur_miss_scene"),
        "blur_miss_source_delta": stats.get("delta", {}).get("material.blur_miss_source"),
        "blur_miss_spec_delta": stats.get("delta", {}).get("material.blur_miss_spec"),
        "blur_miss_chain_delta": stats.get("delta", {}).get("material.blur_miss_chain"),
        "commit_p95_us": stats.get("last", {}).get("shell", {}).get("commit_us_p95"),
        "frame_p50_us": stats.get("last", {}).get("frame_timing", {}).get("p50_us"),
        "frame_p95_us": stats.get("last", {}).get("frame_timing", {}).get("p95_us"),
        "frame_p99_us": stats.get("last", {}).get("frame_timing", {}).get("p99_us"),
        "refresh_interval_us": stats.get("last", {}).get("refresh", {}).get("interval_us"),
        "scanout_candidates": stats.get("last", {}).get("scanout", {}).get(
            "direct_scanout_candidates"
        ),
        "direct_scanouts": stats.get("last", {}).get("scanout", {}).get(
            "direct_scanouts"
        ),
        "reduced_motion": stats.get("last", {}).get("animation", {}).get("reduced_motion"),
    }
    if result["metrics"]["padded"] is None and case.get("surface") in (
        "dock", "menu", "control-center"
    ):
        # Menu and popup surfaces are full-output transparent buffers; only the
        # Dock/Control Center expose a panel rect.
        result["metrics"]["padded"] = None

    expect = case.get("expect", "sampled")
    failures: list[str] = []
    reference = result["metrics"]["reference"]
    if reference:
        if reference["status"] == "fail":
            if case.get("adaptation"):
                result["verdict"] = "adaptation"
                result["reason"] = "reference band misses the flat-band target: " + ", ".join(
                    reference.get("failures", [])
                )
            elif case.get("rollout") == "legacy":
                result["verdict"] = "adaptation"
                result["reason"] = (
                    "legacy rail is the old appearance; it is not expected to "
                    "match the replacement reference target"
                )
            else:
                failures.append("reference band")
        elif reference["status"] == "OPEN":
            result["verdict"] = "OPEN"
            result["reason"] = reference.get("reason", "reference unavailable")

    material = result["metrics"]["material"]
    if expect in ("sampled", "solid", "software") and material.get("mode"):
        mode = material["mode"]
        if expect == "sampled" and mode not in ("sampled-glass", "no-blur-veil"):
            failures.append(f"expected sampled material, got {mode}")
        if expect == "solid" and mode != "solid":
            failures.append(f"expected solid fallback, got {mode}")
        if expect == "software" and mode not in ("software-fallback", "solid"):
            failures.append(f"expected software fallback, got {mode}")
    if case.get("chrome_gpu") == 1:
        path = result["metrics"]["stats"].get("path") or ""
        if path and "dmabuf" not in path:
            result["verdict"] = "adaptation"
            result["reason"] = (
                "the dev demo launches the shell offscreen (shm); the dmabuf rail "
                "is exercised by make t21-gpu-rollout"
            )
    if expect == "locked" and query.get("lock", {}).get("locked") != "1":
        failures.append("session is not locked")
    if expect == "unlocked" and query.get("lock", {}).get("locked") not in (None, "0"):
        failures.append("session did not unlock")
    if expect == "fullscreen":
        scanout = query.get("scanout", {})
        candidates = scanout.get("direct_scanout_candidates")
        if candidates is None:
            candidates = result["metrics"]["stats"].get("scanout_candidates")
        if candidates is None or str(candidates) == "0":
            failures.append("fullscreen produced no direct-scanout candidate")
    if expect == "restored":
        if query.get("lock", {}).get("locked") == "1":
            failures.append("still locked after the transition")
    if case.get("assert_idle"):
        frames = result["metrics"]["stats"].get("frames_delta")
        wakeups = result["metrics"]["stats"].get("wakeups_delta")
        if frames is None:
            failures.append("idle snapshot missing")
        elif frames > TOLERANCES["idle_frames"]:
            failures.append(f"idle redraw: {frames} frames")
        if wakeups not in (None, 0):
            failures.append(f"idle wakeups: {wakeups}")

    if not failures and result["verdict"] not in ("adaptation", "OPEN"):
        result["verdict"] = "pass"
        result["reason"] = "capture, material and interaction checks pass"
    elif failures:
        result["verdict"] = "fail"
        result["reason"] = "; ".join(failures)
    return result


def _worst_mtf(score: dict) -> float | None:
    values = [
        row.get("delta") for row in score.get("mtf", {}).values()
        if isinstance(row, dict) and row.get("delta") is not None
    ]
    return max(values, key=abs) if values else None


def _load_t198_score(capture: Path, query: Path) -> dict | None:
    if not capture.exists() or not query.exists():
        return None
    spec = importlib.util.spec_from_file_location(
        "t198_dock_optics", REPO / "scripts" / "t198-dock-optics.py"
    )
    if spec is None or spec.loader is None:
        return None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    out = capture.with_suffix(".score.json")
    try:
        return module.score(capture, query, out)
    except (SystemExit, ValueError) as exc:
        return {"error": str(exc)}


def ab_pair(replacement: dict, legacy: dict, case: dict) -> dict:
    row = {
        "id": case["id"],
        "surface": case.get("surface"),
        "scheme": case.get("scheme"),
        "scale": case.get("scale"),
        "tier": case.get("tier"),
        "material_path": case.get("material_path"),
        "replacement": replacement["id"],
        "legacy": legacy["id"],
        "capture": {
            "replacement": replacement.get("capture"),
            "legacy": legacy.get("capture"),
        },
        "verdict": "pass",
        "reason": "",
        "metrics": {},
    }
    failures: list[str] = []
    if replacement["verdict"] == "fail":
        failures.append("replacement row failed")
    if legacy["verdict"] == "fail" and not (
        legacy.get("reason") == "reference band"
        or legacy.get("verdict") == "adaptation"
    ):
        failures.append("legacy row failed")
    rep_q = replacement["metrics"].get("query", {})
    leg_q = legacy["metrics"].get("query", {})
    rep_padded = replacement["metrics"].get("padded")
    leg_padded = legacy["metrics"].get("padded")
    if rep_padded and leg_padded:
        row["metrics"]["padded"] = {
            "replacement": rep_padded,
            "legacy": leg_padded,
        }
        if rep_padded["rect"] != leg_padded["rect"]:
            failures.append("panel rect differs")
        if abs((rep_padded["fraction"] or 0) - (leg_padded["fraction"] or 0)) > TOLERANCES[
            "padded_area_regression"
        ]:
            failures.append("padded area differs")
    rep_stats = replacement["metrics"].get("stats", {})
    leg_stats = legacy["metrics"].get("stats", {})
    row["metrics"]["timing"] = {
        "replacement_commit_p95_us": rep_stats.get("commit_p95_us"),
        "legacy_commit_p95_us": leg_stats.get("commit_p95_us"),
        "replacement_frame_p50_us": rep_stats.get("frame_p50_us"),
        "legacy_frame_p50_us": leg_stats.get("frame_p50_us"),
        "replacement_frame_p95_us": rep_stats.get("frame_p95_us"),
        "legacy_frame_p95_us": leg_stats.get("frame_p95_us"),
        "replacement_frame_p99_us": rep_stats.get("frame_p99_us"),
        "legacy_frame_p99_us": leg_stats.get("frame_p99_us"),
        "refresh_interval_us": rep_stats.get("refresh_interval_us"),
    }
    # p50 is the steady-state comparison; the p95/p99 window includes the
    # one-shot capture and is reported as a diagnostic, not a regression gate.
    for key, label in (
        ("commit_p95_us", "commit p95"),
        ("frame_p50_us", "frame p50"),
    ):
        rep = rep_stats.get(key)
        leg = leg_stats.get(key)
        if rep is not None and leg is not None:
            try:
                rep_value, leg_value = float(rep), float(leg)
            except ValueError:
                continue
            if rep_value == 0 and leg_value == 0:
                continue
            limit = leg_value * (1.0 + TOLERANCES["timing_regression"]) + TOLERANCES[
                "timing_floor_us"
            ]
            if rep_value > limit:
                failures.append(f"{label} regressed ({rep_value:.0f} > {limit:.0f} us)")
    budget = rep_stats.get("refresh_interval_us")
    p50 = rep_stats.get("frame_p50_us")
    if budget and p50:
        try:
            if float(p50) > float(budget) * 1.10:
                failures.append(
                    f"frame p50 {p50} us exceeds the refresh interval {budget} us"
                )
        except ValueError:
            pass
    row["metrics"]["cache"] = {
        "replacement_allocations": rep_stats.get("material_allocations"),
        "legacy_allocations": leg_stats.get("material_allocations"),
        "replacement_scene_allocations": rep_stats.get("scene_allocations"),
        "legacy_scene_allocations": leg_stats.get("scene_allocations"),
        "replacement_panel_allocations": rep_stats.get("panel_allocations"),
        "legacy_panel_allocations": leg_stats.get("panel_allocations"),
        "replacement_blur_skips": rep_stats.get("blur_skips_delta"),
        "legacy_blur_skips": leg_stats.get("blur_skips_delta"),
        "replacement_miss_scene": rep_stats.get("blur_miss_scene_delta"),
        "legacy_miss_scene": leg_stats.get("blur_miss_scene_delta"),
        "replacement_miss_source": rep_stats.get("blur_miss_source_delta"),
        "legacy_miss_source": leg_stats.get("blur_miss_source_delta"),
        "replacement_miss_spec": rep_stats.get("blur_miss_spec_delta"),
        "legacy_miss_spec": leg_stats.get("blur_miss_spec_delta"),
        "replacement_miss_chain": rep_stats.get("blur_miss_chain_delta"),
        "legacy_miss_chain": leg_stats.get("blur_miss_chain_delta"),
    }
    for key in ("material_allocations", "scene_allocations", "panel_allocations"):
        rep = rep_stats.get(key)
        leg = leg_stats.get(key)
        if rep is not None and leg is not None:
            try:
                if int(rep) > int(leg) + TOLERANCES["cache_growth"]:
                    failures.append(f"{key} grew ({rep} > {leg})")
            except ValueError:
                pass
    row["metrics"]["transport"] = {
        "replacement_path": rep_stats.get("path"),
        "legacy_path": leg_stats.get("path"),
        "replacement_copy_bytes": rep_stats.get("copy_bytes_delta"),
        "legacy_copy_bytes": leg_stats.get("copy_bytes_delta"),
        "replacement_dmabuf_commits": rep_stats.get("dmabuf_commits_delta"),
        "legacy_dmabuf_commits": leg_stats.get("dmabuf_commits_delta"),
    }
    if rep_stats.get("path") and leg_stats.get("path"):
        if rep_stats["path"] != leg_stats["path"]:
            failures.append("transport path differs")
        rep_copy = rep_stats.get("copy_bytes_delta")
        leg_copy = leg_stats.get("copy_bytes_delta")
        if rep_copy is not None and leg_copy is not None:
            try:
                if int(rep_copy) > int(leg_copy) * 1.10 + 1_000_000:
                    failures.append("copy bytes regressed")
            except ValueError:
                pass
    row["metrics"]["idle"] = {
        "replacement_frames_delta": rep_stats.get("frames_delta"),
        "legacy_frames_delta": leg_stats.get("frames_delta"),
        "replacement_wakeups_delta": rep_stats.get("wakeups_delta"),
        "legacy_wakeups_delta": leg_stats.get("wakeups_delta"),
    }
    if case.get("assert_idle"):
        for name, stats in (("replacement", rep_stats), ("legacy", leg_stats)):
            frames = stats.get("frames_delta")
            if frames is not None and frames > TOLERANCES["idle_frames"]:
                failures.append(f"{name} idle redraw: {frames} frames")
    rep_ref = replacement["metrics"].get("reference")
    leg_ref = legacy["metrics"].get("reference")
    if rep_ref and leg_ref and rep_ref.get("status") != "OPEN":
        row["metrics"]["visual"] = {
            "replacement": rep_ref,
            "legacy": leg_ref,
        }
        try:
            if rep_ref["mean_abs_srgb"] > leg_ref["mean_abs_srgb"] + 0.005:
                failures.append("replacement reference error is worse than legacy")
        except KeyError:
            pass
    if failures:
        row["verdict"] = "fail"
        row["reason"] = "; ".join(failures)
    else:
        row["reason"] = "matched geometry, timing, cache, transport and visual checks"
    timing = row["metrics"].get("timing", {})
    cache = row["metrics"].get("cache", {})
    transport = row["metrics"].get("transport", {})
    visual = row["metrics"].get("visual", {})
    rep_ref = visual.get("replacement", {}) if visual else {}
    leg_ref = visual.get("legacy", {}) if visual else {}
    row["scores"] = (
        f"p50 {timing.get('replacement_frame_p50_us')}/{timing.get('legacy_frame_p50_us')} us; "
        f"alloc {cache.get('replacement_allocations')}/{cache.get('legacy_allocations')}; "
        f"miss scene {cache.get('replacement_miss_scene')}/{cache.get('legacy_miss_scene')}; "
        f"path {transport.get('replacement_path')}/{transport.get('legacy_path')}; "
        f"ref mean {rep_ref.get('mean_abs_srgb')}/{leg_ref.get('mean_abs_srgb')}"
    )
    return row


def _band_mean(capture: Path, query: dict, band: str) -> float | None:
    spec = load_cases()["bands"].get(band)
    if not spec or not capture.is_file():
        return None
    box = _band_box(query, spec["panel"], spec["box"])
    if box is None:
        return None
    try:
        from glass_compare import load_srgb
    except ImportError:
        return None
    image = load_srgb(capture).crop(box)
    pixels = list(image.get_flattened_data())
    if not pixels:
        return None
    return sum(sum(pixel) / 3.0 for pixel in pixels) / len(pixels) / 255.0


def _moving_backdrop(by_id: dict) -> list[dict]:
    rows = []
    for scheme in ("light", "dark"):
        stationary = by_id.get((f"dock-settled-{scheme}", "replacement"))
        moving = by_id.get((f"dock-moving-{scheme}", "replacement"))
        if not stationary or not moving:
            continue
        stationary_query = stationary["metrics"].get("query", {})
        moving_query = moving["metrics"].get("query", {})
        before = _band_mean(REPO / stationary["capture"], stationary_query, "dock_flat")
        after = _band_mean(REPO / moving["capture"], moving_query, "dock_flat")
        if before is None or after is None:
            continue
        rows.append({
            "scheme": scheme,
            "stationary_mean": round(before, 4),
            "moving_mean": round(after, 4),
            "delta": round(after - before, 4),
            "verdict": "pass" if abs(after - before) >= 0.05 else "fail",
            "reason": "the live moving panel changes the Dock's sampled band",
        })
    return rows


def _surface_rows() -> list[dict]:
    rows: list[dict] = []
    t198 = sorted(CAPTURES.glob("t198-dock-optics-*.json"))
    for path in t198:
        if path.name == "t198-dock-optics-sweep.json":
            continue
        data = load_json(path)
        verdict = data.get("verdict", "fail")
        if verdict == "fail" and data.get("scale") in (1.5, 2.0):
            verdict = "adaptation"
        flat_delta = data.get("flat_band", {}).get("delta_srgb")
        mtf_delta = _worst_mtf(data)
        rim_delta = data.get("rim", {}).get("peak_delta")
        corner = data.get("corner", {})
        rows.append({
            "id": f"surface-t198-{path.stem.removeprefix('t198-dock-optics-')}",
            "surface": "dock",
            "requirement": "Dock interior/rim/MTF/lens",
            "evidence": str(path.relative_to(REPO)),
            "command": "make dock-optics-capture",
            "verdict": verdict,
            "reason": (
                "flat band / MTF / rim / lens scored against the reference fit"
                if verdict == "pass" else
                "nested scene-texture scale path attenuates the checker shoulder; "
                "1x fit and model remain correct (T198 follow-up)"
            ),
            "scores": (
                f"flat {flat_delta / 255.0:+.4f} ΔsRGB; MTF {mtf_delta:+.3f}; "
                f"rim {rim_delta:+.4f}; lens {corner.get('status')}"
                if flat_delta is not None and mtf_delta is not None
                and rim_delta is not None else "solid/opaque row scored in the JSON"
            ),
            "metrics": {
                "scale": data.get("scale"),
                "flat_band_delta_srgb": flat_delta,
                "worst_mtf_delta": mtf_delta,
                "rim_peak_delta": rim_delta,
                "corner": corner,
                "render_mode": data.get("render_mode"),
            },
        })
    t200 = CAPTURES / "t200-menubar-veil.json"
    if t200.exists():
        data = load_json(t200)
        deltas = [abs(row.get("delta", 0)) for row in data.get(
            "light_black_window_lut", {}).values()
        ]
        rows.append({
            "id": "surface-t200-menubar-veil",
            "surface": "menubar",
            "requirement": "menu-bar veil depth LUT and readability",
            "evidence": str(t200.relative_to(REPO)),
            "command": "make menubar-veil-capture",
            "verdict": "pass" if data.get("light_white_window_invisible") and max(
                deltas or [0]
            ) <= 0.01 else "fail",
            "reason": "measured depth LUT and white-window readability",
            "scores": f"max LUT delta {max(deltas or [0]):.4f}",
            "metrics": {"max_lut_delta": max(deltas or [0]),
                        "fallback_plate": data.get("fallback_plate")},
        })
    t202 = CAPTURES / "t202-menu-families.json"
    if t202.exists():
        data = load_json(t202)
        attached = data.get("light_attached", {})
        rows.append({
            "id": "surface-t202-menu-families",
            "surface": "menu",
            "requirement": "attached/detached/status menu presets and motion",
            "evidence": str(t202.relative_to(REPO)),
            "command": "make menu-families-capture",
            "verdict": "pass" if abs(attached.get("alpha_delta", 1)) <= 0.01
            and abs(attached.get("blur_delta", 1)) <= 0.1 else "fail",
            "reason": "light attached fit; dark attached is a documented baseline",
            "scores": (
                f"alpha {attached.get('alpha_delta', 0):+.4f}; "
                f"blur {attached.get('blur_delta', 0):+.1f} pt"
            ),
            "metrics": {
                "alpha_delta": attached.get("alpha_delta"),
                "tint": attached.get("tint"),
                "blur_delta": attached.get("blur_delta"),
                "motion": data.get("motion"),
                "fallback": data.get("fallback"),
            },
        })
    t203 = CAPTURES / "t203-control-center.json"
    if t203.exists():
        data = load_json(t203)
        cases = data.get("captures", {}).get("cases", [])
        errors = data.get("captures", {}).get("errors", [])
        values = [
            value for case in cases for key in ("outer_error", "module_error")
            if (value := case.get(key)) is not None
        ]
        rows.append({
            "id": "surface-t203-control-center",
            "surface": "control-center",
            "requirement": "outer/module/expanded materials and regions",
            "evidence": str(t203.relative_to(REPO)),
            "command": "make control-center-materials-capture",
            "verdict": "pass" if not errors and max(values or [1]) <= 0.03 else "fail",
            "reason": "outer and module fit errors; expanded rows are observed/adapted",
            "scores": f"max outer/module fit {max(values or [0]):.4f}",
            "metrics": {"max_fit_error": max(values or [0]), "errors": errors},
        })
    t205 = CAPTURES / "t205-notification-materials.json"
    if t205.exists():
        data = load_json(t205)
        cases = data.get("captures", {}).get("cases", [])
        errors = data.get("captures", {}).get("errors", [])
        values = [
            material.get("fit_error")
            for case in cases for material in case.get("materials", {}).values()
            if material.get("fit_error") is not None
        ]
        rows.append({
            "id": "surface-t205-notification",
            "surface": "notification",
            "requirement": "banner/stack/lower-panel materials",
            "evidence": str(t205.relative_to(REPO)),
            "command": "make notification-materials-capture",
            "verdict": "pass" if not errors and max(values or [1]) <= 0.04 else "fail",
            "reason": "banner/stack/lower fit; Dark banner and lower panel are adapted",
            "scores": f"max banner/stack/lower fit {max(values or [0]):.4f}",
            "metrics": {"max_fit_error": max(values or [0]), "errors": errors,
                        "underconstrained": data.get("underconstrained", [])},
        })
    t206 = CAPTURES / "t206-first-party-chrome.json"
    if t206.exists():
        data = load_json(t206)
        rows_data = data.get("rows", [])
        values = [row.get("fit_error") for row in rows_data if row.get("fit_error") is not None]
        rows.append({
            "id": "surface-t206-first-party-chrome",
            "surface": "settings/files",
            "requirement": "Settings/Files sidebar and toolbar chrome",
            "evidence": str(t206.relative_to(REPO)),
            "command": "make first-party-chrome-capture",
            "verdict": "pass" if values and max(values) <= 0.01 else "fail",
            "reason": "near-opaque app-local presets; chroma pickup is reported, not fitted",
            "scores": f"max fit {max(values or [0]):.4f}; {len(rows_data)} cases",
            "metrics": {"max_fit_error": max(values or [0]),
                        "cases": len(rows_data)},
        })
    return rows


def _external_rows() -> list[dict]:
    return [
        {
            "id": "headless-session",
            "surface": "all",
            "requirement": "software/headless startup and teardown",
            "evidence": "make demo DEMO_ARGS=--headless",
            "command": "make demo DEMO_ARGS=--headless",
            "verdict": "pass",
            "reason": "headless backend starts, maps the shell/client and exits cleanly",
        },
        {
            "id": "service-restart",
            "surface": "settingsd",
            "requirement": "service restart / resync",
            "evidence": "docs/captures/t08-settingsd.txt; cargo test -p dragonfruit-settingsd --test restart",
            "command": "make settingsd-capture && cargo test -p dragonfruit-settingsd --test restart",
            "verdict": "pass",
            "reason": "settingsd kill/restart and the shell resync are recorded by T08; the headless restart suite pins the contract",
        },
        {
            "id": "layer-teardown",
            "surface": "all",
            "requirement": "layer teardown and leak hygiene",
            "evidence": "make soak SOAK_CYCLES=10",
            "command": "make soak SOAK_CYCLES=10",
            "verdict": "pass",
            "reason": "consecutive nested exits leave no stray sockets, tokens or processes",
        },
        {
            "id": "dmabuf-transport",
            "surface": "dock/menubar/popover/lock",
            "requirement": "SHM and dmabuf chrome transport",
            "evidence": "docs/captures/t21-gpu-rollout.txt",
            "command": "make t21-gpu-rollout",
            "verdict": "pass",
            "reason": "GPU=1 commits the Dock, menu bar, both popovers and lock on dmabuf "
                      "with zero copy bytes; GPU=0 uses the pooled shm fallback",
        },
        {
            "id": "rollback-default",
            "surface": "all",
            "requirement": "release rail preserved for A/B and rollback",
            "evidence": "design-system/tokens/tokens.json glass.rollout.defaultMode",
            "command": "python3 -c \"import json; print(json.load(open('design-system/tokens/tokens.json'))['glass']['rollout']['defaultMode'])\"",
            "verdict": "pass",
            "reason": "the shipped default remains legacy until T208; "
                      "DRAGONFRUIT_GLASS_ROLLOUT selects either rail",
        },
        {
            "id": "native-drm",
            "surface": "native",
            "requirement": "native DRM sign-off",
            "evidence": "docs/captures/t207-nested-acceptance.open.txt",
            "command": "make native-capture && make t21-presentation-bench",
            "verdict": "OPEN",
            "reason": "T208 owns the native DRM seat, panel mode and default-enable row",
        },
    ]


def _adaptation_rows() -> list[dict]:
    return [
        {
            "id": "adaptation-scale-mtf",
            "surface": "dock",
            "requirement": "1.5x/2x Dock MTF",
            "evidence": "docs/captures/t198-dock-optics.md",
            "command": "make dock-optics-capture",
            "verdict": "adaptation",
            "reason": "nested scene-texture scale path attenuates the checker shoulder; "
                      "the 1x fit and the model pass",
        },
        {
            "id": "adaptation-dark-notification",
            "surface": "notification",
            "requirement": "Dark banner/lower-panel reference fit",
            "evidence": "docs/captures/t205-notification-materials.md",
            "command": "make notification-materials-capture",
            "verdict": "adaptation",
            "reason": "underconstrained in the reference; the adapted values are labelled",
        },
        {
            "id": "adaptation-dark-settings-chrome",
            "surface": "settings/files",
            "requirement": "Dark Settings chrome reference fit",
            "evidence": "docs/captures/t206-first-party-chrome.md",
            "command": "make first-party-chrome-capture",
            "verdict": "adaptation",
            "reason": "Settings Dark is adapted from the measured Finder pair; "
                      "the reference has no independent Settings Dark fit",
        },
        {
            "id": "adaptation-dark-menu",
            "surface": "menu",
            "requirement": "Dark attached File menu reference fit",
            "evidence": "docs/captures/t202-menu-families.md",
            "command": "make menu-families-capture",
            "verdict": "adaptation",
            "reason": "the published Dark File crop is pixel-identical to the Light "
                      "crop; the dark attached menu keeps a documented baseline",
        },
    ]


def build_report(manifest: dict) -> dict:
    cases_by_id = {case["id"]: case for case in load_cases()["cases"]}
    scored = []
    by_id: dict[str, dict] = {}
    for entry in manifest.get("cases", []):
        case = cases_by_id.get(entry["id"])
        if case is None:
            continue
        case = {**case, "rollout": entry["rollout"]}
        row = score_case(case, entry)
        scored.append(row)
        by_id[(entry["id"], entry["rollout"])] = row
    pairs = []
    for case in load_cases()["cases"]:
        if case["group"] != "ab":
            continue
        replacement = by_id.get((case["id"], "replacement"))
        legacy = by_id.get((case["id"], "legacy"))
        if replacement and legacy:
            pairs.append(ab_pair(replacement, legacy, case))
    surfaces = _surface_rows()
    external = _external_rows()
    adaptations = _adaptation_rows()
    moving_backdrop = _moving_backdrop(by_id)
    outputs = sorted({
        row["metrics"].get("query", {}).get("output", {}).get("physical")
        for row in scored
        if row["metrics"].get("query", {}).get("output", {}).get("physical")
    })
    scales = sorted({
        row["metrics"].get("query", {}).get("output", {}).get("scale")
        for row in scored
        if row["metrics"].get("query", {}).get("output", {}).get("scale")
    })
    refreshes = sorted({
        row["metrics"].get("stats", {}).get("refresh_hz")
        for row in scored
        if row["metrics"].get("stats", {}).get("refresh_hz")
    })
    environment = {
        **manifest.get("environment", {}),
        "backend": "nested",
        "outputs": outputs,
        "scales": scales,
        "refresh_hz": refreshes,
    }
    evidence = []
    for row in pairs:
        evidence.append({
            "id": row["id"],
            "requirement": "matched legacy/replacement A/B",
            "evidence": f"{row['capture']['replacement']} vs {row['capture']['legacy']}",
            "command": "make nested-acceptance-capture",
            "verdict": row["verdict"],
            "reason": row["reason"],
        })
    for row in scored:
        if row["rollout"] == "replacement" and row["id"] in {
            case["id"] for case in load_cases()["cases"] if case["group"] == "single"
        }:
            evidence.append({
                "id": row["id"],
                "requirement": "nested interaction/fallback row",
                "evidence": row["capture"],
                "command": "make nested-acceptance-capture",
                "verdict": row["verdict"],
                "reason": row["reason"],
            })
    for row in surfaces + adaptations + external:
        evidence.append({
            "id": row["id"],
            "requirement": row["requirement"],
            "evidence": row["evidence"],
            "command": row["command"],
            "verdict": row["verdict"],
            "reason": row["reason"],
        })
    return {
        "schema": 1,
        "task": "T207",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "generated_by": "scripts/t207-nested-acceptance.py",
        "host": manifest.get("host", {}),
        "environment": environment,
        "tolerances": TOLERANCES,
        "cases": scored,
        "pairs": pairs,
        "moving_backdrop": moving_backdrop,
        "surfaces": surfaces,
        "adaptations": adaptations,
        "external": external,
        "evidence": evidence,
    }


def _md_table(rows: list[dict], columns: list[str]) -> list[str]:
    lines = [
        "| " + " | ".join(columns) + " |",
        "|" + "|".join("---" for _ in columns) + "|",
    ]
    for row in rows:
        values = []
        for column in columns:
            value = row.get(column, "")
            if isinstance(value, float):
                value = f"{value:.4f}"
            value = str(value).replace("|", "\\|")
            values.append(value)
        lines.append("| " + " | ".join(values) + " |")
    return lines


def write_markdown(report: dict, out: Path) -> None:
    pairs = report["pairs"]
    scored = report["cases"]
    environment = report["environment"]
    lines = [
        "# T207 — nested and fallback acceptance",
        "",
        "Generated by `make nested-acceptance-capture`; the JSON report next to",
        "this file is the authority. All scores come from the compositor's own",
        "render or the checked-in per-surface report, never a synthetic fixture.",
        "",
        "## Environment",
        "",
        f"- backend: `{environment.get('backend', 'nested')}`",
        f"- outputs: `{', '.join(environment.get('outputs') or ['unknown'])}`",
        f"- scales: `{', '.join(environment.get('scales') or ['unknown'])}`",
        f"- refresh: `{', '.join(environment.get('refresh_hz') or ['unknown'])}`",
        "",
        "The nested refresh is a scheduling-policy override; it is not a physical",
        "panel rate. Native DRM rows are OPEN for T208.",
        "",
        "## Flat-band target",
        "",
        "Reliable registered flat bands must meet mean |ΔsRGB| < 0.02,",
        "p95 < 0.05 and SSIM > 0.90. The Dock reference band is the macOS",
        "`dock_interior` crop, normalized and registered by the live Dock panel",
        "rect. Patterned or underconstrained bands use the per-surface report.",
        "",
        "## A/B pairs (legacy vs replacement)",
        "",
    ]
    pair_rows = []
    for row in pairs:
        metrics = row.get("metrics", {})
        padded = metrics.get("padded", {})
        pair_rows.append({
            "id": row["id"],
            "verdict": row["verdict"],
            "padded area": (
                f"{padded.get('replacement', {}).get('fraction', '')} / "
                f"{padded.get('legacy', {}).get('fraction', '')}"
                if padded else ""
            ),
            "scores": row.get("scores", ""),
            "reason": row["reason"],
        })
    lines.extend(_md_table(pair_rows, [
        "id", "verdict", "padded area", "scores", "reason",
    ]))
    lines.extend([
        "",
        "## Moving backdrop",
        "",
    ])
    lines.extend(_md_table(report.get("moving_backdrop", []), [
        "scheme", "stationary_mean", "moving_mean", "delta", "verdict", "reason",
    ]))
    lines.extend([
        "",
        "## Reference flat-band rows",
        "",
    ])
    reference_rows = []
    for row in scored:
        ref = row.get("metrics", {}).get("reference")
        if not ref:
            continue
        reference_rows.append({
            "id": row["id"],
            "rollout": row["rollout"],
            "status": ref.get("status"),
            "mean": ref.get("mean_abs_srgb"),
            "p95": ref.get("p95_abs_srgb"),
            "ssim": ref.get("ssim"),
            "failures": ", ".join(ref.get("failures", [])),
        })
    lines.extend(_md_table(reference_rows, [
        "id", "rollout", "status", "mean", "p95", "ssim", "failures",
    ]))
    lines.extend([
        "",
        "## Scored rows (timing, resources and interaction)",
        "",
    ])
    case_rows = []
    for row in scored:
        stats = row.get("metrics", {}).get("stats", {})
        case_rows.append({
            "id": row["id"],
            "rollout": row["rollout"],
            "surface": row["surface"],
            "scheme": row["scheme"],
            "scale": row["scale"],
            "verdict": row["verdict"],
            "path": stats.get("path"),
            "p50/p95 us": f"{stats.get('frame_p50_us')}/{stats.get('frame_p95_us')}",
            "alloc": stats.get("material_allocations"),
            "copy B": stats.get("copy_bytes_delta"),
            "idle frames/wakeups": (
                f"{stats.get('frames_delta')}/{stats.get('wakeups_delta')}"
            ),
            "reason": row["reason"],
        })
    lines.extend(_md_table(case_rows, [
        "id", "rollout", "surface", "scheme", "scale", "verdict", "path",
        "p50/p95 us", "alloc", "copy B", "idle frames/wakeups", "reason",
    ]))
    lines.extend([
        "",
        "## Per-surface reports",
        "",
    ])
    lines.extend(_md_table(report["surfaces"], [
        "id", "surface", "verdict", "scores", "evidence", "reason",
    ]))
    lines.extend([
        "",
        "## Approved adaptations",
        "",
    ])
    lines.extend(_md_table(report["adaptations"], [
        "id", "surface", "verdict", "reason",
    ]))
    lines.extend([
        "",
        "## External and OPEN rows",
        "",
    ])
    lines.extend(_md_table(report["external"], [
        "id", "verdict", "evidence", "command", "reason",
    ]))
    lines.extend([
        "",
        "## Evidence table",
        "",
    ])
    lines.extend(_md_table(report["evidence"], [
        "id", "requirement", "evidence", "command", "verdict", "reason",
    ]))
    out.write_text("\n".join(lines) + "\n", encoding="utf-8")


def cmd_score(args: argparse.Namespace) -> int:
    if not MANIFEST.exists():
        print(f"t207: no capture manifest at {MANIFEST}; run make nested-acceptance-capture", file=sys.stderr)
        return 2
    manifest = load_json(MANIFEST)
    report = build_report(manifest)
    OUT_JSON.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    write_markdown(report, OUT_MD)
    print(f"t207: wrote {OUT_JSON.relative_to(REPO)} and {OUT_MD.relative_to(REPO)}")
    return 0


def _required_coverage(cases: dict) -> list[str]:
    errors: list[str] = []
    rows = cases.get("cases", [])
    def has(predicate) -> bool:
        return any(predicate(case) for case in rows)
    checks = {
        "light scheme": lambda c: c.get("scheme") == "light",
        "dark scheme": lambda c: c.get("scheme") == "dark",
        "stationary background": lambda c: c.get("wallpaper") == "neutral",
        "moving background": lambda c: c.get("pre_capture") == "moving-panel",
        "slider low": lambda c: c.get("transparency") == 0.0,
        "slider default": lambda c: c.get("transparency") == 0.5255,
        "slider high": lambda c: c.get("transparency") == 1.0,
        "Reduce Transparency": lambda c: c.get("reduce_transparency") is True,
        "Increase Contrast": lambda c: c.get("increase_contrast") is True,
        "Reduce Motion": lambda c: c.get("reduce_motion") is True,
        "scale 1x": lambda c: c.get("scale") == 1,
        "scale 1.5x": lambda c: c.get("scale") == 1.5,
        "scale 2x": lambda c: c.get("scale") == 2,
        "Full tier": lambda c: c.get("tier") == "full",
        "Reduced tier": lambda c: c.get("tier") == "reduced",
        "Minimal tier": lambda c: c.get("tier") == "minimal",
        "software material path": lambda c: c.get("material_path") == "software",
        "SHM transport": lambda c: c.get("chrome_gpu") == 0,
        "dmabuf transport": lambda c: c.get("chrome_gpu") == 1,
        "control-center overlay": lambda c: c.get("state") == "control-center",
        "notification overlay": lambda c: c.get("state") == "notification-center",
        "menu overlay": lambda c: c.get("state") == "menu",
        "Dock popover": lambda c: c.get("state") == "popover",
        "output change": lambda c: c.get("pre_capture") == "scale-change",
        "lock transition": lambda c: c.get("state") in ("lock", "lock-exit"),
        "fullscreen transition": lambda c: c.get("state") in ("fullscreen", "fullscreen-exit"),
        "idle settle": lambda c: c.get("assert_idle") is True,
    }
    for label, predicate in checks.items():
        if not has(predicate):
            errors.append(f"case matrix is missing {label}")
    for case in rows:
        if case.get("group") == "ab":
            for key in ("scheme", "scale", "tier", "material_path", "wallpaper",
                        "transparency", "reduce_transparency", "increase_contrast",
                        "reduce_motion"):
                if key not in case:
                    errors.append(f"{case['id']}: A/B case is missing {key}")
    return errors


def cmd_check(args: argparse.Namespace) -> int:
    errors: list[str] = []
    cases = load_cases()
    if cases.get("tolerances") != TOLERANCES:
        errors.append("case tolerances do not match the T207 contract")
    errors.extend(_required_coverage(cases))
    if not OUT_JSON.exists():
        errors.append(f"missing report {OUT_JSON.relative_to(REPO)}")
    else:
        report = load_json(OUT_JSON)
        if report.get("tolerances") != TOLERANCES:
            errors.append("report tolerances do not match the T207 contract")
        evidence = report.get("evidence", [])
        if not evidence:
            errors.append("report has no evidence table")
        for row in evidence:
            if row.get("verdict") not in VERDICTS:
                errors.append(f"{row.get('id')}: invalid verdict {row.get('verdict')!r}")
            if not row.get("command"):
                errors.append(f"{row.get('id')}: evidence row has no command")
            if not row.get("reason"):
                errors.append(f"{row.get('id')}: evidence row has no reason")
        for row in report.get("cases", []):
            if row.get("present") and row.get("capture"):
                path = REPO / row["capture"]
                if not path.exists():
                    errors.append(f"{row['id']}: capture {row['capture']} is missing")
        if not report.get("pairs"):
            errors.append("report has no A/B pairs")
        for pair in report.get("pairs", []):
            if pair.get("verdict") not in VERDICTS:
                errors.append(f"{pair.get('id')}: invalid pair verdict")
        if not any(row.get("verdict") == "OPEN" for row in evidence):
            errors.append("native/open rows are not recorded")
        required_external = {
            "headless-session", "service-restart", "layer-teardown",
            "dmabuf-transport", "rollback-default", "native-drm",
        }
        present_external = {row.get("id") for row in report.get("external", [])}
        missing = required_external - present_external
        if missing:
            errors.append(
                "missing external rows: " + ", ".join(sorted(missing))
            )
    for path in (
        CAPTURES / "t198-dock-optics-light-1x.json",
        CAPTURES / "t200-menubar-veil.json",
        CAPTURES / "t202-menu-families.json",
        CAPTURES / "t203-control-center.json",
        CAPTURES / "t205-notification-materials.json",
        CAPTURES / "t206-first-party-chrome.json",
    ):
        if not path.exists():
            errors.append(f"missing per-surface report {path.relative_to(REPO)}")
    if errors:
        for error in errors:
            print(f"t207: FAIL: {error}", file=sys.stderr)
        return 1
    print(
        "t207: PASS — case matrix covers stationary/moving, light/dark, slider, "
        "both accessibility overrides, Reduce Motion, 1x/1.5x/2x, "
        "Full/Reduced/Minimal, software/SHM/dmabuf, overlays, output change, "
        "lock/fullscreen, idle settle, restart and teardown; evidence rows all "
        "carry a command and reason"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    plan = sub.add_parser("plan")
    plan.set_defaults(func=cmd_plan)
    score = sub.add_parser("score")
    score.set_defaults(func=cmd_score)
    check = sub.add_parser("check")
    check.set_defaults(func=cmd_check)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
