#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Render the T-21.1a chrome baseline report from a nested demo log.

Called by ``scripts/t21-chrome-baseline.sh``. The demo log carries the shell's
``chrome stats`` / ``chrome surface`` lines and the compositor's ``chrome
stats``, ``frame timing``, ``presentation stats``, ``material stats`` and
``degrade stats`` blocks. The driver log maps each numbered ``SIGUSR1-<n>``
snapshot to a scenario; the report prints per-scenario deltas plus the raw
lines so the numbers stay auditable.
"""
import argparse
import re
import sys

SCENARIO_RE = re.compile(
    r"t21-chrome-baseline: scenario (?P<seq>\d+) (?P<name>\S+) elapsed=(?P<elapsed>[0-9.]+)"
)
SHELL_STATS_RE = re.compile(r"dragonfruit-shell: chrome stats \((?P<label>[^)]+)\): (?P<body>.*)")
SHELL_SURFACE_RE = re.compile(
    r"dragonfruit-shell: chrome surface \((?P<label>[^)]+)\): (?P<body>.*)"
)
COMP_RE = re.compile(
    r"dragonfruit-compositor: (?P<kind>[a-z ]+) stats \((?P<label>[^)]+)\): (?P<body>.*)"
)
COMP_FRAME_RE = re.compile(
    r"dragonfruit-compositor: frame timing \((?P<label>[^)]+)\): (?P<body>.*)"
)


def fields(body):
    out = {}
    for token in body.split():
        key, sep, value = token.partition("=")
        if sep:
            out[key] = value
    return out


def as_int(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def parse(log_path):
    shell = {}
    surfaces = {}
    comp = {}
    raw = []
    with open(log_path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            match = SHELL_STATS_RE.match(line)
            if match:
                label = match.group("label")
                shell.setdefault(label, {}).update(fields(match.group("body")))
                raw.append(line)
                continue
            match = SHELL_SURFACE_RE.match(line)
            if match:
                label = match.group("label")
                body = fields(match.group("body"))
                name = body.get("surface", "unknown")
                surfaces.setdefault(label, {})[name] = body
                raw.append(line)
                continue
            match = COMP_FRAME_RE.match(line)
            if match:
                label = match.group("label")
                entry = comp.setdefault(label, {})
                entry.setdefault("frame timing", []).append(fields(match.group("body")))
                raw.append(line)
                continue
            match = COMP_RE.match(line)
            if match:
                kind = match.group("kind")
                label = match.group("label")
                entry = comp.setdefault(label, {})
                entry.setdefault(kind, []).append(fields(match.group("body")))
                raw.append(line)
    return shell, surfaces, comp, raw


def parse_scenarios(driver_log):
    scenarios = []
    with open(driver_log, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            match = SCENARIO_RE.search(line)
            if match:
                scenarios.append(
                    {
                        "seq": int(match.group("seq")),
                        "name": match.group("name"),
                        "elapsed": float(match.group("elapsed")),
                        "label": f"SIGUSR1-{match.group('seq')}",
                    }
                )
    return scenarios


def delta(curr, prev, key):
    return as_int(curr.get(key)) - as_int(prev.get(key)) if prev else as_int(curr.get(key))


def comp_delta(comp, label, prev_label, kind, key):
    curr = comp.get(label, {}).get(kind, [{}])
    prev = comp.get(prev_label, {}).get(kind, [{}]) if prev_label else [{}]
    current = curr[0] if curr else {}
    previous = prev[0] if prev else {}
    return as_int(current.get(key)) - as_int(previous.get(key))


def comp_line(comp, label, kind):
    entries = comp.get(label, {}).get(kind, [])
    if not entries:
        return None
    return " ".join(f"{key}={value}" for key, value in entries[0].items())


def fmt_float(value):
    return f"{value:.3f}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--log", required=True)
    parser.add_argument("--shell-log", default="")
    parser.add_argument("--driver-log", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--host-info", required=True)
    parser.add_argument("--native", default="")
    parser.add_argument("--visual", default="")
    parser.add_argument("--command", default="make t21-chrome-baseline")
    args = parser.parse_args()

    shell, surfaces, comp, raw = parse(args.log)
    if args.shell_log:
        shell_lines, surface_lines, _, shell_raw = parse(args.shell_log)
        shell.update(shell_lines)
        surfaces.update(surface_lines)
        raw.extend(shell_raw)
    scenarios = parse_scenarios(args.driver_log)
    if not scenarios:
        print("t21-chrome-baseline-report: no scenario snapshots found", file=sys.stderr)
        return 1

    lines = []
    add = lines.append
    add("# T-21.1a chrome transport baseline")
    add("")
    add("Generated by `scripts/t21-chrome-baseline.sh` (`make t21-chrome-baseline`).")
    add("The run is **nested**: the shell is a Wayland client of the nested")
    add("compositor, and the nested output has no hardware vblank. The native")
    add("half is recorded separately below; a nested run is not a native")
    add("presentation measurement.")
    add("")
    add(f"- command: `{args.command}`")
    add("- transport under test: `offscreen QPA -> grabWindow() -> QImage -> fresh wl_shm buffer -> wl_surface.commit`")
    add("- opt-in switches: `DRAGONFRUIT_CHROME_STATS=1`, `QSG_INFO=1`")
    add("")
    add("## Host, backend, and driver")
    add("")
    with open(args.host_info, encoding="utf-8", errors="replace") as handle:
        add("```text")
        add(handle.read().rstrip())
        add("```")
    add("")

    add("## Scenario snapshots")
    add("")
    add("Every snapshot is a pair of `SIGUSR1` dumps (shell and compositor).")
    add("Counters below are **deltas since the previous snapshot**; the shell")
    add("`commit_us_*` percentiles are windowed to the interval since the previous")
    add("dump (the timing rings clear on emit), so they describe this scenario.")
    add("Compositor `frame timing` percentiles are the compositor's current")
    add("8192-sample window, not a per-scenario delta; the raw line is quoted.")
    add("")
    previous_label = None
    for scenario in scenarios:
        label = scenario["label"]
        prev = previous_label
        if label not in shell or label not in comp:
            add(f"### {scenario['name']} ({label})")
            add("")
            add(
                "snapshot missing — one process did not emit this numbered"
                " dump; the next available snapshot covers the interval."
            )
            add("")
            continue
        current_shell = shell.get(label, {})
        previous_shell = shell.get(prev, {}) if prev else {}
        current_comp = comp.get(label, {})
        add(f"### {scenario['name']} ({label}, {fmt_float(scenario['elapsed'])} s)")
        add("")
        add(
            "shell delta: "
            + " ".join(
                f"{key}={delta(current_shell, previous_shell, key)}"
                for key in (
                    "scene_frames",
                    "commits",
                    "grab_renders",
                    "shm_allocs",
                    "shm_bytes",
                    "frame_callbacks",
                )
            )
        )
        elapsed = max(scenario["elapsed"], 0.001)
        add(
            f"cadence: scene_frames/s={delta(current_shell, previous_shell, 'scene_frames') / elapsed:.1f} "
            f"commits/s={delta(current_shell, previous_shell, 'commits') / elapsed:.1f}"
        )
        add(
            "shell timing: "
            + " ".join(
                f"{key}={current_shell.get(key, '0')}"
                for key in (
                    "commit_us_samples",
                    "commit_us_p50",
                    "commit_us_p95",
                    "commit_us_p99",
                    "commit_us_max",
                    "frame_callback_us_samples",
                    "frame_callback_us_p50",
                    "frame_callback_us_p95",
                    "frame_callback_us_p99",
                )
            )
        )
        add(
            "compositor delta: "
            + " ".join(
                f"{key}={comp_delta(comp, label, prev, 'chrome', key)}"
                for key in (
                    "chrome_shm_imports",
                    "chrome_shm_bytes",
                    "chrome_dmabuf_imports",
                    "chrome_frame_callbacks",
                )
            )
        )
        add(
            "compositor render delta: "
            + " ".join(
                f"{key}={comp_delta(comp, label, prev, 'render', key)}"
                for key in (
                    "frames_rendered",
                    "frames_skipped_no_damage",
                    "direct_scanouts",
                    "client_wakeups",
                    "animation_frames_stepped",
                )
            )
        )
        frame = comp_line(comp, label, "frame timing")
        if frame:
            add(f"frame timing ({label}): {frame}")
        presentations = comp.get(label, {}).get("presentation", [])
        if presentations:
            for entry in presentations:
                add(
                    f"presentation stats ({label}): "
                    + " ".join(f"{key}={value}" for key, value in entry.items())
                )
        else:
            add(
                f"presentation stats ({label}): none — the nested backend has no "
                "hardware vblank; DRM `frame_finish` owns this line."
            )
        for kind in ("material", "degrade", "animation", "scanout"):
            line = comp_line(comp, label, kind)
            if line:
                add(f"{kind} stats ({label}): {line}")
        current_surfaces = surfaces.get(label, {})
        previous_surfaces = surfaces.get(prev, {}) if prev else {}
        if current_surfaces:
            add("per-surface:")
            for name in sorted(current_surfaces):
                body = current_surfaces[name]
                old = previous_surfaces.get(name, {})
                add(
                    "  "
                    + " ".join(
                        [
                            f"surface={name}",
                            f"logical_size={body.get('logical_size', '?')}",
                            f"buffer_size={body.get('buffer_size', '?')}",
                            f"buffer_scale={body.get('buffer_scale', '?')}",
                            f"commits_delta={delta(body, old, 'commits')}",
                            f"shm_allocs_delta={delta(body, old, 'shm_allocs')}",
                            f"shm_bytes_delta={delta(body, old, 'shm_bytes')}",
                            f"commit_us_p50={body.get('commit_us_p50', '0')}",
                            f"commit_us_p95={body.get('commit_us_p95', '0')}",
                            f"commit_us_p99={body.get('commit_us_p99', '0')}",
                            f"frame_callback_us_samples={body.get('frame_callback_us_samples', '0')}",
                        ]
                    )
                )
        else:
            add("per-surface: none recorded")
        add("")
        previous_label = label

    add("## Percentiles and idle evidence")
    add("")
    add("| scenario | commit_us p50 | p95 | p99 | frame timing p50 | p95 | p99 |")
    add("|---|---|---|---|---|---|---|")
    for scenario in scenarios:
        label = scenario["label"]
        current_shell = shell.get(label, {})
        frame = comp.get(label, {}).get("frame timing", [{}])[0]
        add(
            f"| {scenario['name']} | {current_shell.get('commit_us_p50', '0')} | "
            f"{current_shell.get('commit_us_p95', '0')} | "
            f"{current_shell.get('commit_us_p99', '0')} | "
            f"{frame.get('p50_us', '0')} | {frame.get('p95_us', '0')} | "
            f"{frame.get('p99_us', '0')} |"
        )
    add("")
    idle = next((scenario for scenario in scenarios if scenario["name"] == "idle"), None)
    if idle:
        prev = f"SIGUSR1-{idle['seq'] - 1}" if idle["seq"] > 1 else None
        current = shell.get(idle["label"], {})
        old = shell.get(prev, {}) if prev else {}
        idle_frame = comp.get(idle["label"], {}).get("frame timing", [{}])[0]
        idle_render = comp.get(idle["label"], {}).get("render", [{}])[0]
        idle_prev_render = comp.get(prev, {}).get("render", [{}])[0] if prev else {}
        add("Idle zero-wakeup evidence (the `idle` snapshot):")
        add("")
        add(
            "```text\n"
            f"shell scene_frames_delta={delta(current, old, 'scene_frames')} "
            f"commits_delta={delta(current, old, 'commits')} "
            f"grab_renders_delta={delta(current, old, 'grab_renders')}\n"
            f"compositor frames_rendered_delta="
            f"{as_int(idle_render.get('frames_rendered')) - as_int(idle_prev_render.get('frames_rendered'))} "
            f"client_wakeups_delta="
            f"{as_int(idle_render.get('client_wakeups')) - as_int(idle_prev_render.get('client_wakeups'))} "
            f"animation_frames_stepped_delta="
            f"{as_int(idle_render.get('animation_frames_stepped')) - as_int(idle_prev_render.get('animation_frames_stepped'))}\n"
            f"frame timing: {comp_line(comp, idle['label'], 'frame timing') or 'none'}\n"
            "```"
        )
        zero = (
            delta(current, old, "scene_frames") == 0
            and delta(current, old, "commits") == 0
            and as_int(idle_render.get("frames_rendered"))
            == as_int(idle_prev_render.get("frames_rendered"))
            and as_int(idle_render.get("client_wakeups"))
            == as_int(idle_prev_render.get("client_wakeups"))
        )
        add("")
        add(
            "Verdict: **zero-wakeup confirmed** — the idle interval produced no"
            " shell scene frames, no chrome commits, no compositor frames, and"
            " no client wakeups."
            if zero
            else "Verdict: **not zero-wakeup** — see the deltas above; the idle"
            " interval still produced work on this host."
        )
    add("")
    add("## Raw lines")
    add("")
    add("```text")
    for line in raw:
        add(line)
    add("```")
    add("")
    add("## Native half")
    add("")
    if args.native:
        with open(args.native, encoding="utf-8", errors="replace") as handle:
            add("```text")
            add(handle.read().rstrip())
            add("```")
    else:
        add(
            "OPEN — no scenario-driven native capture was possible in this run."
            " The host is already a native Dragonfruit session, but its running"
            " compositor is the installed release build without the T-21.1a"
            " counters and without `DRAGONFRUIT_SYNTHETIC_INPUT`; restarting it"
            " would end the session this task runs in. The nested report above"
            " is therefore the transport evidence; the native `presentation"
            " stats` / `chrome stats` half remains OPEN for the hardware rail."
        )
    add("")
    if args.visual:
        add("## Live visual check")
        add("")
        add("Captured through the trusted compositor screenshot path")
        add("(`Cmd+Shift+3` -> `df_toplevel_manager.capture_screenshot`), not a")
        add("screencopy client:")
        add("")
        for path in args.visual.split(","):
            if path:
                add(f"- `{path}`")
        add("")
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines).rstrip() + "\n")
    print(f"t21-chrome-baseline-report: wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
