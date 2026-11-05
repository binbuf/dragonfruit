#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Write the T-21.4b native benchmark row matrix as an explicit OPEN artifact.

The nested benchmark (``scripts/t21-presentation-bench.sh``) can measure the
chrome transport, refresh policy, and material tiers on any host Wayland
session. It cannot measure native DRM presentation: the live Dragonfruit
session already owns DRM master, and restarting that compositor would end the
session the benchmark is running in. This script records every native row the
parent spec asks for, with the blocker for each row, instead of silently
dropping the row or substituting nested numbers.

The output is ``docs/captures/t21-presentation-bench.open.txt`` by default.
The same script also records a live-session diagnostic snapshot when the
benchmark passed one; that snapshot is labelled as an observation, not a
controlled scenario row.
"""
import argparse
import os
import re
import subprocess
import sys
from datetime import datetime, timezone


NATIVE_ROWS = (
    # (row, mode/resolution, tier, extra blocker)
    ("idle", "3840x2160@143.999", "Full", "needs a controlled native session"),
    ("dock-sweep", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("menubar-open/close", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("dock-popover-open/close", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("app-launch", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("mission-control", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("fullscreen", "3840x2160@143.999", "Full", "needs the synthetic-input harness"),
    ("mode", "3840x2160@143.999", "Full", "live session owns DRM master"),
    ("mode", "3840x2160@60", "Full", "live session owns DRM master"),
    ("mode", "1920x1080@60", "Full", "live session owns DRM master"),
    ("mode", "1920x1080@144", "Full", "live session owns DRM master"),
    ("mixed outputs", "60 Hz + 144 Hz", "Full", "only one connected DRM output (DP-1)"),
    ("material tier", "3840x2160@143.999", "Full", "live session tier is not remotely controllable"),
    ("material tier", "3840x2160@143.999", "Reduced", "live session tier is not remotely controllable"),
    ("material tier", "3840x2160@143.999", "Minimal", "live session tier is not remotely controllable"),
    ("missed deadlines", "3840x2160@143.999", "Full", "live counters recorded below, controlled scenario row needs a free seat"),
    ("GPU timer queries", "3840x2160@143.999", "Full", "live release compositor was not started with DRAGONFRUIT_GPU_TIMING=1"),
)


def run(command):
    try:
        return subprocess.run(
            command,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        ).stdout.strip()
    except (FileNotFoundError, OSError):
        return ""


def read(path):
    if not path:
        return ""
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read().rstrip()
    except OSError:
        return ""


def connected_outputs():
    outputs = []
    for entry in sorted(os.listdir("/sys/class/drm")) if os.path.isdir("/sys/class/drm") else []:
        if "-" not in entry:
            continue
        status = os.path.join("/sys/class/drm", entry, "status")
        if os.path.isfile(status):
            try:
                if read(status) == "connected":
                    outputs.append(entry)
            except OSError:
                pass
    return outputs


def first_modes():
    modes = []
    for entry in sorted(os.listdir("/sys/class/drm")) if os.path.isdir("/sys/class/drm") else []:
        path = os.path.join("/sys/class/drm", entry, "modes")
        if os.path.isfile(path):
            first = read(path).splitlines()
            if first:
                modes.append(first[0])
    return sorted(set(modes))


def seat_probe():
    script = os.path.join(os.path.dirname(__file__), "drm-seat-probe.py")
    if not os.path.isfile(script):
        return ""
    return run([sys.executable, script])


def parse_presentation(diagnostics):
    """Return the last live `presentation stats` fields, if present."""
    line = ""
    for candidate in diagnostics.splitlines():
        if "presentation stats" in candidate:
            line = candidate
    if not line:
        return {}
    fields = {}
    for token in line.split(":", 2)[-1].split():
        key, sep, value = token.partition("=")
        if sep:
            fields[key] = value
    return fields


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out",
        default="docs/captures/t21-presentation-bench.open.txt",
    )
    parser.add_argument(
        "--reason",
        default=(
            "the live Dragonfruit session owns DRM master and no free logind "
            "seat/dedicated development user is available; restarting the DRM "
            "compositor would end the session running the benchmark"
        ),
    )
    parser.add_argument("--native-diagnostics", default="")
    parser.add_argument("--command", default="make t21-presentation-bench")
    args = parser.parse_args()

    diagnostics = read(args.native_diagnostics)
    presentation = parse_presentation(diagnostics)
    outputs = connected_outputs()
    modes = first_modes()
    gpu = run(["sh", "-c", "lspci -nnk 2>/dev/null | grep -A3 -E 'VGA compatible|3D controller|Display controller' | tr '\\n' ' '"])

    lines = []
    add = lines.append
    add("T-21.4b presentation benchmark — native rows (T192c)")
    add("====================================================")
    add("")
    add(f"timestamp: {datetime.now(timezone.utc).isoformat(timespec='seconds')}")
    add("verdict: OPEN")
    add(f"reason: {args.reason}")
    add("")
    add("hardware:")
    add(f"  host: {run(['uname', '-srm'])}")
    add(f"  session: XDG_SESSION_TYPE={os.environ.get('XDG_SESSION_TYPE', '<unset>')} "
        f"XDG_CURRENT_DESKTOP={os.environ.get('XDG_CURRENT_DESKTOP', '<unset>')} "
        f"WAYLAND_DISPLAY={os.environ.get('WAYLAND_DISPLAY', '<unset>')} "
        f"XDG_VTNR={os.environ.get('XDG_VTNR', '<unset>')}")
    add(f"  GPU: {gpu or 'unknown'}")
    add(f"  DRM nodes: {' '.join(sorted(os.listdir('/dev/dri'))) if os.path.isdir('/dev/dri') else 'none'}")
    add(f"  connected outputs: {' '.join(outputs) if outputs else 'none'}")
    add(f"  modes: {' '.join(modes) if modes else 'none'}")
    probe = seat_probe()
    if probe:
        add(f"  seat probe: {probe}")
    add("")
    add("Native row matrix (every unavailable row is OPEN, not omitted):")
    add("")
    add("| row | mode/resolution | tier | status | reason |")
    add("|---|---|---|---|---|")
    for row, mode, tier, blocker in NATIVE_ROWS:
        add(f"| {row} | {mode} | {tier} | OPEN | {blocker}; {args.reason} |")
    add("")
    if presentation:
        add("Live-session native observation (cumulative counters, not a controlled scenario row):")
        add("")
        add("```text")
        add(
            "presentation stats: output={output} refresh_hz={refresh_hz} "
            "budget_us={budget_us} samples={samples} missed={missed} "
            "p50_us={p50_us} p95_us={p95_us} p99_us={p99_us}".format(
                output=presentation.get("output", "?"),
                refresh_hz=presentation.get("refresh_hz", "?"),
                budget_us=presentation.get("budget_us", "?"),
                samples=presentation.get("samples", "?"),
                missed=presentation.get("missed", "?"),
                p50_us=presentation.get("p50_us", "?"),
                p95_us=presentation.get("p95_us", "?"),
                p99_us=presentation.get("p99_us", "?"),
            )
        )
        add("```")
        add("")
        add(
            "The live session is the dogfooding desktop and its agent UI is an "
            "active client, so this snapshot is not a controlled idle or Dock "
            "sweep. It proves the native DRM rail is presenting and that "
            "`presentation stats` is available; it does not replace the "
            "free-seat scenario rows above."
        )
        add("")
    if diagnostics:
        add("Raw native diagnostics recorded by the benchmark:")
        add("")
        add("```text")
        add(diagnostics)
        add("```")
        add("")
    add("Rerun on a free DRM seat (the dedicated development user and free")
    add("logind seat are the prerequisites; see `make second-vt-validation`):")
    add("")
    add("```bash")
    add("# one-time, as root:")
    add("sudo useradd -m dfdev && sudo passwd dfdev")
    add("# from the free seat / second VT:")
    add("make native")
    add("make t21-presentation-bench")
    add("# attach the DRM `presentation stats` lines and replace this OPEN file")
    add("```")
    add("")
    add(f"Benchmark command used for the nested rows: `{args.command}`")
    add("")

    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines).rstrip() + "\n")
    print(f"t21-presentation-native-rows: wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
