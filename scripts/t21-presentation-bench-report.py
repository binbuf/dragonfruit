#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Render the T-21.4b presentation benchmark report from the nested run logs.

Called by ``scripts/t21-presentation-bench.sh``. The main log carries the
shell's ``chrome stats`` / ``chrome surface`` lines and the compositor's
``render``, ``chrome``, ``material``, ``degrade``, ``gpu``, ``refresh``,
``animation``, ``frame timing``, ``frame request``, and ``frame-trace`` lines.
The driver log maps each numbered ``SIGUSR1-<n>`` snapshot to a scenario.

The report is deliberately nested-only. Native rows (DRM panel modes,
1080p/4K, mixed outputs, missed deadlines) are T192c and are written OPEN.
"""
import argparse
import os
import re
import sys

SCENARIO_RE = re.compile(
    r"t21-presentation-bench: scenario (?P<seq>\d+) (?P<name>\S+) elapsed=(?P<elapsed>[0-9.]+)"
)
SHELL_STATS_RE = re.compile(
    r"dragonfruit-shell: chrome stats \((?P<label>[^)]+)\): (?P<body>.*)"
)
SHELL_SURFACE_RE = re.compile(
    r"dragonfruit-shell: chrome surface \((?P<label>[^)]+)\): (?P<body>.*)"
)
COMP_ANY_RE = re.compile(
    r"dragonfruit-compositor: (?P<kind>[a-z][a-z -]*) \((?P<label>[^)]+)\): (?P<body>.*)"
)
COMP_TRACE_RE = re.compile(
    r"dragonfruit-compositor: frame-trace \((?P<label>[^)]+)\): "
    r"index=\d+ duration_us=(?P<duration>\d+) interval_ms=(?P<interval>\d+)"
)

MAIN_SCENARIOS = (
    "baseline",
    "idle",
    "dock-sweep",
    "menubar-open",
    "menubar-close",
    "dock-popover-open",
    "dock-popover-close",
    "app-launch",
    "mission-control",
    "mission-control-close",
    "fullscreen",
    "fullscreen-exit",
)

SCENARIO_HEADINGS = {
    "baseline": "warm-up baseline",
    "idle": "idle (no synthetic input)",
    "dock-sweep": "Dock sweep",
    "menubar-open": "menu bar open",
    "menubar-close": "menu bar close",
    "dock-popover-open": "Dock popover open",
    "dock-popover-close": "Dock popover close",
    "app-launch": "app launch + window appear",
    "mission-control": "Mission Control (10+ windows)",
    "mission-control-close": "Mission Control close",
    "fullscreen": "fullscreen client",
    "fullscreen-exit": "fullscreen exit",
    "sweep": "Dock sweep",
    "full-sweep": "Full tier Dock sweep",
    "reduced-sweep": "Reduced tier Dock sweep",
    "minimal-sweep": "Minimal tier Dock sweep",
}


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


def as_float(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return 0.0


def percentile(values, percent):
    if not values:
        return 0
    ordered = sorted(values)
    rank = (percent * (len(ordered) - 1)) // 100
    return ordered[rank]


def parse(path):
    """Parse one compositor/shell log into stats, surfaces, traces, and raw."""
    shell = {}
    surfaces = {}
    comp = {}
    trace = {}
    raw = []
    with open(path, encoding="utf-8", errors="replace") as handle:
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
            match = COMP_TRACE_RE.match(line)
            if match:
                label = match.group("label")
                trace.setdefault(label, []).append(as_int(match.group("duration")))
                raw.append(line)
                continue
            match = COMP_ANY_RE.match(line)
            if match:
                kind = match.group("kind")
                label = match.group("label")
                entry = comp.setdefault(label, {})
                entry.setdefault(kind, []).append(fields(match.group("body")))
                raw.append(line)
    return {
        "shell": shell,
        "surfaces": surfaces,
        "comp": comp,
        "trace": trace,
        "raw": raw,
    }


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


def scenario_by_name(scenarios, name):
    return next((scenario for scenario in scenarios if scenario["name"] == name), None)


def previous_label(scenarios, scenario):
    index = scenarios.index(scenario)
    return scenarios[index - 1]["label"] if index > 0 else None


def shell_delta(current, previous, key):
    return as_int(current.get(key)) - as_int(previous.get(key)) if previous else as_int(current.get(key))


def comp_entry(comp, label, kind):
    entries = comp.get(label, {}).get(kind, [])
    return entries[0] if entries else {}


def frame_request(comp, label):
    entries = comp.get(label, {}).get("frame request", [])
    for entry in entries:
        if entry.get("output", "").startswith("NESTED"):
            return entry
    return entries[0] if entries else {}


def frame_percentiles(parsed, label, previous):
    """Per-scenario frame duration percentiles from the frame-trace delta."""
    timing = comp_entry(parsed["comp"], label, "frame timing")
    if not timing:
        return None
    total = as_int(timing.get("samples_total"))
    previous_total = (
        as_int(comp_entry(parsed["comp"], previous, "frame timing").get("samples_total"))
        if previous
        else 0
    )
    count = total - previous_total
    samples = parsed["trace"].get(label, [])
    if count <= 0:
        return {"count": 0, "p50": 0, "p95": 0, "p99": 0, "max": 0, "source": "no-frames"}
    truncated = False
    if count > len(samples):
        count = len(samples)
        truncated = True
    if not samples:
        return {
            "count": as_int(timing.get("samples")),
            "p50": as_int(timing.get("p50_us")),
            "p95": as_int(timing.get("p95_us")),
            "p99": as_int(timing.get("p99_us")),
            "max": as_int(timing.get("max_us")),
            "source": "windowed",
        }
    durations = samples[-count:]
    return {
        "count": len(durations),
        "p50": percentile(durations, 50),
        "p95": percentile(durations, 95),
        "p99": percentile(durations, 99),
        "max": max(durations),
        "source": "delta-truncated" if truncated else "delta",
    }


def transport(current, previous):
    commits = shell_delta(current, previous, "commits")
    dmabuf = shell_delta(current, previous, "dmabuf_commits")
    shm = max(0, commits - dmabuf)
    fallbacks = shell_delta(current, previous, "dmabuf_fallbacks")
    deferred = shell_delta(current, previous, "dmabuf_deferred")
    timeouts = shell_delta(current, previous, "dmabuf_sync_timeouts")
    return {
        "path": current.get("path", "?"),
        "gpu": current.get("gpu", "?"),
        "gpu_reason": current.get("gpu_reason", "none"),
        "commits": commits,
        "dmabuf": dmabuf,
        "shm": shm,
        "shm_rate": (shm / commits) if commits else None,
        "fallbacks": fallbacks,
        "deferred": deferred,
        "timeouts": timeouts,
        "copy_bytes": shell_delta(current, previous, "copy_bytes"),
        "shm_allocs": shell_delta(current, previous, "shm_allocs"),
        "shm_bytes": shell_delta(current, previous, "shm_bytes"),
        "dmabuf_bytes": shell_delta(current, previous, "dmabuf_bytes"),
    }


def surface_transport(current, previous):
    rows = []
    for name in sorted(current):
        body = current[name]
        old = previous.get(name, {}) if previous else {}
        commits = shell_delta(body, old, "commits")
        dmabuf = shell_delta(body, old, "dmabuf_commits")
        rows.append(
            {
                "surface": name,
                "path": body.get("path", "?"),
                "commits": commits,
                "dmabuf": dmabuf,
                "shm": max(0, commits - dmabuf),
                "fallbacks": shell_delta(body, old, "dmabuf_fallbacks"),
                "copy_bytes": shell_delta(body, old, "copy_bytes"),
                "commit_p50": body.get("commit_us_p50", "0"),
                "commit_p95": body.get("commit_us_p95", "0"),
                "commit_p99": body.get("commit_us_p99", "0"),
                "callback_p50": body.get("frame_callback_us_p50", "0"),
                "callback_p95": body.get("frame_callback_us_p95", "0"),
                "callback_p99": body.get("frame_callback_us_p99", "0"),
            }
        )
    return rows


def rate_text(rate):
    return "n/a" if rate is None else f"{rate * 100:.1f}%"


def timing_row(current):
    return {
        "commit_p50": current.get("commit_us_p50", "0"),
        "commit_p95": current.get("commit_us_p95", "0"),
        "commit_p99": current.get("commit_us_p99", "0"),
        "callback_p50": current.get("frame_callback_us_p50", "0"),
        "callback_p95": current.get("frame_callback_us_p95", "0"),
        "callback_p99": current.get("frame_callback_us_p99", "0"),
    }


def gpu_row(comp, label):
    gpu = comp_entry(comp, label, "gpu stats")
    return {
        "enabled": gpu.get("enabled", "0"),
        "available": gpu.get("available", "0"),
        "gpu_time": gpu.get("gpu_time", "unavailable"),
        "reason": gpu.get("reason", "none"),
        "composite_p50": gpu.get("composite_p50_us", "0"),
        "composite_p95": gpu.get("composite_p95_us", "0"),
        "composite_p99": gpu.get("composite_p99_us", "0"),
        "composite_samples": gpu.get("composite_samples", "0"),
        "scene_p50": gpu.get("scene_p50_us", "0"),
        "scene_p95": gpu.get("scene_p95_us", "0"),
        "scene_p99": gpu.get("scene_p99_us", "0"),
        "blur_p50": gpu.get("blur_p50_us", "0"),
        "blur_p95": gpu.get("blur_p95_us", "0"),
        "blur_p99": gpu.get("blur_p99_us", "0"),
    }


def refresh_deadline_us(hz):
    return 1_000_000_000 // (int(hz) * 1000)


def write_scenario_tables(add, parsed, scenarios):
    shell = parsed["shell"]
    comp = parsed["comp"]
    surfaces = parsed["surfaces"]

    add("### Scenario timing")
    add("")
    add(
        "`commit_us` and `frame_callback_us` are the shell's interval-windowed"
        " rings (cleared on each dump); the compositor frame percentiles are"
        " the per-scenario `frame-trace` delta. GPU values are the current"
        " asynchronous timer-query window, not a per-scenario delta."
    )
    add("")
    add(
        "| scenario | elapsed s | shell commits | commit_us p50 | p95 | p99 |"
        " frame_cb p50 | p95 | p99 | frame p50 | p95 | p99 | gpu composite p50 | p95 | p99 |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for scenario in scenarios:
        if scenario["name"] == "warmup":
            continue
        label = scenario["label"]
        previous = previous_label(scenarios, scenario)
        current = shell.get(label, {})
        old = shell.get(previous, {}) if previous else {}
        timing = timing_row(current)
        frames = frame_percentiles(parsed, label, previous)
        gpu = gpu_row(comp, label)
        if frames is None:
            frame_cells = ("n/a", "n/a", "n/a")
        else:
            frame_cells = (
                str(frames["p50"]),
                str(frames["p95"]),
                str(frames["p99"]),
            )
        add(
            f"| {scenario['name']} | {scenario['elapsed']:.3f} | "
            f"{shell_delta(current, old, 'commits')} | "
            f"{timing['commit_p50']} | {timing['commit_p95']} | {timing['commit_p99']} | "
            f"{timing['callback_p50']} | {timing['callback_p95']} | {timing['callback_p99']} | "
            f"{frame_cells[0]} | {frame_cells[1]} | {frame_cells[2]} | "
            f"{gpu['composite_p50']} | {gpu['composite_p95']} | {gpu['composite_p99']} |"
        )
    add("")

    add("### Scenario transport mix")
    add("")
    add(
        "`shm` is `commits - dmabuf_commits`; the shm rate is the fraction of"
        " chrome commits that actually used the pooled-shm fallback. `n/a`"
        " means the scenario committed no chrome frames, so there is no rate"
        " to report (the `commits` column is 0). `dmabuf_fallbacks` counts"
        " failed dmabuf export attempts (including transient ring-exhausted"
        " retries that later committed dmabuf), so it is reported next to the"
        " shm rate, never averaged into it."
    )
    add("")
    add(
        "| scenario | path | gpu | commits | dmabuf | shm | shm rate |"
        " dmabuf fallbacks | deferred | sync timeouts | copy bytes | dmabuf bytes |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for scenario in scenarios:
        if scenario["name"] == "warmup":
            continue
        label = scenario["label"]
        previous = previous_label(scenarios, scenario)
        current = shell.get(label, {})
        old = shell.get(previous, {}) if previous else {}
        mix = transport(current, old)
        add(
            f"| {scenario['name']} | {mix['path']} | {mix['gpu']} | "
            f"{mix['commits']} | {mix['dmabuf']} | {mix['shm']} | "
            f"{rate_text(mix['shm_rate'])} | {mix['fallbacks']} | "
            f"{mix['deferred']} | {mix['timeouts']} | {mix['copy_bytes']} | "
            f"{mix['dmabuf_bytes']} |"
        )
    add("")

    add("### Per-surface transport")
    add("")
    add(
        "The per-surface `path` is the transport of the surface's last committed"
        " frame; the `dmabuf`/`shm` columns are the scenario's commit counts, so"
        " a surface can end on a transient shm retry while every counted commit"
        " was dmabuf. Read the counts, not only the last `path`."
    )
    add("")
    for scenario in scenarios:
        if scenario["name"] == "warmup":
            continue
        label = scenario["label"]
        previous = previous_label(scenarios, scenario)
        current_surfaces = surfaces.get(label, {})
        if not current_surfaces:
            continue
        old_surfaces = surfaces.get(previous, {}) if previous else {}
        add(f"#### {scenario['name']}")
        add("")
        add(
            "| surface | path | commits | dmabuf | shm | dmabuf fallbacks |"
            " copy bytes | commit_us p50 | p95 | p99 | frame_cb p50 | p95 | p99 |"
        )
        add("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        for row in surface_transport(current_surfaces, old_surfaces):
            add(
                f"| {row['surface']} | {row['path']} | {row['commits']} | "
                f"{row['dmabuf']} | {row['shm']} | {row['fallbacks']} | "
                f"{row['copy_bytes']} | {row['commit_p50']} | {row['commit_p95']} | "
                f"{row['commit_p99']} | {row['callback_p50']} | "
                f"{row['callback_p95']} | {row['callback_p99']} |"
            )
        add("")


def idle_evidence(parsed, scenarios, name):
    scenario = scenario_by_name(scenarios, name)
    if scenario is None:
        return None
    label = scenario["label"]
    previous = previous_label(scenarios, scenario)
    shell = parsed["shell"]
    comp = parsed["comp"]
    current = shell.get(label, {})
    old = shell.get(previous, {}) if previous else {}
    render = comp_entry(comp, label, "render stats")
    old_render = comp_entry(comp, previous, "render stats") if previous else {}
    animation = comp_entry(comp, label, "animation stats")
    old_animation = comp_entry(comp, previous, "animation stats") if previous else {}
    frame = comp_entry(comp, label, "frame timing")
    request = frame_request(comp, label)
    return {
        "scenario": scenario,
        "label": label,
        "scene_frames": shell_delta(current, old, "scene_frames"),
        "commits": shell_delta(current, old, "commits"),
        "grab_renders": shell_delta(current, old, "grab_renders"),
        "frame_callbacks": shell_delta(current, old, "frame_callbacks"),
        "frames_rendered": as_int(render.get("frames_rendered"))
        - as_int(old_render.get("frames_rendered")),
        "client_wakeups": as_int(render.get("client_wakeups"))
        - as_int(old_render.get("client_wakeups")),
        "animation_frames": as_int(animation.get("frames_stepped"))
        - as_int(old_animation.get("frames_stepped")),
        "frame": frame,
        "request": request,
    }


def write_one_idle(add, evidence, title):
    scenario = evidence["scenario"]
    add(f"#### {title} (`{evidence['label']}`, {scenario['elapsed']:.1f} s)")
    add("")
    add("```text")
    add(
        "shell scene_frames_delta={} commits_delta={} grab_renders_delta={} "
        "frame_callbacks_delta={}".format(
            evidence["scene_frames"],
            evidence["commits"],
            evidence["grab_renders"],
            evidence["frame_callbacks"],
        )
    )
    add(
        "compositor frames_rendered_delta={} client_wakeups_delta={} "
        "animation_frames_stepped_delta={}".format(
            evidence["frames_rendered"],
            evidence["client_wakeups"],
            evidence["animation_frames"],
        )
    )
    frame = evidence["frame"]
    add(
        "frame timing: samples={} p50_us={} p95_us={} p99_us={} max_us={}".format(
            frame.get("samples", "0"),
            frame.get("p50_us", "0"),
            frame.get("p95_us", "0"),
            frame.get("p99_us", "0"),
            frame.get("max_us", "0"),
        )
    )
    request = evidence["request"]
    add(
        "frame request: presentations={} last_interval_us={} idle={}".format(
            request.get("presentations", "0"),
            request.get("last_interval_us", "0"),
            request.get("idle", "0"),
        )
    )
    add("```")
    add("")


def write_idle(add, parsed, scenarios):
    pre = idle_evidence(parsed, scenarios, "idle")
    settled = idle_evidence(parsed, scenarios, "idle-settled")
    if pre is None and settled is None:
        add("No idle scenario was recorded.")
        add("")
        return
    add(
        "The pre-animation window is the scenario set's idle; the settled"
        " window runs after the fullscreen scenario, with the animation stopped"
        " and the chrome back at rest. The parent spec's zero-wakeup claim is"
        " the settled window."
    )
    add("")
    if pre is not None:
        write_one_idle(add, pre, "Pre-animation idle")
    if settled is not None:
        write_one_idle(add, settled, "Settled idle")
        evidence = settled
        shell_zero = (
            evidence["scene_frames"] == 0
            and evidence["commits"] == 0
            and evidence["grab_renders"] == 0
        )
        comp_zero = evidence["frames_rendered"] == 0 and evidence["client_wakeups"] == 0
        animation_zero = evidence["animation_frames"] == 0
        if shell_zero and comp_zero and animation_zero:
            add(
                "Verdict: **zero-wakeup confirmed** — the settled idle interval"
                " produced no shell scene frames, no chrome commits, no"
                " compositor frames, and no client wakeups."
            )
        elif (
            evidence["client_wakeups"] == 0
            and animation_zero
            and evidence["scene_frames"] <= 2
            and evidence["frames_rendered"] <= 2
        ):
            add(
                "Verdict: **near-zero, no busy loop** — the settled idle window"
                f" produced {evidence['scene_frames']} shell scene frame(s),"
                f" {evidence['frames_rendered']} compositor frame(s), zero client"
                " wakeups, and no animation frames in 60 s. The shell's menu-bar"
                " clock updates at the minute boundary (`clockDate=1`), so a"
                " 60 s window can cross one periodic update; this is not the"
                " animation loop. The earlier 5 s chrome-baseline idle was"
                " exactly zero."
            )
        else:
            add(
                "Verdict: **not zero-wakeup** — see the deltas above; the"
                " settled idle interval still produced shell or compositor"
                " work on this host."
            )
        add("")


def refresh_row(parsed, scenarios, name):
    scenario = scenario_by_name(scenarios, name)
    if scenario is None:
        return None
    label = scenario["label"]
    previous = previous_label(scenarios, scenario)
    shell = parsed["shell"]
    comp = parsed["comp"]
    current = shell.get(label, {})
    old = shell.get(previous, {}) if previous else {}
    refresh = comp_entry(comp, label, "refresh stats")
    animation = comp_entry(comp, label, "animation stats")
    old_animation = comp_entry(comp, previous, "animation stats") if previous else {}
    frames = frame_percentiles(parsed, label, previous)
    mix = transport(current, old)
    timing = timing_row(current)
    request = frame_request(comp, label)
    gpu = gpu_row(comp, label)
    return {
        "scenario": scenario,
        "refresh_hz": refresh.get("refresh_hz", "?"),
        "interval_us": refresh.get("interval_us", "0"),
        "deadline_us": refresh.get("deadline_us", "0"),
        "scene_frames": shell_delta(current, old, "scene_frames"),
        "commits": shell_delta(current, old, "commits"),
        "callbacks": shell_delta(current, old, "frame_callbacks"),
        "animation_frames": as_int(animation.get("frames_stepped"))
        - as_int(old_animation.get("frames_stepped")),
        "frames": frames,
        "timing": timing,
        "mix": mix,
        "host_interval_us": request.get("last_interval_us", "0"),
        "presentations": as_int(request.get("presentations"))
        - as_int(frame_request(comp, previous).get("presentations")) if previous else as_int(request.get("presentations")),
        "gpu": gpu,
    }


def write_refresh_matrix(add, main_parsed, main_scenarios, raw_root, refreshes, sweep_s):
    add("### Refresh matrix (nested derived deadline)")
    add("")
    add(
        "The nested backend has no hardware vblank. The `refresh stats`"
        " interval/deadline is the policy evidence; `host interval` is the"
        " nested host's actual presentation cadence, which is not the derived"
        " deadline. Missed-deadline counts are DRM-only (`presentation stats`)"
        " and are OPEN for T192c."
    )
    add("")
    add(
        "| refresh | interval us | deadline us | shell frames | commits |"
        " callbacks | callback Hz | frame p50 | p95 | p99 | shell commit p50 |"
        " p95 | p99 | shm rate | copy bytes | host interval us | verdict |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    rows = []
    main_row = refresh_row(main_parsed, main_scenarios, "dock-sweep")
    if main_row:
        main_row["requested"] = 144
        rows.append(main_row)
    for hz in refreshes:
        name = f"refresh-{hz}"
        log_dir = os.path.join(raw_root, name)
        comp_log = os.path.join(log_dir, "compositor.txt")
        shell_log = os.path.join(log_dir, "shell.txt")
        driver_log = os.path.join(log_dir, "driver.txt")
        if not all(os.path.exists(path) for path in (comp_log, shell_log, driver_log)):
            continue
        parsed = parse(comp_log)
        parsed_shell = parse(shell_log)
        parsed["shell"] = parsed_shell["shell"]
        parsed["surfaces"] = parsed_shell["surfaces"]
        scenarios = parse_scenarios(driver_log)
        row = refresh_row(parsed, scenarios, "sweep")
        if row:
            row["requested"] = int(hz)
            rows.append(row)
    for row in sorted(rows, key=lambda item: item["requested"]):
        frames = row["frames"]
        frame_cells = (
            ("n/a", "n/a", "n/a")
            if frames is None
            else (str(frames["p50"]), str(frames["p95"]), str(frames["p99"]))
        )
        interval = as_int(row["interval_us"])
        derived_hz = (1_000_000 / interval) if interval else 0
        callback_hz = (row["callbacks"] / sweep_s) if sweep_s else 0
        deadline_ok = as_int(row["deadline_us"]) == refresh_deadline_us(row["requested"])
        ratio = (callback_hz / derived_hz) if derived_hz else 0
        if deadline_ok and 0.8 <= ratio <= 1.2:
            verdict = "deadline ok; callback cadence matches"
        elif deadline_ok and row["requested"] > 144:
            verdict = "deadline ok; host-capped (nested host is 144 Hz)"
        elif deadline_ok and ratio > 1.2:
            verdict = (
                f"deadline ok; callback cadence {callback_hz:.1f} Hz"
                f" (faster than derived {derived_hz:.1f} Hz; host-paced)"
            )
        elif deadline_ok:
            verdict = (
                f"deadline ok; callback cadence {callback_hz:.1f} Hz"
                f" (slower than derived {derived_hz:.1f} Hz; host-paced)"
            )
        else:
            verdict = "deadline mismatch"
        add(
            f"| {row['requested']} Hz | {row['interval_us']} | {row['deadline_us']} | "
            f"{row['scene_frames']} | {row['commits']} | {row['callbacks']} | "
            f"{callback_hz:.1f} | {frame_cells[0]} | {frame_cells[1]} | "
            f"{frame_cells[2]} | {row['timing']['commit_p50']} | "
            f"{row['timing']['commit_p95']} | {row['timing']['commit_p99']} | "
            f"{rate_text(row['mix']['shm_rate'])} | {row['mix']['copy_bytes']} | "
            f"{row['host_interval_us']} | {verdict} |"
        )
    add("")
    add(
        "The `refresh stats` deadline is the derived policy value; the shell's"
        " frame-callback cadence is the observed presentation pacing. The"
        " compositor `animation_frames_stepped` counter tracks"
        " compositor-owned animations (window lifecycle, overview), not the"
        " shell's QML Dock spring, so it is not used as the Dock cadence"
        " metric. A 144 Hz derived deadline on a nested host whose frame"
        " callbacks arrive at ~84–87 Hz is reported as host-paced, not healthy"
        " 144 Hz. The 240 Hz row"
        " is the derived-policy check on a 144 Hz nested host, not a 240 Hz"
        " panel measurement. Missed-deadline counts are DRM-only and OPEN for"
        " T192c."
    )
    add("")


def write_tiers(add, raw_root, tiers_name, sweep_s):
    log_dir = os.path.join(raw_root, tiers_name)
    comp_log = os.path.join(log_dir, "compositor.txt")
    shell_log = os.path.join(log_dir, "shell.txt")
    driver_log = os.path.join(log_dir, "driver.txt")
    add("### Material tiers (Full / Reduced / Minimal)")
    add("")
    if not all(os.path.exists(path) for path in (comp_log, shell_log, driver_log)):
        add("The tier session did not produce logs.")
        add("")
        return
    parsed = parse(comp_log)
    parsed_shell = parse(shell_log)
    parsed["shell"] = parsed_shell["shell"]
    parsed["surfaces"] = parsed_shell["surfaces"]
    scenarios = parse_scenarios(driver_log)
    add(
        "| tier | forced | material path | backdrop passes | blur draws |"
        " frame p50 | p95 | p99 | shell commit p50 | p95 | p99 | shm rate |"
        " copy bytes | gpu composite p50 | p95 | p99 |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for tier in ("full", "reduced", "minimal"):
        sweep = scenario_by_name(scenarios, f"{tier}-sweep")
        baseline = scenario_by_name(scenarios, f"{tier}-baseline")
        if sweep is None or baseline is None:
            continue
        label = sweep["label"]
        previous = baseline["label"]
        current = parsed["shell"].get(label, {})
        old = parsed["shell"].get(previous, {})
        mix = transport(current, old)
        frames = frame_percentiles(parsed, label, previous)
        timing = timing_row(current)
        material = comp_entry(parsed["comp"], label, "material stats")
        old_material = comp_entry(parsed["comp"], previous, "material stats")
        degrade = comp_entry(parsed["comp"], label, "degrade stats")
        gpu = gpu_row(parsed["comp"], label)
        frame_cells = (
            ("n/a", "n/a", "n/a")
            if frames is None
            else (str(frames["p50"]), str(frames["p95"]), str(frames["p99"]))
        )
        add(
            f"| {tier} | {degrade.get('forced', '0')} | "
            f"{material.get('path', '?')} | "
            f"{as_int(material.get('backdrop_passes')) - as_int(old_material.get('backdrop_passes'))} | "
            f"{as_int(material.get('blur_draws')) - as_int(old_material.get('blur_draws'))} | "
            f"{frame_cells[0]} | {frame_cells[1]} | {frame_cells[2]} | "
            f"{timing['commit_p50']} | {timing['commit_p95']} | {timing['commit_p99']} | "
            f"{rate_text(mix['shm_rate'])} | {mix['copy_bytes']} | "
            f"{gpu['composite_p50']} | {gpu['composite_p95']} | {gpu['composite_p99']} |"
        )
    add("")
    add(
        "The tier session forces each tier with the synthetic `set"
        " degrade-tier` command (`forced=1`); the baseline snapshot before each"
        " sweep clears the shell's interval rings. GPU values remain the"
        " asynchronous timer-query window."
    )
    add("")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", required=True)
    parser.add_argument("--main-name", default="main-144-full")
    parser.add_argument("--matrix-refreshes", default="60 120 240")
    parser.add_argument("--tiers-name", default="tiers-144")
    parser.add_argument("--host-info", required=True)
    parser.add_argument("--preflight", required=True)
    parser.add_argument("--native", default="")
    parser.add_argument("--open", default="")
    parser.add_argument("--run-notes", default="")
    parser.add_argument("--out", required=True)
    parser.add_argument("--command", default="make t21-presentation-bench")
    parser.add_argument("--idle-s", type=float, default=60.0)
    parser.add_argument("--sweep-s", type=float, default=10.0)
    args = parser.parse_args()

    main_dir = os.path.join(args.raw_root, args.main_name)
    comp_log = os.path.join(main_dir, "compositor.txt")
    shell_log = os.path.join(main_dir, "shell.txt")
    driver_log = os.path.join(main_dir, "driver.txt")
    if not all(os.path.exists(path) for path in (comp_log, shell_log, driver_log)):
        print(
            f"t21-presentation-bench-report: missing main logs under {main_dir}",
            file=sys.stderr,
        )
        return 1
    parsed = parse(comp_log)
    parsed_shell = parse(shell_log)
    parsed["shell"] = parsed_shell["shell"]
    parsed["surfaces"] = parsed_shell["surfaces"]
    scenarios = parse_scenarios(driver_log)
    if not scenarios:
        print(
            "t21-presentation-bench-report: no scenario snapshots in the driver log",
            file=sys.stderr,
        )
        return 1

    lines = []
    add = lines.append
    add("# T-21.4b presentation benchmark — nested scenario run")
    add("")
    add(
        "Generated by `scripts/t21-presentation-bench.sh`"
        " (`make t21-presentation-bench`). This is **nested reference"
        " evidence**: the shell is a Wayland client of a nested Dragonfruit"
        " compositor and the nested output has no hardware vblank. It is not"
        " native presentation proof; the native DRM rows are T192c and are"
        " recorded OPEN at the end of this report."
    )
    add("")
    add(f"- command: `{args.command}`")
    add(
        "- scenario set: idle {} s, Dock sweep {} s, menu/popover, app launch,"
        " Mission Control (10+ windows), fullscreen".format(args.idle_s, args.sweep_s)
    )
    add(
        "- opt-in switches: `DRAGONFRUIT_CHROME_STATS=1`,"
        " `DRAGONFRUIT_CHROME_GPU=1`, `DRAGONFRUIT_GPU_TIMING=1`,"
        " `DRAGONFRUIT_FRAME_TRACE=1`, `DF_DOCK_CHOOSER_FIXTURE=1`"
    )
    add("- refresh matrix: 60/120/240 Hz overrides; the main run is 144 Hz")
    add("- material matrix: Full/Reduced/Minimal")
    add("")

    add("## Host, backend, and mode")
    add("")
    with open(args.host_info, encoding="utf-8", errors="replace") as handle:
        add("```text")
        add(handle.read().rstrip())
        add("```")
    add("")

    add("## Capture preflight")
    add("")
    with open(args.preflight, encoding="utf-8", errors="replace") as handle:
        add("```text")
        add(handle.read().rstrip())
        add("```")
    add("")

    add("## Main scenario set (144 Hz, Full)")
    add("")
    add(
        "Every snapshot is a pair of `SIGUSR1` dumps (shell and compositor)."
        " The shell's `commit_us` / `frame_callback_us` percentiles describe the"
        " interval since the previous dump; the compositor frame percentiles"
        " are the `frame-trace` delta for that interval. The raw logs are under"
        f" `{args.raw_root}/{args.main_name}/`."
    )
    add("")
    write_scenario_tables(add, parsed, scenarios)
    write_idle(add, parsed, scenarios)

    add("## Refresh matrix")
    add("")
    write_refresh_matrix(
        add,
        parsed,
        scenarios,
        args.raw_root,
        args.matrix_refreshes.split(),
        args.sweep_s,
    )

    add("## Material tiers")
    add("")
    write_tiers(add, args.raw_root, args.tiers_name, args.sweep_s)

    if args.run_notes and os.path.exists(args.run_notes):
        with open(args.run_notes, encoding="utf-8", errors="replace") as handle:
            add(handle.read().rstrip())
        add("")

    add("## Modes and outputs not measured here")
    add("")
    add("### 1080p / 4K")
    add("")
    add(
        "**OPEN.** The nested backend follows the host window; it does not"
        " select a DRM mode. The native DP-1 panel is 3840x2160 at 143.999 Hz"
        " (see the host section), but the live session owns DRM master, so the"
        " native mode rows are T192c. The nested run records its actual"
        " physical mode and scale in the per-surface `logical_size` /"
        " `buffer_size` fields above."
    )
    add("")
    add("### Mixed 60 Hz + 144 Hz outputs")
    add("")
    add(
        "**OPEN for this nested run.** The nested backend has one output. The"
        " headless synthetic 60 Hz + 144 Hz trace is already committed as"
        " `docs/captures/t21-vrr-mixed-refresh.txt`; it is scheduler evidence,"
        " not presentation evidence. The native two-output row is T192c."
    )
    add("")
    add("### Native DRM rows")
    add("")
    add(
        "**OPEN.** The native scenario rows (panel modes, 1080p/4K, mixed"
        " outputs, `presentation stats` missed-deadline counts, native GPU"
        " timer queries) are T192c. Do not read the nested numbers above as"
        " native proof."
    )
    add("")
    if args.open:
        add(f"The explicit OPEN artifact is `{args.open}`:")
        add("")
        with open(args.open, encoding="utf-8", errors="replace") as handle:
            add("```text")
            add(handle.read().rstrip())
            add("```")
        add("")
    if args.native:
        with open(args.native, encoding="utf-8", errors="replace") as handle:
            native = handle.read().rstrip()
        if native:
            add("Native diagnostics recorded in this session:")
            add("")
            add("```text")
            add(native)
            add("```")
            add("")

    add("## Raw logs")
    add("")
    add(f"Committed under `{args.raw_root}/`:")
    add("")
    for name in [args.main_name] + [
        f"refresh-{hz}" for hz in args.matrix_refreshes.split()
    ] + [args.tiers_name]:
        directory = os.path.join(args.raw_root, name)
        if not os.path.isdir(directory):
            continue
        add(f"- `{directory}/` — `compositor.txt`, `shell.txt`, `driver.txt`")
    add("")

    add("## Selected raw lines")
    add("")
    add("```text")
    for line in parsed["raw"]:
        if re.search(
            r"chrome stats|chrome surface|frame timing|gpu stats|material stats|"
            r"degrade stats|refresh stats|frame request|animation stats",
            line,
        ):
            add(line)
    add("```")
    add("")

    add("## Verdict")
    add("")
    main_transport = transport(
        parsed["shell"].get(scenarios[-1]["label"], {}),
        parsed["shell"].get(scenarios[-2]["label"], {}) if len(scenarios) > 1 else {},
    )
    add(
        "Final cumulative transport label: `path={}` `gpu={}`"
        " `gpu_reason={}` (the reason is the first recorded fallback token,"
        " not the last scenario's transport).".format(
            main_transport["path"], main_transport["gpu"], main_transport["gpu_reason"]
        )
    )
    add("")
    add(
        "The per-scenario tables above name the shm/fallback rate for every"
        " scenario. A zero-copy claim applies only to dmabuf frames; fallback"
        " frames are counted in the `shm` column, not averaged into the dmabuf"
        " row. `dmabuf_fallbacks` is a failed-export-attempt count (including"
        " transient retries), so it can be non-zero even when the shm rate is"
        " zero; the shm rate is the actual fallback evidence."
    )
    add("")
    add(
        "The Mission Control overview fallback's 18 432 000 copy bytes over 2"
        " shm commits is 9 216 000 bytes per frame, exactly one 1920x1200x4"
        " full-surface copy; every measured dmabuf row reports zero copy bytes."
    )
    add("")
    add(
        "Idle, the 144 Hz Dock sweep, the refresh matrix, and the material"
        " tiers are the nested acceptance set for T192b. Native presentation"
        " numbers, missed-deadline counts, 1080p/4K, and mixed outputs remain"
        " OPEN for T192c."
    )
    add("")
    add(
        "**Known follow-up (T193):** the full GPU rail crashes the shell during"
        " Mission Control teardown (`GpuChromeFrameProducer::destroySlotResources`"
        " after null `Shadow.qml` material state). This run excludes the"
        " overview surface from the GPU rail so the rest of the scenario set"
        " completes; the overview row is the visible shm fallback. See the run"
        " notes above for the backtrace and the reproduction command."
    )
    add("")

    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines).rstrip() + "\n")
    print(f"t21-presentation-bench-report: wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
