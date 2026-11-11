#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the T238 material workload benchmark and summarise the results.

The shell script starts one release compositor + shell per (arm, workload)
pair. This driver launches the workload clients, injects synthetic input where
a client cannot produce the traffic, samples RSS, dumps SIGUSR1 stats, and
writes one JSON report.

Usage:
  t238-material-workloads-driver.py run --socket S --synth P --display D ...
  t238-material-workloads-driver.py summary reports/*.json
"""
import argparse
import importlib.util
import json
import os
import re
import signal
import socket
import subprocess
import sys
import threading
import time


def load_ab_driver():
    path = os.path.join(os.path.dirname(__file__), "t238-dock-popover-ab-driver.py")
    spec = importlib.util.spec_from_file_location("t238_ab_driver", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


AB = load_ab_driver()


def log(message):
    print(f"t238-material-workloads: {message}", flush=True)


class Synth:
    def __init__(self, path):
        self.path = path
        runtime = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
        self.local = os.path.join(runtime, f"t238-workload-{os.getpid()}.sock")
        try:
            os.unlink(self.local)
        except OSError:
            pass
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.sock.bind(self.local)
        self.sock.settimeout(0.2)

    def send(self, line):
        self.sock.sendto((line + "\n").encode(), self.path)

    def query(self, line, timeout=1.0):
        self.sock.settimeout(timeout)
        self.sock.sendto((line + "\n").encode(), self.path)
        lines = []
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                data, _ = self.sock.recvfrom(65535)
            except socket.timeout:
                break
            for text in data.decode(errors="replace").splitlines():
                if text == "end":
                    return lines
                lines.append(text)
        return lines

    def close(self):
        self.sock.close()
        try:
            os.unlink(self.local)
        except OSError:
            pass


def rss_kb(pid):
    try:
        with open(f"/proc/{pid}/status", encoding="utf-8") as handle:
            for line in handle:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1])
    except OSError:
        pass
    return 0


class RssSampler:
    def __init__(self, pid, interval=0.05):
        self.pid = pid
        self.interval = interval
        self.max_kb = 0
        self.last_kb = 0
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def _run(self):
        while not self._stop.is_set():
            value = rss_kb(self.pid)
            self.max_kb = max(self.max_kb, value)
            self.last_kb = value
            self._stop.wait(self.interval)

    def start(self):
        self._thread.start()

    def stop(self):
        self._stop.set()
        self._thread.join(timeout=1.0)
        return self.max_kb, self.last_kb


def spawn_xmessage(display, index, count, logical):
    # Tile without overlap so the damage tracker cannot cull whole windows:
    # the element-count scaling must reflect the window count, not occlusion.
    width, height = logical
    columns = max(1, min(count, 6))
    rows = max(1, (count + columns - 1) // columns)
    cell_w = max(160, width // columns)
    cell_h = max(140, height // rows)
    x = (index % columns) * cell_w + 8
    y = (index // columns) * cell_h + 40
    w = max(140, cell_w - 24)
    h = max(90, cell_h - 60)
    title = f"T238 workload window {index}"
    env = dict(os.environ)
    env["DISPLAY"] = display
    return subprocess.Popen(
        ["xmessage", "-geometry", f"{w}x{h}+{x}+{y}", "-title", title, title],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def spawn_scroll(display, duration_s):
    env = dict(os.environ)
    env["DISPLAY"] = display
    path = os.path.join(os.path.dirname(__file__), "t238-scroll-client.py")
    return subprocess.Popen(
        [sys.executable, path, str(int(duration_s * 1000))],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def spawn_video(display, duration_s):
    env = dict(os.environ)
    env["DISPLAY"] = display
    return subprocess.Popen(
        [
            "ffplay",
            "-loglevel",
            "error",
            "-an",
            "-noborder",
            "-window_title",
            "T238 Video",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=60",
            "-t",
            f"{duration_s:.1f}",
            "-autoexit",
        ],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def parse_decorations(synth):
    windows = []
    for line in synth.query("query decorations"):
        parts = line.split()
        if not parts or parts[0] != "decoration" or len(parts) < 13:
            continue
        try:
            windows.append(
                {
                    "id": int(parts[1]),
                    "ssd": int(parts[2]),
                    "titlebar": tuple(int(value) for value in parts[3:7]),
                    "content": tuple(int(value) for value in parts[7:11]),
                    "state": parts[11],
                    "space": parts[12],
                }
            )
        except ValueError:
            continue
    return windows


def output_sizes(synth):
    for line in synth.query("query scale"):
        match = re.search(
            r"scale output \S+ physical=(\d+)x(\d+) logical=(\d+)x(\d+)", line
        )
        if match:
            physical = (int(match.group(1)), int(match.group(2)))
            logical = (int(match.group(3)), int(match.group(4)))
            return physical, logical
    return None


def output_logical_size(synth):
    sizes = output_sizes(synth)
    return sizes[1] if sizes else None


def drag_window(synth, window, logical, duration_s):
    width, height = logical
    tx, ty, tw, th = window["titlebar"]
    x = (tx + tw / 2) / width
    y = (ty + th / 2) / height
    synth.send(f"motion-abs {x:.6f} {y:.6f}")
    time.sleep(0.05)
    synth.send("button 272 down")
    end = time.monotonic() + duration_s
    step = 0
    while time.monotonic() < end:
        direction = 2 if (step // 30) % 2 == 0 else -2
        synth.send(f"motion {direction} {direction // 2}")
        step += 1
        time.sleep(1.0 / 60.0)
    synth.send("button 272 up")


def resize_window(synth, window, logical, duration_s):
    width, height = logical
    cx, cy, cw, ch = window["content"]
    # The SSD frame's bottom-right corner is inside the content bounds; the
    # hit test rejects the exclusive outer edge.
    x = (cx + cw - 3) / width
    y = (cy + ch - 3) / height
    synth.send(f"motion-abs {x:.6f} {y:.6f}")
    time.sleep(0.05)
    synth.send("button 272 down")
    end = time.monotonic() + duration_s
    step = 0
    while time.monotonic() < end:
        direction = 2 if (step // 30) % 2 == 0 else -2
        synth.send(f"motion {direction} {direction}")
        step += 1
        time.sleep(1.0 / 60.0)
    synth.send("button 272 up")


def focus_cycle(synth, windows, logical, duration_s):
    """Click each window's titlebar in turn: focus changes the titlebar chrome
    and the window shadow treatment without moving any surface."""
    width, height = logical
    if not windows:
        return
    end = time.monotonic() + duration_s
    index = 0
    while time.monotonic() < end:
        window = windows[index % len(windows)]
        tx, ty, tw, th = window["titlebar"]
        x = (tx + tw / 2) / width
        y = (ty + th / 2) / height
        synth.send(f"motion-abs {x:.6f} {y:.6f}")
        synth.send("button 272 down")
        synth.send("button 272 up")
        index += 1
        time.sleep(0.25)


def mission_control(synth, duration_s):
    # Ctrl+Up opens Mission Control; Ctrl+Up again closes it.
    synth.send("key 29 down")
    synth.send("key 103 down")
    synth.send("key 103 up")
    synth.send("key 29 up")
    time.sleep(0.4)
    end = time.monotonic() + duration_s
    step = 0
    while time.monotonic() < end:
        direction = 2 if (step // 40) % 2 == 0 else -2
        synth.send(f"motion {direction} 0")
        step += 1
        time.sleep(1.0 / 60.0)
    synth.send("key 29 down")
    synth.send("key 103 down")
    synth.send("key 103 up")
    synth.send("key 29 up")


def scheme_pulse(synth, duration_s):
    end = time.monotonic() + duration_s
    light = True
    while time.monotonic() < end:
        synth.send("set color-scheme light" if light else "set color-scheme dark")
        light = not light
        time.sleep(0.1)


def run_workload(args, synth, display):
    sizes = output_sizes(synth)
    if sizes is None:
        physical, logical = (1920, 1200), (1920, 1200)
    else:
        physical, logical = sizes
    windows = parse_decorations(synth)
    floating = [window for window in windows if window["ssd"] and window["state"] == "floating"]
    target = floating[0] if floating else (windows[0] if windows else None)
    before = {window["id"]: window for window in windows}
    effective = True
    if args.workload == "scroll":
        spawn_scroll(display, args.duration)
        # Input-driven scroll on top of the client's own repaint.
        end = time.monotonic() + args.duration
        while time.monotonic() < end:
            synth.send("axis 0 12")
            time.sleep(1.0 / 60.0)
    elif args.workload == "drag":
        if target is None:
            effective = False
        else:
            drag_window(synth, target, logical, args.duration)
    elif args.workload == "resize":
        if target is None:
            effective = False
        else:
            resize_window(synth, target, logical, args.duration)
    elif args.workload == "video":
        child = spawn_video(display, args.duration)
        time.sleep(args.duration)
        child.terminate()
        try:
            child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            child.kill()
    elif args.workload == "focus":
        focus_cycle(synth, windows, logical, args.duration)
    elif args.workload == "mission-control":
        mission_control(synth, args.duration)
    elif args.workload == "scheme-pulse":
        scheme_pulse(synth, args.duration)
    elif args.workload == "hover":
        if target is None:
            effective = False
        else:
            tx, ty, tw, th = target["titlebar"]
            # `titlebar_hover_at` requires the pointer over the traffic-light
            # cluster, not the whole titlebar. The first light centre is the
            # chrome inset plus half a button.
            x = (tx + 16) / logical[0]
            y = (ty + 16) / logical[1]
            end = time.monotonic() + args.duration
            inside = True
            while time.monotonic() < end:
                synth.send(f"motion-abs {x:.6f} {y:.6f}" if inside else "motion-abs 0.001 0.001")
                inside = not inside
                time.sleep(0.25)
    elif args.workload == "static":
        time.sleep(args.duration)
    else:
        raise SystemExit(f"unknown workload {args.workload}")

    after = {window["id"]: window for window in parse_decorations(synth)}
    geometry_changed = before != after
    if args.workload in ("drag", "resize") and not geometry_changed:
        effective = False
    return {
        "effective": effective,
        "geometry_changed": geometry_changed,
        "window_count": len(after),
        "physical_size": physical,
        "logical_size": logical,
    }


def scene_epoch(synth):
    for line in synth.query("query scene"):
        if line.startswith("scene "):
            return AB.numeric_fields(line).get("epoch", 0)
    return 0


def wait_for_line(path, needle, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                if needle in handle.read():
                    return True
        except OSError:
            pass
        time.sleep(0.1)
    return False


def run(args):
    if not os.path.exists(args.synth):
        log(f"synthetic socket missing: {args.synth}")
        return 1
    synth = Synth(args.synth)
    logical = output_logical_size(synth) or (1920, 1200)
    children = [
        spawn_xmessage(args.display, index, args.windows, logical)
        for index in range(args.windows)
    ]
    # Let the shell/Dock and the windows map before measuring. Without this the
    # first half of a short run is startup/no-material frames.
    time.sleep(args.warmup)
    if args.pin_tier:
        synth.send(f"set degrade-tier {args.pin_tier}")
        time.sleep(0.3)
    if args.capture_ab:
        # Same compositor, same windows, same focus: only the shadow path
        # changes between the two captures. This is the visual-equivalence
        # evidence the span/shader comparison needs.
        os.makedirs(args.capture_ab, exist_ok=True)
        for name, enabled in (("shadow-off", "off"), ("shadow-on", "on")):
            synth.send(f"set shadow-gpu {enabled}")
            time.sleep(0.4)
            path = os.path.join(os.path.abspath(args.capture_ab), f"{name}.png")
            synth.send(f"capture {path}")
            time.sleep(0.8)
        for child in children:
            child.terminate()
        synth.close()
        log(f"capture A/B wrote {args.capture_ab}")
        return 0
    sampler = RssSampler(args.compositor_pid)
    sampler.start()
    scene_before = scene_epoch(synth)
    started = time.monotonic()
    try:
        workload = run_workload(args, synth, args.display)
        elapsed = time.monotonic() - started
        # Snapshot before tearing the workload windows down, so post-workload
        # frames cannot dominate the element count.
        time.sleep(0.3)
        if args.capture:
            synth.send(f"capture {os.path.abspath(args.capture)}")
            time.sleep(0.6)
        try:
            os.kill(args.compositor_pid, signal.SIGUSR1)
        except ProcessLookupError:
            pass
        try:
            os.kill(args.shell_pid, signal.SIGUSR1)
        except ProcessLookupError:
            pass
        wait_for_line(args.compositor_log, "frame timeline (SIGUSR1-1):", 5.0)
        time.sleep(0.2)
        scene_after = scene_epoch(synth)
        max_rss, last_rss = sampler.stop()
    finally:
        for child in children:
            child.terminate()
        for child in children:
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                child.kill()
    synth.close()

    compositor_text = read(args.compositor_log)
    shell_text = read(args.shell_log)
    # The SIGUSR1 dump line separates the measured frames from teardown.
    trace_text = compositor_text.split("frame timeline (SIGUSR1-1):", 1)[0]
    compositor = AB.parse_compositor(compositor_text)
    compositor["frame_elements"] = AB.summarise_frame_elements(
        AB.parse_frame_elements(trace_text)
    )
    render = AB.numeric_fields(AB.last_line(compositor_text, "render stats (SIGUSR1-1):"))
    rendered = render.get("frames_rendered", 0)
    skipped = render.get("frames_skipped_no_damage", 0)
    accounting_line = AB.last_line(
        compositor_text, "nested frame accounting (SIGUSR1-1):"
    )
    accounting = AB.numeric_fields(accounting_line)
    damage_frames = accounting.get("damage", 0)
    no_damage_swaps = accounting.get("no_damage_swaps", skipped)
    if damage_frames or no_damage_swaps:
        dirty_pct = (
            100.0 * damage_frames / (damage_frames + no_damage_swaps)
            if (damage_frames + no_damage_swaps)
            else 0.0
        )
    else:
        dirty_pct = (
            100.0 * rendered / (rendered + skipped) if (rendered + skipped) else 0.0
        )
    source_match = re.search(r"presentation_source=(\S+)", accounting_line or "")
    if source_match is None:
        source_match = re.search(
            r"presentation provenance \(SIGUSR1-1\): source=(\S+)",
            compositor_text,
        )
    superseded = AB.numeric_fields(
        AB.last_line(compositor_text, "superseded ages (SIGUSR1-1):")
    )
    latency = AB.numeric_fields(AB.last_line(compositor_text, "latency stats (SIGUSR1-1):"))
    presents = [
        event
        for event in AB.parse_dftrace(trace_text)
        if event["event"] == "present"
    ]
    intervals = [
        presents[index]["t_ms"] - presents[index - 1]["t_ms"]
        for index in range(1, len(presents))
    ]
    report = {
        "label": args.label,
        "arm": args.arm,
        "workload": args.workload,
        "duration_s": args.duration,
        "elapsed_s": elapsed,
        "commit": args.commit,
        "effective": workload["effective"],
        "geometry_changed": workload["geometry_changed"],
        "window_count": workload["window_count"],
        "requested_windows": args.windows,
        "pinned_tier": args.pin_tier,
        "physical_size": workload["physical_size"],
        "logical_size": workload["logical_size"],
        "dirty_pct": dirty_pct,
        "frames_rendered": rendered,
        "frames_skipped": skipped,
        "present_stalls": render.get("present_stalls", 0),
        "frame_accounting": accounting,
        "frame_attempts": accounting.get("attempts", 0),
        "damage_frames": damage_frames,
        "no_damage_swaps": no_damage_swaps,
        "frames_submitted": accounting.get("submitted", 0),
        "swaps_completed": accounting.get("completed", rendered),
        "swaps_failed": accounting.get("failed", 0),
        "frames_coalesced": accounting.get("coalesced", 0),
        "frames_superseded": accounting.get("superseded", 0),
        "superseded": superseded,
        "superseded_samples": superseded.get("samples", 0),
        "superseded_damage": superseded.get("damage", 0),
        "superseded_no_damage": superseded.get("no_damage", 0),
        "superseded_submit_p50_us": superseded.get("submit_p50_us", 0),
        "superseded_submit_p95_us": superseded.get("submit_p95_us", 0),
        "superseded_completion_p50_us": superseded.get("completion_p50_us", 0),
        "superseded_completion_p95_us": superseded.get("completion_p95_us", 0),
        "superseded_completion_max_us": superseded.get("completion_max_us", 0),
        "superseded_newer_p50_us": superseded.get("newer_p50_us", 0),
        "superseded_newer_p95_us": superseded.get("newer_p95_us", 0),
        "presentation_feedback_sent": accounting.get("feedback_sent", 0),
        "presentation_source": source_match.group(1) if source_match else None,
        "latency": latency,
        "latency_last_us": latency.get("last_us", 0),
        "latency_max_us": latency.get("max_us", 0),
        "latency_samples": latency.get("samples", 0),
        "rss_max_kb": max_rss,
        "rss_last_kb": last_rss,
        "scene_epoch_start": scene_before,
        "scene_epoch_end": scene_after,
        "scene_epoch_delta": max(0, scene_after - scene_before),
        "present_samples": len(presents),
        "present_interval_ms": AB.timing(intervals),
        "compositor": compositor,
        "shell": AB.parse_shell(shell_text, compositor_text),
    }
    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    log(f"wrote {args.out}")
    return 0


def read(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as handle:
            return handle.read()
    except OSError:
        return ""


def metric(report, *path):
    value = report
    for key in path:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def median(values):
    values = sorted(value for value in values if value is not None)
    if not values:
        return None
    mid = len(values) // 2
    if len(values) % 2:
        return values[mid]
    return (values[mid - 1] + values[mid]) / 2.0


def group_reports(reports):
    groups = {}
    for report in reports:
        key = (
            report.get("requested_windows", 0),
            report["workload"],
            report["arm"],
        )
        groups.setdefault(key, []).append(report)
    return groups


def med(group, *path):
    return median([metric(report, *path) for report in group])


def med_dict(group, *path):
    """Median every key of a nested metric dict across the group's trials."""
    dicts = [metric(report, *path) for report in group]
    dicts = [value for value in dicts if isinstance(value, dict)]
    if not dicts:
        return {}
    keys = set()
    for value in dicts:
        keys.update(value.keys())
    return {key: median([value.get(key) for value in dicts]) for key in keys}


def med_field(group, key):
    return median([report.get(key) for report in group])


def spread(group, *path):
    values = [metric(report, *path) for report in group]
    values = [value for value in values if value is not None]
    if len(values) < 2:
        return ""
    return f"{AB.fmt(min(values))}–{AB.fmt(max(values))}"


def summary(paths):
    reports = []
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            reports.append(json.load(handle))
    groups = group_reports(reports)
    order = [
        "static",
        "scroll",
        "drag",
        "resize",
        "video",
        "mission-control",
        "scheme-pulse",
    ]
    keys = sorted(
        groups,
        key=lambda key: (
            key[0],
            order.index(key[1]) if key[1] in order else len(order),
            key[1],
            key[2],
        ),
    )
    print("# T238 material workload comparison")
    print()
    print(
        "| windows | workload | arm | trials | frame p50 (spread) | frame p95 | frame p99 | "
        "dirty % | scene build | scene CPU | composite CPU | composite GPU | "
        "scene bytes | RSS max | present p50 | present p95 | skips | epoch Δ |"
    )
    print(
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        timing = med_dict(group, "compositor", "frame_timing") or {}
        stages = med_dict(group, "compositor", "frame_stages") or {}
        timeline = med_dict(group, "compositor", "frame_timeline") or {}
        material = med_dict(group, "compositor", "material") or {}
        present = med_dict(group, "present_interval_ms") or {}
        print(
            f"| {windows} | {workload} | {arm} | {len(group)} | "
            f"{AB.fmt(timing.get('p50_us'))} ({spread(group, 'compositor', 'frame_timing', 'p50_us')}) | "
            f"{AB.fmt(timing.get('p95_us'))} | {AB.fmt(timing.get('p99_us'))} | "
            f"{med_field(group, 'dirty_pct') or 0:.1f} | "
            f"{AB.fmt(stages.get('scene_build_p50_us'))} | "
            f"{AB.fmt(timeline.get('scene_cpu_p50_us'))} | "
            f"{AB.fmt(timeline.get('composite_cpu_p50_us'))} | "
            f"{AB.fmt(timeline.get('composite_gpu_p50_us'))} | "
            f"{AB.fmt(material.get('material_scene_bytes'))} | "
            f"{(med_field(group, 'rss_max_kb') or 0) / 1024:.1f} MB | "
            f"{AB.fmt(present.get('p50'))} | {AB.fmt(present.get('p95'))} | "
            f"{AB.fmt(material.get('material_scene_skips'))} | "
            f"{AB.fmt(med_field(group, 'scene_epoch_delta'))} |"
        )
    print()
    print("## Frame accounting")
    print()
    print(
        "| windows | workload | arm | attempts | damage | no-damage swaps | "
        "damage/s | completed/s | coalesced | superseded | stale % | "
        "failed | feedback sent | source |"
    )
    print(
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|"
    )
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        duration = med_field(group, "duration_s") or 0
        attempts = med_field(group, "frame_attempts")
        damage = med_field(group, "damage_frames")
        no_damage = med_field(group, "no_damage_swaps")
        completed = med_field(group, "swaps_completed")
        coalesced = med_field(group, "frames_coalesced")
        superseded = med_field(group, "frames_superseded")
        failed = med_field(group, "swaps_failed")
        feedback = med_field(group, "presentation_feedback_sent")
        source = group[0].get("presentation_source") or "-"
        rate = lambda value: f"{value / duration:.1f}" if duration and value is not None else "-"
        stale_pct = (
            100.0 * superseded / damage if damage else 0.0
        )
        print(
            f"| {windows} | {workload} | {arm} | {AB.fmt(attempts)} | "
            f"{AB.fmt(damage)} | {AB.fmt(no_damage)} | {rate(damage)} | "
            f"{rate(completed)} | {AB.fmt(coalesced)} | "
            f"{AB.fmt(superseded)} | {stale_pct:.1f} | "
            f"{AB.fmt(failed)} | {AB.fmt(feedback)} | "
            f"{source} |"
        )
    print()
    print(
        "Counts are lifetime totals for the measured interval; `/s` rates use "
        "the workload duration. `completed` counts swaps (damage and wake-only), "
        "not physically presented frames. `presentation_source=nested-swap-completion` "
        "means the nested rail cannot reach the host's wp_presentation; swap "
        "completion is the host frame-callback wait, not a vblank timestamp."
    )
    print()
    print("## Superseded frame age")
    print()
    print(
        "| windows | workload | arm | samples | damage | wake-only | "
        "submit p50/p95 us | completion p50/p95 us | newer p50/p95 us | max us |"
    )
    print(
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|"
    )
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        print(
            f"| {windows} | {workload} | {arm} | "
            f"{AB.fmt(med_field(group, 'superseded_samples'))} | "
            f"{AB.fmt(med_field(group, 'superseded_damage'))} | "
            f"{AB.fmt(med_field(group, 'superseded_no_damage'))} | "
            f"{AB.fmt(med_field(group, 'superseded_submit_p50_us'))}/"
            f"{AB.fmt(med_field(group, 'superseded_submit_p95_us'))} | "
            f"{AB.fmt(med_field(group, 'superseded_completion_p50_us'))}/"
            f"{AB.fmt(med_field(group, 'superseded_completion_p95_us'))} | "
            f"{AB.fmt(med_field(group, 'superseded_newer_p50_us'))}/"
            f"{AB.fmt(med_field(group, 'superseded_newer_p95_us'))} | "
            f"{AB.fmt(med_field(group, 'superseded_completion_max_us'))} |"
        )
    print()
    print(
        "Ages are measured from the frame's build start. `submit` is when the "
        "frame was handed to the presenter; `completion` is when the host swap "
        "returned. `newer` is how long the newer scene had existed at swap "
        "completion. The nested rail's completion is the host frame callback, "
        "not a physical vblank; the host may present later."
    )
    print()
    print("## Input latency")
    print()
    print("| windows | workload | arm | last us | max us | samples | dropped |")
    print("|---|---|---|---:|---:|---:|---:|")
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        print(
            f"| {windows} | {workload} | {arm} | "
            f"{AB.fmt(med_field(group, 'latency_last_us'))} | "
            f"{AB.fmt(med_field(group, 'latency_max_us'))} | "
            f"{AB.fmt(med_field(group, 'latency_samples'))} | "
            f"{AB.fmt((med_dict(group, 'latency') or {}).get('dropped'))} |"
        )
    print()
    print(
        "`max` is the lifetime maximum and can include a startup/first-input "
        "sample; `last` is the most recent sample. `dropped` must stay 0."
    )
    print()
    print("## Element breakdown")
    print()
    print(
        "| windows | workload | arm | material % | elements p50 | elements p95 | "
        "min–max | decoration | shadow | titlebar | fill | lights | glyphs | "
        "content | chrome | growth % |"
    )
    print(
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
    )
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        elements = med_dict(group, "compositor", "frame_elements") or {}
        print(
            f"| {windows} | {workload} | {arm} | "
            f"{(elements.get('material_frame_pct') or 0):.1f} | "
            f"{AB.fmt(elements.get('total_p50'))} | "
            f"{AB.fmt(elements.get('total_p95'))} | "
            f"{AB.fmt(elements.get('total_min'))}–{AB.fmt(elements.get('total_max'))} | "
            f"{AB.fmt(elements.get('decoration_avg'))} | "
            f"{AB.fmt(elements.get('window_shadow_avg'))} | "
            f"{AB.fmt(elements.get('titlebar_avg'))} | "
            f"{AB.fmt(elements.get('titlebar_fill_avg'))} | "
            f"{AB.fmt(elements.get('titlebar_lights_avg'))} | "
            f"{AB.fmt(elements.get('titlebar_glyphs_avg'))} | "
            f"{AB.fmt(elements.get('window_content_avg'))} | "
            f"{AB.fmt(elements.get('chrome_avg'))} | "
            f"{(elements.get('growth_pct') or 0):.1f} |"
        )
    print()
    print("## Shaded decoration area")
    print()
    print(
        "| windows | workload | arm | output px | shadow px | shadow % | "
        "shadow span px | titlebar px | titlebar % |"
    )
    print("|---|---|---|---:|---:|---:|---:|---:|---:|")
    for key in keys:
        group = groups[key]
        windows, workload, arm = key
        elements = med_dict(group, "compositor", "frame_elements") or {}
        physical = group[0].get("physical_size") or [0, 0]
        if not physical[0] or not physical[1]:
            # Older reports predate the physical-size field; an area ratio
            # against an unknown output would be meaningless.
            continue
        output_px = int(physical[0]) * int(physical[1])
        shadow = elements.get("shadow_area_avg") or 0
        span = elements.get("shadow_span_area_avg") or 0
        titlebar = elements.get("titlebar_area_avg") or 0
        print(
            f"| {windows} | {workload} | {arm} | {output_px} | "
            f"{AB.fmt(shadow)} | {100.0 * shadow / output_px:.2f} | "
            f"{AB.fmt(span)} | {AB.fmt(titlebar)} | "
            f"{100.0 * titlebar / output_px:.3f} |"
        )
    print()
    print(
        "Medians across the listed trials. All time values are microseconds. "
        "`dirty %` is damage / (damage + wake-only swaps). `scene build` is "
        "the CPU time to construct the element list (geometry), separate from "
        "the GL submission. `scene bytes` is the material scene texture "
        "allocation. RSS is the compositor process peak over the run."
    )


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    run_parser = sub.add_parser("run")
    run_parser.add_argument("--socket", required=True)
    run_parser.add_argument("--synth", required=True)
    run_parser.add_argument("--display", required=True)
    run_parser.add_argument("--shell-pid", type=int, required=True)
    run_parser.add_argument("--compositor-pid", type=int, required=True)
    run_parser.add_argument("--shell-log", required=True)
    run_parser.add_argument("--compositor-log", required=True)
    run_parser.add_argument("--out", required=True)
    run_parser.add_argument("--label", required=True)
    run_parser.add_argument("--arm", required=True)
    run_parser.add_argument("--workload", required=True)
    run_parser.add_argument("--duration", type=float, default=5.0)
    run_parser.add_argument("--warmup", type=float, default=1.5)
    run_parser.add_argument("--windows", type=int, default=2)
    run_parser.add_argument("--pin-tier", default="")
    run_parser.add_argument("--capture", default="")
    run_parser.add_argument("--capture-ab", default="")
    run_parser.add_argument("--commit", default="")

    summary_parser = sub.add_parser("summary")
    summary_parser.add_argument("reports", nargs="+")

    args = parser.parse_args()
    if args.command == "run":
        return run(args)
    summary(args.reports)
    return 0


if __name__ == "__main__":
    sys.exit(main())
