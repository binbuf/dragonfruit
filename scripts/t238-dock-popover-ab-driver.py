#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive and summarise the T238 Dock-popover stable-size A/B.

The shell runs with ``DF_DOCK_POPOVER_AB=<cycles>`` and opens/closes the real
window-chooser popover on a fixed cadence. Each opening carries one
``chrome trace id=...`` correlation id; the stage lines and the
``chrome stage stats`` dump are parsed here. The driver also spawns two X11
windows so the Dock has a grouped entry for the chooser.

Usage:
  t238-dock-popover-ab-driver.py run --synth S --socket S --shell-pid P ...
  t238-dock-popover-ab-driver.py compare A.json B.json

The A/B is a diagnostic, not a production switch. It keeps the transport and
the running revision identical between the two configurations; only
``DRAGONFRUIT_DOCK_STABLE_POPOVER`` changes.
"""
import argparse
import json
import os
import re
import signal
import subprocess
import sys
import time


def log(message):
    print(f"t238-dock-popover-ab: {message}", flush=True)


def field(text, key):
    match = re.search(rf"(?:^|\s){re.escape(key)}=(\S+)", text)
    return match.group(1) if match else None


def numeric_fields(text):
    out = {}
    for match in re.finditer(r"(?:^|\s)([A-Za-z0-9_]+)=(-?[0-9]+(?:\.[0-9]+)?)", text):
        key, value = match.groups()
        out[key] = float(value) if "." in value else int(value)
    return out


def percentile(values, p):
    if not values:
        return 0
    ordered = sorted(values)
    index = (p * (len(ordered) - 1)) // 100
    return ordered[index]


def timing(values):
    return {
        "samples": len(values),
        "p50": percentile(values, 50),
        "p95": percentile(values, 95),
        "p99": percentile(values, 99),
        "max": max(values) if values else 0,
    }


def parse_traces(text):
    traces = {}
    begin_re = re.compile(
        r"chrome trace id=(\d+) stage=begin kind=(\S+) t_ms=(\d+)"
        r"(?: detail=(.*))?$"
    )
    event_re = re.compile(
        r"chrome trace id=(\d+) stage=(\S+) dt_us=(-?\d+) t_ms=(\d+)"
        r"(?: detail=(.*))?$"
    )
    for line in text.splitlines():
        if "chrome trace " not in line:
            continue
        begin = begin_re.search(line)
        if begin:
            trace_id, kind, t_ms, detail = begin.groups()
            traces[trace_id] = {
                "id": int(trace_id),
                "kind": kind,
                "begin_t_ms": int(t_ms),
                "begin_detail": detail or "",
                "stages": {},
                "stage_t_ms": {},
                "details": {},
                "end_us": None,
                "end_t_ms": None,
            }
            continue
        event = event_re.search(line)
        if not event:
            continue
        trace_id, stage, dt_us, _t_ms, detail = event.groups()
        trace = traces.get(trace_id)
        if trace is None:
            continue
        if stage == "end":
            trace["end_us"] = int(dt_us)
            trace["end_t_ms"] = int(_t_ms)
            trace["end_detail"] = detail or ""
        else:
            trace["stages"][stage] = int(dt_us)
            trace["stage_t_ms"][stage] = int(_t_ms)
            trace["details"][stage] = detail or ""
    return traces


def stage_delta(trace, first, second):
    stages = trace["stages"]
    if first in stages and second in stages:
        return stages[second] - stages[first]
    return None


def parse_frame_elements(text):
    """Per-frame element-type counts from the compositor trace."""
    events = []
    pattern = re.compile(r"DFTRACE frame-elements (.*) t=(\d+)$")
    for line in text.splitlines():
        match = pattern.search(line)
        if not match:
            continue
        detail, _t_ms = match.groups()
        events.append(numeric_fields(detail))
    return events


def summarise_frame_elements(events):
    if not events:
        return {}
    material = [event for event in events if event.get("material", 0) == 1]
    # The element breakdown is about the material/full-output draw list. The
    # startup and no-material fallback frames have a different composition;
    # report them separately so they cannot hide the steady-state shape.
    sample = material or events
    # Drop the first quarter: a window/Dock mapping transition can inflate the
    # early element count. Growth is then measured across the settled half.
    stable = sample[len(sample) // 4 :]
    totals = [event.get("total", 0) for event in stable]
    mid = len(totals) // 2
    first = sum(totals[:mid]) / mid if mid else 0
    second = sum(totals[mid:]) / (len(totals) - mid) if len(totals) > mid else 0

    def avg(key):
        return sum(event.get(key, 0) for event in sample) / len(sample)

    return {
        "samples": len(events),
        "material_samples": len(material),
        "material_frame_pct": 100.0 * len(material) / len(events),
        "total_avg": sum(totals) / len(totals),
        "total_min": min(totals),
        "total_p50": percentile(totals, 50),
        "total_p95": percentile(totals, 95),
        "total_max": max(totals),
        "first_half_avg": first,
        "second_half_avg": second,
        "growth_pct": (second / first - 1.0) * 100.0 if first else 0.0,
        "decoration_avg": avg("decoration"),
        "chrome_avg": avg("chrome"),
        "appear_avg": avg("appear"),
        "wallpaper_avg": avg("wallpaper"),
        "blur_avg": avg("blur"),
        "scene_avg": avg("scene"),
        "pointer_avg": avg("pointer"),
        "space_avg": avg("space"),
        "titlebar_avg": avg("titlebar"),
        "titlebar_fill_avg": avg("titlebar_fill"),
        "titlebar_lights_avg": avg("titlebar_lights"),
        "titlebar_glyphs_avg": avg("titlebar_glyphs"),
        "window_content_avg": avg("window_content"),
        "window_shadow_avg": avg("window_shadow"),
        "shadow_area_avg": avg("shadow_area"),
        "shadow_span_area_avg": avg("shadow_span_area"),
        "titlebar_area_avg": avg("titlebar_area"),
        "motion_avg": avg("motion"),
        "menu_avg": avg("menu"),
        "desktop_avg": avg("desktop"),
        "lock_avg": avg("lock"),
    }


def parse_dftrace(text):
    events = []
    pattern = re.compile(r"DFTRACE (\S+) (.*) t=(\d+)$")
    for line in text.splitlines():
        match = pattern.search(line)
        if not match:
            continue
        event, detail, t_ms = match.groups()
        events.append(
            {
                "event": event,
                "detail": detail,
                "t_ms": int(t_ms),
                "surface": field(detail, "surface"),
                "output": field(detail, "output"),
                "frame": int(field(detail, "frame") or 0),
            }
        )
    events.sort(key=lambda event: event["t_ms"])
    return events


def summarise_traces(traces, compositor_events, imports_by_trace):
    ended = [trace for trace in traces.values() if trace.get("end_us") is not None]
    ended.sort(key=lambda trace: trace["id"])
    # A request that closed before its first frame callback never became
    # visible; keep it in the individual trace list and count it, but do not
    # let it contaminate the request-to-visible percentiles.
    openings = [trace for trace in ended if trace.get("end_detail") == "presented"]
    closed_early = len(ended) - len(openings)
    # The first ten requests include the cold allocation and the first paint;
    # the rest describe the steady-state path.
    first = openings[:10]
    steady = openings[10:]

    def collect(group):
        opens = [trace["end_us"] for trace in group]
        scene = [trace["stages"].get("scene-prepared") for trace in group]
        scene = [value for value in scene if value is not None]
        export = [
            stage_delta(trace, "export-begin", "export-done") for trace in group
        ]
        export = [value for value in export if value is not None]
        commit = [
            stage_delta(trace, "commit-begin", "commit-done") for trace in group
        ]
        commit = [value for value in commit if value is not None]
        present = [trace["stages"].get("frame-presented") for trace in group]
        present = [value for value in present if value is not None]
        holds = []
        shared = []
        for trace in group:
            for detail in trace["details"].values():
                if "hold_us=" in detail:
                    value = int(field(detail, "hold_us"))
                    if value >= 0:
                        holds.append(value)
                    if field(detail, "last") == "1":
                        shared.append(value)

        # T238 follow-up: correlate the shell's commit with the compositor's
        # receive and actual presentation. The shell's frame callback is sent
        # after `queue_frame` on DRM and after the host swap on nested, so it
        # is not the panel presentation endpoint.
        import_roundtrips = []
        import_count = 0
        for trace in group:
            for event in imports_by_trace.get(trace["id"], []):
                import_count += 1
                roundtrip = event.get("roundtrip_us")
                if isinstance(roundtrip, int) and roundtrip >= 0:
                    import_roundtrips.append(roundtrip)

        submit_us = []
        present_us = []
        present_after_commit_us = []
        present_to_callback_us = []
        for trace in group:
            commit_done_t = trace["stage_t_ms"].get("commit-done")
            if commit_done_t is None:
                continue
            begin_t = trace["begin_t_ms"]
            end_t = trace.get("end_t_ms")
            # The receive timestamp must be close to the shell commit; a
            # later commit (the next opening or a close) is not this frame.
            chrome_commit = next(
                (
                    event
                    for event in compositor_events
                    if event["event"] == "chrome-commit"
                    and event["surface"] in ("dock", "dock-popup")
                    and commit_done_t <= event["t_ms"] <= commit_done_t + 20
                    and (end_t is None or event["t_ms"] <= end_t + 20)
                ),
                None,
            )
            if chrome_commit is None:
                continue
            # Prefer the exact frame id: the compositor logs `frame-start`
            # before the frame and `present` with the same id at the real
            # presentation endpoint. This removes the millisecond-window
            # guess that left some openings unmatched.
            frame_start = next(
                (
                    event
                    for event in compositor_events
                    if event["event"] == "frame-start"
                    and event["t_ms"] >= chrome_commit["t_ms"]
                    and (end_t is None or event["t_ms"] <= end_t + 20)
                ),
                None,
            )
            present = None
            if frame_start is not None and frame_start["frame"]:
                present = next(
                    (
                        event
                        for event in compositor_events
                        if event["event"] == "present"
                        and event["frame"] == frame_start["frame"]
                    ),
                    None,
                )
            if present is None:
                # Fallback for a log without frame ids: the next present after
                # the commit, still bounded to the opening window.
                present = next(
                    (
                        event
                        for event in compositor_events
                        if event["event"] == "present"
                        and event["t_ms"] >= chrome_commit["t_ms"]
                        and (end_t is None or event["t_ms"] <= end_t + 20)
                    ),
                    None,
                )
            if present is None:
                continue
            receive_t = frame_start["t_ms"] if frame_start is not None else chrome_commit["t_ms"]
            submit_us.append((receive_t - commit_done_t) * 1000)
            present_us.append((present["t_ms"] - begin_t) * 1000)
            present_after_commit_us.append(
                (present["t_ms"] - commit_done_t) * 1000
            )
            if end_t is not None:
                present_to_callback_us.append((end_t - present["t_ms"]) * 1000)

        return {
            "openings": len(group),
            "open_us": timing(opens),
            "scene_prepared_us": timing(scene),
            "export_us": timing(export),
            "commit_us": timing(commit),
            "present_us": timing(present),
            "submit_us": timing(submit_us),
            "actual_present_us": timing(present_us),
            "present_after_commit_us": timing(present_after_commit_us),
            "present_to_callback_us": timing(present_to_callback_us),
            "import_count": import_count,
            "import_roundtrip_us": timing(import_roundtrips),
            "dmabuf_hold_us": timing(holds),
            "dmabuf_shared_hold_us": timing(shared),
        }

    def import_summary(group):
        count = 0
        roundtrips = []
        for trace in group:
            for event in imports_by_trace.get(trace["id"], []):
                count += 1
                roundtrip = event.get("roundtrip_us")
                if isinstance(roundtrip, int) and roundtrip >= 0:
                    roundtrips.append(roundtrip)
        return {"count": count, "roundtrip_us": timing(roundtrips)}

    counts = {
        "target_committed": sum(
            sum(1 for stage in trace["stages"] if stage == "target-committed")
            for trace in openings
        ),
        "target_deferred": sum(
            sum(1 for stage in trace["stages"] if stage == "target-deferred")
            for trace in openings
        ),
        "target_failed": sum(
            sum(1 for stage in trace["stages"] if stage == "target-failed")
            for trace in openings
        ),
        "dmabuf_release": sum(
            sum(1 for stage in trace["stages"] if stage == "dmabuf-release")
            for trace in openings
        ),
        "closed_before_present": closed_early,
    }
    return {
        "all": collect(openings),
        "first10": collect(first),
        "steady": collect(steady),
        "imports": {
            "all": import_summary(ended),
            "first10": import_summary(first),
            "steady": import_summary(steady),
        },
        "counts": counts,
        "opening_ids": [trace["id"] for trace in ended],
        "presented_ids": [trace["id"] for trace in openings],
        # Keep the individual traces next to the percentiles: the A/B is a
        # causal diagnostic, and a single cold opening can explain a p99.
        "openings": [
            {
                "id": trace["id"],
                "begin_t_ms": trace["begin_t_ms"],
                "begin_detail": trace["begin_detail"],
                "end_us": trace["end_us"],
                "end_detail": trace.get("end_detail", ""),
                "stages": trace["stages"],
                "details": trace["details"],
                "imports": imports_by_trace.get(trace["id"], []),
            }
            for trace in ended
        ],
    }


def last_line(text, needle):
    found = None
    for line in text.splitlines():
        if needle in line:
            found = line
    return found or ""


def parse_shell(text, compositor_text):
    stage_line = last_line(text, "chrome stage stats (SIGUSR1-1):")
    stats_line = last_line(text, "chrome stats (SIGUSR1-1):")
    surfaces = {}
    for line in text.splitlines():
        if "chrome surface (SIGUSR1-1):" not in line:
            continue
        name = field(line, "surface")
        if name:
            surfaces[name] = numeric_fields(line)
    resizes = [
        numeric_fields(line)
        for line in text.splitlines()
        if "chrome resize:" in line
    ]
    fbo_events = [
        numeric_fields(line)
        for line in text.splitlines()
        if "chrome fbo:" in line
    ]
    import_events = [
        numeric_fields(line)
        for line in text.splitlines()
        if "chrome import:" in line
    ]
    imports_by_trace = {}
    for line in text.splitlines():
        if "chrome import:" not in line:
            continue
        event = numeric_fields(line)
        event["surface"] = field(line, "surface")
        imports_by_trace.setdefault(event.get("trace_id", 0), []).append(event)
    return {
        "stage": numeric_fields(stage_line),
        "stats": numeric_fields(stats_line),
        "surfaces": surfaces,
        "resize_events": resizes,
        "fbo_events": fbo_events,
        "import_events": import_events,
        "traces": summarise_traces(
            parse_traces(text),
            parse_dftrace(compositor_text),
            imports_by_trace,
        ),
    }


def parse_compositor(text):
    material_line = last_line(text, "material stats (SIGUSR1-1):") or ""
    material = numeric_fields(material_line)
    passes = material.get("material_scene_passes") or 0
    composite_frames = material.get("composite_frames") or 0
    material["scene_elements_kept_per_pass"] = (
        material.get("scene_elements_kept", 0) / passes if passes else 0
    )
    material["scene_elements_offered_per_pass"] = (
        material.get("scene_elements_total", 0) / passes if passes else 0
    )
    material["composite_elements_per_frame"] = (
        material.get("composite_elements", 0) / composite_frames
        if composite_frames
        else 0
    )
    return {
        "frame_timing": numeric_fields(
            last_line(text, "frame timing (SIGUSR1-1):")
        ),
        "frame_stages": numeric_fields(
            last_line(text, "frame stage timing (SIGUSR1-1):")
        ),
        "frame_timeline": numeric_fields(
            last_line(text, "frame timeline (SIGUSR1-1):")
        ),
        "gpu": numeric_fields(last_line(text, "gpu stats (SIGUSR1-1):")),
        "material": material,
        "composite_counts": numeric_fields(
            last_line(text, "composite elements (SIGUSR1-1):")
        ),
        "frame_elements": summarise_frame_elements(parse_frame_elements(text)),
        "material_region_groups": field(material_line, "region_groups"),
        "output_material": numeric_fields(
            last_line(text, "output material stats (SIGUSR1-1):")
        ),
    }


def spawn_windows(display, count=2):
    children = []
    for index in range(count):
        x = 40 + (index % 4) * 280
        y = 120 + (index // 4) * 180
        title = f"T238 A/B window {index}"
        env = dict(os.environ)
        env["DISPLAY"] = display
        children.append(
            subprocess.Popen(
                [
                    "xmessage",
                    "-geometry",
                    f"240x120+{x}+{y}",
                    "-title",
                    title,
                    title,
                ],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        )
    return children


def wait_for_line(path, needle, timeout):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                text = handle.read()
        except OSError:
            text = ""
        if needle in text:
            return True
        time.sleep(0.2)
    return False


def wait_for_stats(path, needle, timeout=20.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                text = handle.read()
        except OSError:
            text = ""
        if needle in text:
            time.sleep(0.3)
            return True
        time.sleep(0.1)
    return False


def read(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def build_report(
    label, stable, cycles, commit, shell_text, compositor_text, arm=""
):
    return {
        "label": label,
        "stable_popover": stable,
        "arm": arm,
        "cycles": cycles,
        "commit": commit,
        "shell": parse_shell(shell_text, compositor_text),
        "compositor": parse_compositor(compositor_text),
    }


def write_report(report, out, cycles):
    openings = len(report["shell"]["traces"]["opening_ids"])
    if openings < cycles:
        log(
            f"only {openings} of {cycles} openings produced a "
            "complete trace; refusing to write a partial A/B"
        )
        return False
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    with open(out, "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    log(f"wrote {out}")
    return True


def run(args):
    runtime = os.environ.get("XDG_RUNTIME_DIR", "/run/user")
    display_file = os.path.join(runtime, f"{args.socket}.x11-display")
    display = ""
    if os.path.exists(display_file):
        with open(display_file, encoding="utf-8") as handle:
            display = handle.read().strip()
    if not display:
        log("no nested Xwayland DISPLAY hand-off; cannot create a grouped entry")
        return 1

    children = spawn_windows(display, 2)
    try:
        if not wait_for_line(
            args.shell_log,
            "Dock popover A/B complete count=",
            args.timeout,
        ):
            log("the shell did not finish the popover cycle")
            return 1
        time.sleep(args.settle)
        for pid in (args.shell_pid, args.compositor_pid):
            try:
                os.kill(pid, signal.SIGUSR1)
            except ProcessLookupError:
                log(f"pid {pid} disappeared before the stats dump")
                return 1
        if not wait_for_stats(
            args.shell_log, "chrome stage stats (SIGUSR1-1):"
        ):
            log("the shell did not emit chrome stage stats")
            return 1
        if not wait_for_stats(
            args.compositor_log, "frame timing (SIGUSR1-1):"
        ):
            log("the compositor did not emit frame timing")
            return 1
        shell_text = read(args.shell_log)
        compositor_text = read(args.compositor_log)
        report = build_report(
            args.label,
            args.stable,
            args.cycles,
            args.commit,
            shell_text,
            compositor_text,
            args.arm,
        )
        return 0 if write_report(report, args.out, args.cycles) else 1
    finally:
        for child in children:
            try:
                child.terminate()
            except ProcessLookupError:
                pass
        for child in children:
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                child.kill()


def parse_logs(args):
    report = build_report(
        args.label,
        args.stable,
        args.cycles,
        args.commit,
        read(args.shell_log),
        read(args.compositor_log),
        args.arm,
    )
    return 0 if write_report(report, args.out, args.cycles) else 1


def fmt(value):
    if value is None:
        return "—"
    if isinstance(value, float):
        return f"{value:.0f}"
    return str(value)


def table_row(name, left, right, unit="µs"):
    return (
        f"| {name} | {fmt(left)} | {fmt(right)} | {unit} |"
    )


def compare(left_path, right_path):
    with open(left_path, encoding="utf-8") as handle:
        left = json.load(handle)
    with open(right_path, encoding="utf-8") as handle:
        right = json.load(handle)

    def metric(report, *path):
        value = report
        for key in path:
            if not isinstance(value, dict):
                return None
            value = value.get(key)
        return value

    left_name = f"{left['label']} (stable={int(left['stable_popover'])})"
    right_name = f"{right['label']} (stable={int(right['stable_popover'])})"
    print(f"# T238 Dock popover A/B: {left_name} vs {right_name}")
    print()
    print(f"- revision: `{left.get('commit', 'unknown')}`")
    print(f"- cycles per configuration: {left.get('cycles')}")
    print()
    print("## Request-to-visible and stage costs")
    print()
    print(f"| metric | {left_name} | {right_name} | unit |")
    print("|---|---:|---:|---|")
    for group in ("first10", "steady", "all"):
        print(f"| **{group} openings** | | | |")
        for key, label in (
            ("open_us", "popover open (request → frame callback)"),
            ("scene_prepared_us", "scene prepared"),
            ("export_us", "export (render + fence)"),
            ("commit_us", "commit (import + flush)"),
            ("present_us", "frame-presented timestamp"),
            ("actual_present_us", "request → actual presentation"),
            ("submit_us", "shell commit → compositor receive"),
            ("present_after_commit_us", "shell commit → actual presentation"),
            ("present_to_callback_us", "actual presentation → frame callback"),
        ):
            for p in ("p50", "p95", "p99"):
                print(
                    table_row(
                        f"{label} {p}",
                        metric(left, "shell", "traces", group, key, p),
                        metric(right, "shell", "traces", group, key, p),
                    )
                )
    print()
    print("## Buffer hold (every release, stage counters)")
    print()
    print(f"| metric | {left_name} | {right_name} | unit |")
    print("|---|---:|---:|---|")
    for key, label in (
        ("dmabuf_hold_us", "per-surface buffer hold"),
        ("dmabuf_shared_hold_us", "shared Dock+popup hold"),
    ):
        for p in ("p50", "p95", "p99"):
            print(
                table_row(
                    f"{label} {p}",
                    metric(left, "shell", "stage", f"{key}_{p}"),
                    metric(right, "shell", "stage", f"{key}_{p}"),
                )
            )
    print()
    print("## Allocation, resize, and exhaustion")
    print()
    print(f"| counter | {left_name} | {right_name} |")
    print("|---|---:|---:|")
    for key, label in (
        ("scene_resizes", "scene resizes"),
        ("fbo_allocs", "FBO allocations"),
        ("fbo_bytes_live", "FBO bytes live"),
        ("fbo_bytes_peak", "FBO bytes peak"),
        ("dmabuf_imports", "dma-buf imports"),
        ("dmabuf_import_failures", "dma-buf import failures"),
        ("ring_exhausted", "ring exhausted"),
        ("popover_opens", "popover openings"),
        ("dmabuf_allocs", "dma-buf buffers allocated"),
        ("dmabuf_buffers", "dma-buf buffers live"),
        ("dmabuf_reuses", "dma-buf buffer reuses"),
    ):
        print(
            table_row(
                label,
                metric(left, "shell", "stage", key)
                if key.startswith(("scene_", "fbo_", "dmabuf_import", "ring_", "popover_"))
                else metric(left, "shell", "stats", key),
                metric(right, "shell", "stage", key)
                if key.startswith(("scene_", "fbo_", "dmabuf_import", "ring_", "popover_"))
                else metric(right, "shell", "stats", key),
                "",
            )
        )
    print()
    print("## dma-buf imports attributed to openings")
    print()
    print(f"| metric | {left_name} | {right_name} | unit |")
    print("|---|---:|---:|---|")
    for group in ("first10", "steady", "all"):
        print(f"| **{group}** | | | |")
        print(
            table_row(
                "imports",
                metric(left, "shell", "traces", "imports", group, "count"),
                metric(right, "shell", "traces", "imports", group, "count"),
                "",
            )
        )
        for p in ("p50", "p95", "p99"):
            print(
                table_row(
                    f"import round-trip {p}",
                    metric(
                        left,
                        "shell",
                        "traces",
                        "imports",
                        group,
                        "roundtrip_us",
                        p,
                    ),
                    metric(
                        right,
                        "shell",
                        "traces",
                        "imports",
                        group,
                        "roundtrip_us",
                        p,
                    ),
                )
            )
    print()
    print("## Compositor frame and GPU")
    print()
    print(f"| metric | {left_name} | {right_name} | unit |")
    print("|---|---:|---:|---|")
    for key in ("p50_us", "p95_us", "p99_us", "max_us"):
        print(
            table_row(
                f"compositor frame {key}",
                metric(left, "compositor", "frame_timing", key),
                metric(right, "compositor", "frame_timing", key),
            )
        )
    for key in (
        "scene_p99_us",
        "blur_p99_us",
        "composite_p99_us",
    ):
        print(
            table_row(
                f"gpu {key}",
                metric(left, "compositor", "gpu", key),
                metric(right, "compositor", "gpu", key),
            )
        )
    print()
    print("## Compositor frame stages")
    print()
    print(f"| stage | {left_name} p50 / p95 | {right_name} p50 / p95 |")
    print("|---|---:|---:|")
    for stage in (
        "bind",
        "prepare",
        "scene",
        "blur",
        "composite",
        "submit",
        "callback",
        "scene_sync",
        "composite_sync",
    ):
        left_stage = metric(left, "compositor", "frame_stages", f"{stage}_p50_us")
        left_high = metric(left, "compositor", "frame_stages", f"{stage}_p95_us")
        right_stage = metric(right, "compositor", "frame_stages", f"{stage}_p50_us")
        right_high = metric(right, "compositor", "frame_stages", f"{stage}_p95_us")
        print(
            f"| {stage} | {fmt(left_stage)} / {fmt(left_high)} | "
            f"{fmt(right_stage)} / {fmt(right_high)} |"
        )
    print()
    print("## Per-frame CPU/GPU correlation")
    print()
    print(
        f"| metric | {left_name} | {right_name} | unit |"
    )
    print("|---|---:|---:|---|")
    for key, label in (
        ("matched", "joined frames"),
        ("pending", "pending (GPU result not yet in)"),
        ("cpu_without_gpu", "frames with CPU scene but no GPU scene"),
        ("gpu_without_cpu", "GPU scene samples with no CPU frame"),
        ("cpu_total_p50_us", "per-frame CPU scene+composite p50"),
        ("cpu_total_p95_us", "per-frame CPU scene+composite p95"),
        ("gpu_total_p50_us", "per-frame GPU scene+composite p50"),
        ("gpu_total_p95_us", "per-frame GPU scene+composite p95"),
        ("gap_p50_us", "per-frame CPU−GPU gap p50"),
        ("gap_p95_us", "per-frame CPU−GPU gap p95"),
        ("scene_cpu_p50_us", "per-frame scene CPU p50"),
        ("scene_cpu_p95_us", "per-frame scene CPU p95"),
        ("scene_gpu_p50_us", "per-frame scene GPU p50"),
        ("scene_gpu_p95_us", "per-frame scene GPU p95"),
        ("scene_gpu_zero", "frames with zero scene GPU draws"),
        ("scene_gap_p50_us", "per-frame scene CPU−GPU gap p50"),
        ("composite_cpu_p50_us", "per-frame composite CPU p50"),
        ("composite_cpu_p95_us", "per-frame composite CPU p95"),
        ("composite_gpu_p50_us", "per-frame composite GPU p50"),
        ("composite_gpu_p95_us", "per-frame composite GPU p95"),
        ("composite_gpu_zero", "frames with zero composite GPU draws"),
        ("composite_gap_p50_us", "per-frame composite CPU−GPU gap p50"),
    ):
        print(
            table_row(
                label,
                metric(left, "compositor", "frame_timeline", key),
                metric(right, "compositor", "frame_timeline", key),
                ""
                if key
                in (
                    "matched",
                    "pending",
                    "cpu_without_gpu",
                    "gpu_without_cpu",
                    "scene_gpu_zero",
                    "composite_gpu_zero",
                )
                else "µs",
            )
        )
    print()
    def per_pass(report, key):
        material = metric(report, "compositor", "material") or {}
        passes = material.get("material_scene_passes") or 0
        value = material.get(key) or 0
        return value / passes if passes else 0

    def per_composite(report, key):
        material = metric(report, "compositor", "material") or {}
        frames = material.get("composite_frames") or 0
        value = material.get(key) or 0
        return value / frames if frames else 0

    print("## Material scene bounding")
    print()
    print(f"| metric | {left_name} | {right_name} |")
    print("|---|---:|---:|")
    for key, label in (
        ("material_scene_passes", "offscreen scene passes"),
        ("material_scene_skips", "scene passes skipped"),
        ("material_scene_bytes", "scene texture bytes"),
        ("scene_cache_skip", "scene cache skip enabled (0/1)"),
        ("scene_elements_total", "scene elements offered"),
        ("scene_elements_kept", "scene elements kept by region"),
        ("composite_frames", "composite frames"),
        ("composite_elements", "composite elements submitted"),
        ("material_off", "material disabled (0/1)"),
    ):
        print(
            table_row(
                label,
                metric(left, "compositor", "material", key),
                metric(right, "compositor", "material", key),
                "",
            )
        )
    for key, label in (
        ("scene_elements_total", "scene elements offered / pass"),
        ("scene_elements_kept", "scene elements kept / pass"),
        ("composite_elements", "composite elements / frame"),
    ):
        left_value = (
            per_pass(left, key)
            if key != "composite_elements"
            else per_composite(left, key)
        )
        right_value = (
            per_pass(right, key)
            if key != "composite_elements"
            else per_composite(right, key)
        )
        print(table_row(label, left_value, right_value, ""))
    print(
        table_row(
            "region groups",
            metric(left, "compositor", "material_region_groups"),
            metric(right, "compositor", "material_region_groups"),
            "",
        )
    )
    print()
    print("## Composite element breakdown")
    print()
    print(f"| metric | {left_name} | {right_name} |")
    print("|---|---:|---:|")
    for key, label in (
        ("total_avg", "elements / frame avg"),
        ("total_max", "elements / frame max"),
        ("first_half_avg", "first-half avg"),
        ("second_half_avg", "second-half avg"),
        ("growth_pct", "second-half growth"),
        ("decoration_avg", "decoration / frame"),
        ("chrome_avg", "chrome / frame"),
        ("titlebar_avg", "titlebar / frame"),
        ("window_content_avg", "window content / frame"),
        ("window_shadow_avg", "window shadow / frame"),
        ("menu_avg", "menu / frame"),
        ("desktop_avg", "desktop / frame"),
        ("wallpaper_avg", "wallpaper / frame"),
        ("blur_avg", "blur / frame"),
        ("lock_avg", "lock / frame"),
    ):
        unit = "%" if key == "growth_pct" else ""
        print(
            table_row(
                label,
                metric(left, "compositor", "frame_elements", key),
                metric(right, "compositor", "frame_elements", key),
                unit,
            )
        )
    print()
    print("## Trace event counts")
    print()
    print(f"| event | {left_name} | {right_name} |")
    print("|---|---:|---:|")
    for key in (
        "target_committed",
        "target_deferred",
        "target_failed",
        "dmabuf_release",
        "closed_before_present",
    ):
        print(
            table_row(
                key,
                metric(left, "shell", "traces", "counts", key),
                metric(right, "shell", "traces", "counts", key),
                "",
            )
        )
    print()
    print(
        "The open interval starts at the shell's popover-open request and ends "
        "at the first frame callback for the committed Dock frame. It is not "
        "hardware input-to-photon; the trace carries the raw `t_ms` timestamps "
        "so a synthetic-input run can align them with the compositor's "
        "`DFTRACE` lines."
    )


def summary(paths):
    reports = []
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            reports.append(json.load(handle))

    def metric(report, *path):
        value = report
        for key in path:
            if not isinstance(value, dict):
                return None
            value = value.get(key)
        return value

    names = [
        (
            f"{report['label']} ({report['arm']})"
            if report.get("arm")
            else f"{report['label']} (stable={int(report['stable_popover'])})"
        )
        for report in reports
    ]
    print("# T238 Dock popover summary")
    print()
    print("| metric | " + " | ".join(names) + " |")
    print("|---|" + "---:|" * len(names))
    # A short run has fewer than ten openings, so `steady` is empty. Fall back
    # to `all` rather than reporting a misleading zero.
    def window(report):
        steady = metric(report, "shell", "traces", "steady", "open_us", "samples")
        return "steady" if steady else "all"

    rows = (
        ("open p50", ("shell", "traces", "window", "open_us", "p50")),
        ("open p95", ("shell", "traces", "window", "open_us", "p95")),
        (
            "actual present p50",
            ("shell", "traces", "window", "actual_present_us", "p50"),
        ),
        (
            "actual present p95",
            ("shell", "traces", "window", "actual_present_us", "p95"),
        ),
        (
            "commit→present p50",
            ("shell", "traces", "window", "present_after_commit_us", "p50"),
        ),
        (
            "commit→present p95",
            ("shell", "traces", "window", "present_after_commit_us", "p95"),
        ),
        ("compositor frame p50", ("compositor", "frame_timing", "p50_us")),
        ("compositor frame p95", ("compositor", "frame_timing", "p95_us")),
        ("prepare p50", ("compositor", "frame_stages", "prepare_p50_us")),
        ("scene p50", ("compositor", "frame_stages", "scene_p50_us")),
        ("blur p50", ("compositor", "frame_stages", "blur_p50_us")),
        ("composite p50", ("compositor", "frame_stages", "composite_p50_us")),
        ("submit p50", ("compositor", "frame_stages", "submit_p50_us")),
        ("callback p50", ("compositor", "frame_stages", "callback_p50_us")),
        (
            "per-frame CPU p50",
            ("compositor", "frame_timeline", "cpu_total_p50_us"),
        ),
        (
            "per-frame CPU p95",
            ("compositor", "frame_timeline", "cpu_total_p95_us"),
        ),
        (
            "per-frame GPU p50",
            ("compositor", "frame_timeline", "gpu_total_p50_us"),
        ),
        (
            "per-frame GPU p95",
            ("compositor", "frame_timeline", "gpu_total_p95_us"),
        ),
        ("joined frames", ("compositor", "frame_timeline", "matched")),
        (
            "unmatched CPU frames",
            ("compositor", "frame_timeline", "cpu_without_gpu"),
        ),
        (
            "unmatched GPU samples",
            ("compositor", "frame_timeline", "gpu_without_cpu"),
        ),
        (
            "scene passes",
            ("compositor", "material", "material_scene_passes"),
        ),
        (
            "scene passes skipped",
            ("compositor", "material", "material_scene_skips"),
        ),
        (
            "scene texture bytes",
            ("compositor", "material", "material_scene_bytes"),
        ),
        (
            "scene elements kept",
            ("compositor", "material", "scene_elements_kept"),
        ),
        (
            "scene elements kept/pass",
            ("compositor", "material", "scene_elements_kept_per_pass"),
        ),
        (
            "composite elements",
            ("compositor", "material", "composite_elements"),
        ),
        (
            "composite elements/frame",
            ("compositor", "material", "composite_elements_per_frame"),
        ),
        (
            "elements/frame (traced)",
            ("compositor", "frame_elements", "total_avg"),
        ),
        (
            "element growth %",
            ("compositor", "frame_elements", "growth_pct"),
        ),
        (
            "decoration/frame",
            ("compositor", "frame_elements", "decoration_avg"),
        ),
        (
            "shadow/frame",
            ("compositor", "frame_elements", "window_shadow_avg"),
        ),
        (
            "titlebar/frame",
            ("compositor", "frame_elements", "titlebar_avg"),
        ),
        ("scene CPU p50", ("compositor", "frame_timeline", "scene_cpu_p50_us")),
        ("scene GPU p50", ("compositor", "frame_timeline", "scene_gpu_p50_us")),
        (
            "scene GPU-zero frames",
            ("compositor", "frame_timeline", "scene_gpu_zero"),
        ),
        (
            "composite CPU p50",
            ("compositor", "frame_timeline", "composite_cpu_p50_us"),
        ),
        (
            "composite GPU p50",
            ("compositor", "frame_timeline", "composite_gpu_p50_us"),
        ),
        (
            "composite gap p50",
            ("compositor", "frame_timeline", "composite_gap_p50_us"),
        ),
        ("imports", ("shell", "traces", "imports", "all", "count")),
        ("ring exhausted", ("shell", "stage", "ring_exhausted")),
        ("FBO bytes live", ("shell", "stage", "fbo_bytes_live")),
    )
    for label, path in rows:
        if "window" in path:
            values = []
            for report in reports:
                resolved = tuple(window(report) if key == "window" else key for key in path)
                values.append(fmt(metric(report, *resolved)))
        else:
            values = [fmt(metric(report, *path)) for report in reports]
        print(f"| {label} | " + " | ".join(values) + " |")
    print()
    print("All time values are microseconds.")


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    run_parser = sub.add_parser("run")
    run_parser.add_argument("--socket", required=True)
    run_parser.add_argument("--shell-pid", type=int, required=True)
    run_parser.add_argument("--compositor-pid", type=int, required=True)
    run_parser.add_argument("--shell-log", required=True)
    run_parser.add_argument("--compositor-log", required=True)
    run_parser.add_argument("--out", required=True)
    run_parser.add_argument("--label", required=True)
    run_parser.add_argument("--stable", type=int, choices=(0, 1), required=True)
    run_parser.add_argument("--arm", default="")
    run_parser.add_argument("--cycles", type=int, default=100)
    run_parser.add_argument("--commit", default="")
    run_parser.add_argument("--timeout", type=float, default=180.0)
    run_parser.add_argument("--settle", type=float, default=1.0)

    compare_parser = sub.add_parser("compare")
    compare_parser.add_argument("left")
    compare_parser.add_argument("right")

    summary_parser = sub.add_parser("summary")
    summary_parser.add_argument("reports", nargs="+")

    parse_parser = sub.add_parser("parse")
    parse_parser.add_argument("--shell-log", required=True)
    parse_parser.add_argument("--compositor-log", required=True)
    parse_parser.add_argument("--out", required=True)
    parse_parser.add_argument("--label", required=True)
    parse_parser.add_argument("--stable", type=int, choices=(0, 1), required=True)
    parse_parser.add_argument("--arm", default="")
    parse_parser.add_argument("--cycles", type=int, default=100)
    parse_parser.add_argument("--commit", default="")

    args = parser.parse_args()
    if args.command == "run":
        return run(args)
    if args.command == "parse":
        return parse_logs(args)
    if args.command == "summary":
        summary(args.reports)
        return 0
    compare(args.left, args.right)
    return 0


if __name__ == "__main__":
    sys.exit(main())
