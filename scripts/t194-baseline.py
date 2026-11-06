#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Write the T194 baseline manifest and derived surface crops.

The starting Dragonfruit appearance is recorded from the committed v0.1.1
nested captures (``t20-chrome-*``) and the T-21 presentation benchmark.  The
script crops the surfaces the glass work compares — Dock, menu bar, attached
menu, detached menu, Control Center and notification banner — into stable
``t194-glass-baseline-*`` files, hashes them, and records the frame, region,
allocation and transport counters from the benchmark's stationary (idle) and
moving (Dock sweep) snapshots.

If a live host capture is available, ``scripts/capture-t194-glass-baseline.sh``
can point ``--source-dir`` at a fresh ``capture-t20-chrome-material.sh`` output
directory; the crop and manifest logic is identical.  When the host capture
cannot run, the script still writes the baseline and the capture script writes
an explicit OPEN artifact with the exact command and reason.

Usage:
    python3 scripts/t194-baseline.py --write
    python3 scripts/t194-baseline.py --check
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parent.parent
CAPTURES = REPO / "docs" / "captures"
OUT_JSON = CAPTURES / "t194-glass-baseline.json"
OUT_MD = CAPTURES / "t194-glass-baseline.md"
OPEN_TXT = CAPTURES / "t194-glass-baseline.open.txt"
BENCH = CAPTURES / "t21-presentation-bench-raw" / "main-144-full"
REFERENCE_MANIFEST = CAPTURES / "glass-reference-manifest.json"

SURFACES = [
    ("dock", "dock", "Dock plate and bar background"),
    ("bar", "dock", "menu bar"),
    ("menu_attached", "menubar", "attached menu-bar dropdown"),
    ("menu_detached", "contextmenu", "detached Dock context menu"),
    ("control_center", "controlcenter", "Control Center panel"),
    ("banner", "notification", "notification banner"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _crop_rect(surface: str, width: int, height: int) -> tuple[int, int, int, int]:
    if surface == "dock":
        return (
            round(width * 560 / 1920),
            height - 200,
            round(width * 1360 / 1920),
            height,
        )
    if surface == "bar":
        return (0, 0, width, max(1, round(height * 44 / 1200)))
    if surface == "menu_attached":
        return (0, 0, round(width * 0.44), round(height * 0.58))
    if surface == "menu_detached":
        return (
            round(width * 0.20),
            round(height * 0.42),
            round(width * 0.80),
            height,
        )
    if surface == "control_center":
        return (round(width * 0.58), 0, width, round(height * 0.96))
    if surface == "banner":
        return (round(width * 0.58), 0, width, round(height * 0.24))
    raise ValueError(f"unknown surface {surface!r}")


def _source_path(source_dir: Path, capture: str, scheme: str) -> Path:
    path = source_dir / f"t20-chrome-{capture}-{scheme}.png"
    if path.exists():
        return path
    # The committed baseline is the same v0.1.1 set; keep the source path
    # explicit even when a fresh capture directory was requested.
    fallback = CAPTURES / path.name
    if fallback.exists():
        return fallback
    raise FileNotFoundError(
        f"baseline source missing: {path} (and fallback {fallback})"
    )


def write_crops(source_dir: Path, outdir: Path) -> list[dict]:
    outdir.mkdir(parents=True, exist_ok=True)
    rows = []
    for surface, capture, label in SURFACES:
        for scheme in ("light", "dark"):
            source = _source_path(source_dir, capture, scheme)
            image = Image.open(source).convert("RGB")
            rect = _crop_rect(surface, image.width, image.height)
            crop = image.crop(rect)
            dest = outdir / f"t194-glass-baseline-{surface}-{scheme}.png"
            crop.save(dest, optimize=False)
            rows.append(
                {
                    "surface": surface,
                    "label": label,
                    "scheme": scheme,
                    "kind": "attached" if surface == "menu_attached"
                    else ("detached" if surface == "menu_detached" else "surface"),
                    "artifact": str(dest.relative_to(REPO)),
                    "sha256": sha256(dest),
                    "size_px": [crop.width, crop.height],
                    "source_artifact": str(source.relative_to(REPO))
                    if source.is_relative_to(REPO)
                    else str(source),
                    "source_sha256": sha256(source),
                    "crop_rect_px": list(rect),
                }
            )
    return rows


def _parse_stats(path: Path, label: str) -> dict:
    categories = {}
    pattern = re.compile(r": ([a-z][a-z ]+) \(" + re.escape(label) + r"\): (.*)$")
    token = re.compile(r"([a-zA-Z_][a-zA-Z0-9_-]*)=([^\s]+)")
    if not path.exists():
        return categories
    for line in path.read_text(errors="replace").splitlines():
        match = pattern.search(line)
        if not match:
            continue
        category = match.group(1).strip()
        categories.setdefault(category, {}).update(token.findall(match.group(2)))
    return categories


def _number(value):
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        try:
            return float(value)
        except ValueError:
            return value


def _derived(categories: dict) -> dict:
    render = categories.get("render stats", {})
    chrome = categories.get("chrome stats", {})
    material = categories.get("material stats", {})
    output = categories.get("output material stats", {})
    refresh = categories.get("refresh stats", {})
    degrade = categories.get("degrade stats", {})
    shell = categories.get("chrome stats", {})
    return {
        "frames_rendered": _number(render.get("frames_rendered")),
        "frames_skipped_no_damage": _number(render.get("frames_skipped_no_damage")),
        "direct_scanouts": _number(render.get("direct_scanouts")),
        "client_wakeups": _number(render.get("client_wakeups")),
        "chrome_shm_imports": _number(chrome.get("chrome_shm_imports")),
        "chrome_shm_bytes": _number(chrome.get("chrome_shm_bytes")),
        "chrome_dmabuf_imports": _number(chrome.get("chrome_dmabuf_imports")),
        "chrome_frame_callbacks": _number(chrome.get("chrome_frame_callbacks")),
        "material_frames": _number(output.get("material_frames")),
        "material_allocations": _number(output.get("material_allocations")),
        "material_region_px": _number(output.get("material_region_px")),
        "material_region": output.get("material_region"),
        "material_path": material.get("path"),
        "material_scene_allocations": _number(material.get("material_scene_allocations")),
        "material_panel_allocations": _number(material.get("material_panel_allocations")),
        "refresh_hz": _number(refresh.get("refresh_hz")),
        "refresh_source": refresh.get("source"),
        "interval_us": _number(refresh.get("interval_us")),
        "deadline_us": _number(refresh.get("deadline_us")),
        "tier": degrade.get("tier"),
        "tier_forced": _number(degrade.get("forced")),
        "transport_path": shell.get("path"),
        "transport_gpu": shell.get("gpu"),
        "transport_reason": shell.get("gpu_reason"),
        "shm_allocs": _number(shell.get("shm_allocs")),
        "shm_bytes": _number(shell.get("shm_bytes")),
        "copy_bytes": _number(shell.get("copy_bytes")),
        "dmabuf_commits": _number(shell.get("dmabuf_commits")),
        "dmabuf_bytes": _number(shell.get("dmabuf_bytes")),
        "damage_pixels": _number(shell.get("damage_pixels")),
        "commit_us_p95": _number(shell.get("commit_us_p95")),
        "frame_callback_us_p95": _number(shell.get("frame_callback_us_p95")),
    }


def _snapshot(compositor: Path, shell: Path, label: str) -> dict:
    comp = _parse_stats(compositor, label)
    shell_stats = _parse_stats(shell, label)
    derived = _derived(comp)
    for key, value in _derived(shell_stats).items():
        if value is not None:
            derived[key] = value
    return {"compositor": comp, "shell": shell_stats, "derived": derived}


def _delta(before: dict, after: dict) -> dict:
    out = {}
    for key, value in after.items():
        old = before.get(key)
        if isinstance(value, (int, float)) and isinstance(old, (int, float)):
            out[key] = value - old
    return out


def _counters() -> dict:
    compositor = BENCH / "compositor.txt"
    shell = BENCH / "shell.txt"
    snapshots = {
        label: _snapshot(compositor, shell, label)
        for label in ("SIGUSR1-2", "SIGUSR1-3", "SIGUSR1-4")
    }
    rows = {}
    for name, label, previous, scenario in (
        ("stationary", "SIGUSR1-3", "SIGUSR1-2", "idle (60 s, no pointer motion)"),
        ("moving", "SIGUSR1-4", "SIGUSR1-3", "Dock sweep (10 s of synthetic pointer motion)"),
    ):
        snap = snapshots[label]
        rows[name] = {
            "scenario": scenario,
            "snapshot": label,
            "previous_snapshot": previous,
            "source": {
                "compositor_log": str(compositor.relative_to(REPO)),
                "shell_log": str(shell.relative_to(REPO)),
            },
            "compositor": snap["compositor"],
            "shell": snap["shell"],
            "derived": snap["derived"],
            "delta_from_previous": _delta(
                snapshots[previous]["derived"], snap["derived"]
            ),
        }
    return rows


def write_backdrops(outdir: Path) -> list[dict]:
    """Record the stationary and moving backdrop evidence already in the tree."""
    rows = []
    for name, source, note in (
        (
            "stationary",
            CAPTURES / "t21-stable-dock-1x.png",
            "settled Dock/desktop, no pointer motion",
        ),
        (
            "moving",
            CAPTURES / "t21-region-motion-1x.png",
            "nested Dock sweep with the pointer moving",
        ),
        (
            "moving_window_dark",
            CAPTURES / "t21-native-dark-live-under-dock-2x.png",
            "native live-content check: an override-redirect test panel moved behind the Dock (dark)",
        ),
        (
            "moving_window_light",
            CAPTURES / "t21-native-light-live-under-dock-2x.png",
            "native live-content check: an override-redirect test panel moved behind the Dock (light)",
        ),
    ):
        if not source.exists():
            rows.append(
                {
                    "id": name,
                    "status": "missing",
                    "source": str(source.relative_to(REPO)),
                    "note": note,
                }
            )
            continue
        dest = outdir / f"t194-glass-baseline-{name}.png"
        shutil.copyfile(source, dest)
        rows.append(
            {
                "id": name,
                "status": "present",
                "artifact": str(dest.relative_to(REPO)),
                "sha256": sha256(dest),
                "source_artifact": str(source.relative_to(REPO)),
                "source_sha256": sha256(source),
                "note": note,
            }
        )
    return rows


def _host() -> dict:
    def run(*args):
        try:
            return subprocess.run(args, check=True, capture_output=True, text=True).stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return ""

    gpu = ""
    if shutil.which("lspci"):
        gpu = run("lspci", "-nnk")
        gpu = "\n".join(
            line for line in gpu.splitlines()
            if re.search(r"VGA compatible|3D controller|Display controller", line)
            or "Kernel driver in use" in line
        )
    return {
        "host": run("uname", "-srm"),
        "distro": run("sh", "-c", ". /etc/os-release 2>/dev/null && echo \"$PRETTY_NAME\""),
        "session_type": os.environ.get("XDG_SESSION_TYPE", ""),
        "current_desktop": os.environ.get("XDG_CURRENT_DESKTOP", ""),
        "wayland_display": os.environ.get("WAYLAND_DISPLAY", ""),
        "drm_nodes": sorted(p.name for p in Path("/dev/dri").glob("*")) if Path("/dev/dri").exists() else [],
        "gpu": gpu,
    }


def _open_status() -> dict | None:
    if not OPEN_TXT.exists():
        return None
    text = OPEN_TXT.read_text(errors="replace")
    status = {"artifact": str(OPEN_TXT.relative_to(REPO))}
    for key in ("verdict", "reason", "rerun", "command"):
        match = re.search(rf"^{key}: (.*)$", text, re.MULTILINE)
        if match:
            status[key] = match.group(1)
    return status


def _gaps(open_status: dict | None) -> list[dict]:
    gaps = [
        {
            "id": "notification_center_unmapped",
            "status": "unmapped",
            "reason": "no Notification Center shell surface is mapped; history and the "
            "compositor input action exist, but the reference stack/lower-panel fits are "
            "evidence only",
            "owner": "T204",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md",
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
            "id": "dark_banner_lower_panel",
            "status": "missing",
            "reason": "dark banner and dark Notification Center lower panel are uncovered "
            "as independent reference fits",
            "owner": "T205",
            "evidence": "docs/tasks/glass/plan/REFERENCE.md",
        },
    ]
    if open_status:
        gaps.append(
            {
                "id": "live_host_recapture",
                "status": "open",
                "reason": open_status.get("reason", "host capture did not run"),
                "owner": "T194/T207",
                "evidence": open_status.get("artifact"),
            }
        )
    return gaps


def build(source_dir: Path, outdir: Path) -> dict:
    surfaces = write_crops(source_dir, outdir)
    backdrops = write_backdrops(outdir)
    reference = REFERENCE_MANIFEST
    manifest = {
        "schema": 1,
        "generated_by": "scripts/t194-baseline.py",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "command": "python3 scripts/t194-baseline.py --write",
        "reference_manifest": {
            "path": str(reference.relative_to(REPO)),
            "sha256": sha256(reference) if reference.exists() else None,
        },
        "host": _host(),
        "capture_environment": {
            "backend": "nested",
            "output": "NESTED-1 1920x1200",
            "scale": 1,
            "refresh_hz": 144.0,
            "refresh_source": "override",
            "scheme": "dark",
            "material_tier": "full",
            "material_path": "gpu",
            "transport_path": "render-control+dmabuf",
            "source_capture": "v0.1.1 committed t20-chrome-* stills",
            "source_benchmark": "docs/captures/t21-presentation-bench.md",
            "note": "the nested refresh override is policy evidence; it is not a physical panel rate",
        },
        "surfaces": surfaces,
        "backdrops": backdrops,
        "counters": _counters(),
        "open_capture": _open_status(),
        "gaps": _gaps(_open_status()),
        "commands": {
            "baseline": "make glass-baseline-capture",
            "selftest": "make glass-selftest",
            "reference_verify": "make glass-reference-verify",
            "compare": "python3 scripts/t194-glass-compare.py compare --reference REF --candidate CAND --case CASE.json",
            "timing": "python3 scripts/t194-glass-compare.py timing --reference docs/captures/t194-glass-baseline.json --candidate CAND.json",
        },
    }
    return manifest


def _write_markdown(manifest: dict, path: Path) -> None:
    env = manifest["capture_environment"]
    lines = [
        "# T194 — Dragonfruit glass baseline",
        "",
        f"Generated by `{manifest['generated_by']}`; reference manifest "
        f"`{manifest['reference_manifest']['path']}`.",
        "",
        "This is the starting Dragonfruit appearance and timing record. The stills "
        "are derived from the committed v0.1.1 nested captures; a live host recapture "
        "is recorded separately in the OPEN artifact when it could not run.",
        "",
        "## Environment",
        "",
        f"- backend: `{env['backend']}`",
        f"- output: `{env['output']}`, scale `{env['scale']}`",
        f"- refresh: `{env['refresh_hz']:.0f} Hz` ({env['refresh_source']}); this is a "
        "nested policy override, not a physical panel rate",
        f"- scheme: `{env['scheme']}`; material tier `{env['material_tier']}` "
        f"(`{env['material_path']}`), transport `{env['transport_path']}`",
        f"- host: {manifest['host']['host']} · {manifest['host']['distro']}",
        "",
        "## Surface baseline",
        "",
        "| Surface | Scheme | Kind | Artifact | Source |",
        "|---|---|---|---|---|",
    ]
    for row in manifest["surfaces"]:
        lines.append(
            f"| {row['surface']} | {row['scheme']} | {row['kind']} | "
            f"`{row['artifact']}` | `{row['source_artifact']}` |"
        )
    lines += [
        "",
        "## Backdrop evidence",
        "",
        "| Backdrop | Status | Artifact | Note |",
        "|---|---|---|---|",
    ]
    for row in manifest.get("backdrops", []):
        artifact = f"`{row['artifact']}`" if row.get("artifact") else "—"
        lines.append(f"| {row['id']} | {row['status']} | {artifact} | {row['note']} |")
    lines += [
        "",
        "## Counters",
        "",
        "| Scenario | frames (Δ) | skipped (Δ) | material alloc (Δ) | region px | chrome shm/dmabuf | transport | p95 commit | p95 callback |",
        "|---|---:|---:|---:|---:|---|---|---:|---:|",
    ]
    for name in ("stationary", "moving"):
        row = manifest["counters"][name]
        d = row["derived"]
        delta = row.get("delta_from_previous", {})
        lines.append(
            f"| {name}: {row['scenario']} | {d['frames_rendered']} "
            f"({delta.get('frames_rendered')}) | "
            f"{d['frames_skipped_no_damage']} ({delta.get('frames_skipped_no_damage')}) | "
            f"{d['material_allocations']} ({delta.get('material_allocations')}) | "
            f"{d['material_region_px']} | "
            f"{d['chrome_shm_imports']}/{d['chrome_dmabuf_imports']} | "
            f"{d['transport_path']} | {d['commit_us_p95']} µs | "
            f"{d['frame_callback_us_p95']} µs |"
        )
    lines += [
        "",
        "Raw snapshots: `docs/captures/t21-presentation-bench-raw/main-144-full/"
        "{compositor,shell}.txt` (`SIGUSR1-3` stationary idle, `SIGUSR1-4` moving Dock sweep).",
        "",
        "## Gaps",
        "",
    ]
    for gap in manifest["gaps"]:
        lines.append(
            f"- **{gap['id']}** — {gap['status']}; owner {gap['owner']}. "
            f"{gap['reason']} ({gap['evidence']})"
        )
    lines += [
        "",
        "## Commands",
        "",
        "```bash",
        "make glass-baseline-capture   # recapture on a live host, then rewrite this baseline",
        "make glass-selftest           # deterministic colour/geometry/MTF/motion regression self-test",
        "make glass-reference-verify   # optional reference checkout hash/revision check",
        "```",
        "",
    ]
    path.write_text("\n".join(lines))


def check() -> int:
    if not OUT_JSON.exists():
        print(f"t194-baseline: missing {OUT_JSON}", file=sys.stderr)
        return 2
    manifest = json.loads(OUT_JSON.read_text())
    problems = []
    for row in manifest.get("surfaces", []) + manifest.get("backdrops", []):
        if row.get("status", "present") != "present":
            continue
        path = REPO / row["artifact"]
        if not path.exists():
            problems.append(f"missing artifact {row['artifact']}")
        elif sha256(path) != row["sha256"]:
            problems.append(f"artifact hash mismatch {row['artifact']}")
    for name, row in manifest.get("counters", {}).items():
        if not row.get("derived", {}).get("frames_rendered"):
            problems.append(f"counter row {name} has no frames_rendered")
    if problems:
        print("t194-baseline: FAIL", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 2
    print(
        f"t194-baseline: PASS ({len(manifest['surfaces'])} surface crops, "
        f"{len(manifest['gaps'])} gaps)"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source-dir", type=Path, default=CAPTURES)
    parser.add_argument("--outdir", type=Path, default=CAPTURES)
    args = parser.parse_args()
    if args.check:
        return check()
    if not args.write:
        parser.error("choose --write or --check")
    manifest = build(args.source_dir, args.outdir)
    OUT_JSON.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    _write_markdown(manifest, OUT_MD)
    print(f"wrote {OUT_JSON} and {OUT_MD}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
