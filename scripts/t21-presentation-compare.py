#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Compare the committed T183 chrome baseline with the v0.1.1 benchmark run.

The T183 baseline (``docs/captures/t21-chrome-baseline.txt``) used the same
nested scenario names but a shorter scenario window and the old
offscreen-QPA + fresh-SHM transport. The v0.1.1 run is the T192b/T192c
benchmark. This script reports deltas rather than absolute numbers, and keeps
the transport mix separate: the baseline is 100% SHM, while the v0.1.1 rows
are dmabuf with a counted SHM fallback (never averaged into the dmabuf row).

The baseline and current frame-time columns are both the compositor's
windowed ``frame timing`` line. The current report also carries a
per-scenario frame-trace delta; that is not used for the baseline delta
because T183 has no frame-trace samples.
"""
import argparse
import importlib.util
import os
import re
import sys


def load_report_module():
    path = os.path.join(os.path.dirname(__file__), "t21-presentation-bench-report.py")
    spec = importlib.util.spec_from_file_location("t21_bench_report", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


REPORT = load_report_module()

BASELINE_HEADING = re.compile(
    r"^### (?P<name>\S+) \(SIGUSR1-(?P<seq>\d+), (?P<elapsed>[0-9.]+) s\)"
)
SHELL_DELTA = re.compile(
    r"shell delta: scene_frames=(?P<scene_frames>\d+) commits=(?P<commits>\d+) "
    r"grab_renders=(?P<grab_renders>\d+) shm_allocs=(?P<shm_allocs>\d+) "
    r"shm_bytes=(?P<shm_bytes>\d+) frame_callbacks=(?P<frame_callbacks>\d+)"
)
SHELL_TIMING = re.compile(
    r"shell timing: commit_us_samples=(?P<samples>\d+) "
    r"commit_us_p50=(?P<p50>\d+) commit_us_p95=(?P<p95>\d+) "
    r"commit_us_p99=(?P<p99>\d+) commit_us_max=(?P<max>\d+)"
)
COMPOSITOR_DELTA = re.compile(
    r"compositor delta: chrome_shm_imports=(?P<shm_imports>\d+) "
    r"chrome_shm_bytes=(?P<shm_bytes>\d+) chrome_dmabuf_imports=(?P<dmabuf_imports>\d+) "
    r"chrome_frame_callbacks=(?P<frame_callbacks>\d+)"
)
FRAME_TIMING = re.compile(
    r"frame timing \(SIGUSR1-\d+\): samples=(?P<samples>\d+) "
    r"samples_total=(?P<samples_total>\d+) avg_us=(?P<avg>\d+) "
    r"p50_us=(?P<p50>\d+) p95_us=(?P<p95>\d+) p99_us=(?P<p99>\d+) "
    r"max_us=(?P<max>\d+)"
)


def parse_baseline(path):
    scenarios = {}
    current = None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            match = BASELINE_HEADING.match(line)
            if match:
                current = {
                    "name": match.group("name"),
                    "seq": int(match.group("seq")),
                    "elapsed": float(match.group("elapsed")),
                }
                scenarios[current["name"]] = current
                continue
            if line.startswith("## "):
                current = None
                continue
            if current is None:
                continue
            match = SHELL_DELTA.search(line)
            if match:
                current.update({key: int(value) for key, value in match.groupdict().items()})
                continue
            match = SHELL_TIMING.search(line)
            if match:
                current.update(
                    {
                        "commit_samples": int(match.group("samples")),
                        "commit_p50": int(match.group("p50")),
                        "commit_p95": int(match.group("p95")),
                        "commit_p99": int(match.group("p99")),
                    }
                )
                continue
            match = COMPOSITOR_DELTA.search(line)
            if match:
                current.update(
                    {
                        "shm_imports": int(match.group("shm_imports")),
                        "shm_bytes": int(match.group("shm_bytes")),
                        "dmabuf_imports": int(match.group("dmabuf_imports")),
                        "frame_callbacks": int(match.group("frame_callbacks")),
                    }
                )
                continue
            match = FRAME_TIMING.search(line)
            if match:
                current.update(
                    {
                        "frame_p50": int(match.group("p50")),
                        "frame_p95": int(match.group("p95")),
                        "frame_p99": int(match.group("p99")),
                    }
                )
    return scenarios


def parse_current(raw_root, main_name):
    directory = os.path.join(raw_root, main_name)
    comp_log = os.path.join(directory, "compositor.txt")
    shell_log = os.path.join(directory, "shell.txt")
    driver_log = os.path.join(directory, "driver.txt")
    for path in (comp_log, shell_log, driver_log):
        if not os.path.exists(path):
            raise SystemExit(f"t21-presentation-compare: missing {path}")
    parsed = REPORT.parse(comp_log)
    parsed_shell = REPORT.parse(shell_log)
    parsed["shell"] = parsed_shell["shell"]
    parsed["surfaces"] = parsed_shell["surfaces"]
    scenarios = REPORT.parse_scenarios(driver_log)
    return parsed, scenarios


def scenario_rows(parsed, scenarios):
    rows = {}
    for scenario in scenarios:
        name = scenario["name"]
        if name == "warmup":
            continue
        label = scenario["label"]
        previous = REPORT.previous_label(scenarios, scenario)
        current = parsed["shell"].get(label, {})
        old = parsed["shell"].get(previous, {}) if previous else {}
        frame = REPORT.comp_entry(parsed["comp"], label, "frame timing")
        rows[name] = {
            "scenario": scenario,
            "label": label,
            "commit_p50": REPORT.as_int(current.get("commit_us_p50")),
            "commit_p95": REPORT.as_int(current.get("commit_us_p95")),
            "commit_p99": REPORT.as_int(current.get("commit_us_p99")),
            "frame_p50": REPORT.as_int(frame.get("p50_us")),
            "frame_p95": REPORT.as_int(frame.get("p95_us")),
            "frame_p99": REPORT.as_int(frame.get("p99_us")),
            "transport": REPORT.transport(current, old),
            "frames": REPORT.frame_percentiles(parsed, label, previous),
        }
    return rows


def fmt_int(value):
    return str(value)


def fmt_delta(current, baseline):
    if not baseline:
        return "n/a"
    return f"{current - baseline:+d}"


def fmt_speedup(baseline, current):
    if not current:
        return "n/a"
    return f"{baseline / current:.1f}x"


def fmt_rate(rate):
    return "n/a" if rate is None else f"{rate * 100:.1f}%"


def baseline_transport(row):
    commits = row.get("commits", 0)
    return {
        "path": "offscreen-qpa+shm",
        "commits": commits,
        "dmabuf": 0,
        "shm": commits,
        "shm_rate": 1.0 if commits else None,
        "copy_bytes": "n/a",
        "fallbacks": 0,
        "shm_allocs": row.get("shm_allocs", 0),
        "shm_bytes": row.get("shm_bytes", 0),
    }


def write_native(add, native_path):
    add("## Native rows")
    add("")
    if not native_path or not os.path.exists(native_path):
        add("No native row artifact was supplied.")
        add("")
        return
    add(
        "The native matrix is an explicit OPEN artifact, not a nested "
        f"substitution: `{native_path}`. The live session owns DRM master and "
        "no free logind seat/dedicated development user is available, so every "
        "native scenario, mode, mixed-output, and tier row is OPEN with its "
        "blocker. The live `presentation stats` observation below is not a "
        "controlled benchmark row."
    )
    add("")
    table_lines = []
    with open(native_path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if line.startswith("| row |") or line.startswith("|---"):
                table_lines.append(line.rstrip())
            elif line.startswith("| ") and table_lines:
                table_lines.append(line.rstrip())
    if table_lines:
        add("\n".join(table_lines))
        add("")
    observation = []
    with open(native_path, encoding="utf-8", errors="replace") as handle:
        in_block = False
        for line in handle:
            if line.startswith("Live-session native observation"):
                in_block = True
                continue
            if in_block and line.startswith("```text"):
                continue
            if in_block and line.startswith("```"):
                break
            if in_block:
                observation.append(line.rstrip())
    if any(observation):
        add("Live native observation (cumulative, not a scenario row):")
        add("")
        add("```text")
        add("\n".join(observation).strip())
        add("```")
        add("")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", default="docs/captures/t21-chrome-baseline.txt")
    parser.add_argument("--raw-root", default="docs/captures/t21-presentation-bench-raw")
    parser.add_argument("--main-name", default="main-144-full")
    parser.add_argument("--native-open", default="docs/captures/t21-presentation-bench.open.txt")
    parser.add_argument("--out", default="docs/captures/t21-presentation-bench-compare.md")
    parser.add_argument("--command", default="make t21-presentation-bench")
    args = parser.parse_args()

    baseline = parse_baseline(args.baseline)
    parsed, scenarios = parse_current(args.raw_root, args.main_name)
    current = scenario_rows(parsed, scenarios)

    shared = [
        name
        for name in (
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
        )
        if name in baseline and name in current
    ]
    if not shared:
        raise SystemExit("t21-presentation-compare: no shared scenarios")

    lines = []
    add = lines.append
    add("# T-21.4b baseline comparison — T183 vs v0.1.1")
    add("")
    add(
        "Generated by `scripts/t21-presentation-compare.py` from the committed "
        "T183 report and the v0.1.1 nested benchmark raw logs. This is a "
        "**nested A/B**: both sides ran on the same host as a Wayland client of "
        "a nested Dragonfruit compositor. It is not native presentation proof; "
        "the native rows are the explicit OPEN artifact linked below."
    )
    add("")
    add(f"- command: `{args.command}`")
    add(f"- baseline: `{args.baseline}`")
    add(f"- v0.1.1 raw: `{args.raw_root}/{args.main_name}/`")
    add(f"- native OPEN rows: `{args.native_open}`")
    add("")
    add("## Scenario windows")
    add("")
    add(
        "The scenario names are the same, but the T183 run used shorter "
        "windows (idle 5.3 s, Dock sweep 2.7 s, Mission Control 2.6 s). The "
        "v0.1.1 run uses the T192b set (idle 60 s, Dock sweep 10 s, Mission "
        "Control 6.5 s). Compare the per-frame percentiles, not the frame "
        "counts; the counts are not normalized for elapsed time."
    )
    add("")
    add("| scenario | T183 elapsed s | v0.1.1 elapsed s | T183 commits | v0.1.1 commits |")
    add("|---|---|---|---|---|")
    for name in shared:
        base = baseline[name]
        cur = current[name]
        add(
            f"| {name} | {base['elapsed']:.3f} | {cur['scenario']['elapsed']:.3f} | "
            f"{base.get('commits', 0)} | {cur['transport']['commits']} |"
        )
    add("")

    add("## Shell commit cost")
    add("")
    add(
        "`commit_us` is the shell's interval-windowed ring on both sides. A "
        "negative delta is an improvement. The T183 path re-rendered the "
        "offscreen QML scene, copied the image, and allocated a fresh "
        "`wl_shm` buffer per commit; the v0.1.1 path uses the render-control "
        "GPU rail with pooled buffers and frame callbacks."
    )
    add("")
    add(
        "| scenario | T183 p50 | p95 | p99 | v0.1.1 p50 | p95 | p99 | "
        "delta p50 | delta p95 | delta p99 | p50 speedup |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|")
    for name in shared:
        base = baseline[name]
        cur = current[name]
        add(
            f"| {name} | {base.get('commit_p50', 0)} | {base.get('commit_p95', 0)} | "
            f"{base.get('commit_p99', 0)} | {cur['commit_p50']} | "
            f"{cur['commit_p95']} | {cur['commit_p99']} | "
            f"{fmt_delta(cur['commit_p50'], base.get('commit_p50', 0))} | "
            f"{fmt_delta(cur['commit_p95'], base.get('commit_p95', 0))} | "
            f"{fmt_delta(cur['commit_p99'], base.get('commit_p99', 0))} | "
            f"{fmt_speedup(base.get('commit_p50', 0), cur['commit_p50'])} |"
        )
    add("")

    add("## Compositor frame time (windowed)")
    add("")
    add(
        "Both columns are the compositor's windowed `frame timing` line at the "
        "scenario snapshot. T183 has no `frame-trace`, so the v0.1.1 "
        "per-scenario frame-trace delta (in the main report) is not used for "
        "this comparison. The T183 window is cumulative and can include the "
        "warm-up; read the deltas as directional, not as a per-frame proof."
    )
    add("")
    add(
        "| scenario | T183 p50 | p95 | p99 | v0.1.1 p50 | p95 | p99 | "
        "delta p50 | delta p95 | delta p99 |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|")
    for name in shared:
        base = baseline[name]
        cur = current[name]
        add(
            f"| {name} | {base.get('frame_p50', 0)} | {base.get('frame_p95', 0)} | "
            f"{base.get('frame_p99', 0)} | {cur['frame_p50']} | "
            f"{cur['frame_p95']} | {cur['frame_p99']} | "
            f"{fmt_delta(cur['frame_p50'], base.get('frame_p50', 0))} | "
            f"{fmt_delta(cur['frame_p95'], base.get('frame_p95', 0))} | "
            f"{fmt_delta(cur['frame_p99'], base.get('frame_p99', 0))} |"
        )
    add("")

    add("## Transport and fallback rate")
    add("")
    add(
        "The T183 baseline committed every chrome frame through a fresh SHM "
        "buffer (`path=offscreen-qpa+shm`, 100% SHM). The v0.1.1 rows are "
        "dmabuf commits with the SHM fallback counted separately. `shm rate` "
        "is `shm / commits`; `dmabuf fallbacks` is a failed-export-attempt "
        "count, so a transient retry can raise it while the counted commit is "
        "still dmabuf. Fallback frames are never averaged into the dmabuf row."
    )
    add("")
    add(
        "| scenario | T183 path | T183 commits | T183 shm rate | "
        "v0.1.1 path | v0.1.1 commits | dmabuf | shm | shm rate | "
        "dmabuf fallbacks | copy bytes |"
    )
    add("|---|---|---|---|---|---|---|---|---|---|---|")
    total_base_commits = 0
    total_cur_commits = 0
    total_cur_dmabuf = 0
    total_cur_shm = 0
    for name in shared:
        base = baseline_transport(baseline[name])
        cur = current[name]["transport"]
        total_base_commits += base["commits"]
        total_cur_commits += cur["commits"]
        total_cur_dmabuf += cur["dmabuf"]
        total_cur_shm += cur["shm"]
        add(
            f"| {name} | {base['path']} | {base['commits']} | {fmt_rate(base['shm_rate'])} | "
            f"{cur['path']} | {cur['commits']} | {cur['dmabuf']} | {cur['shm']} | "
            f"{fmt_rate(cur['shm_rate'])} | {cur['fallbacks']} | {cur['copy_bytes']} |"
        )
    add("")
    add("### Shared-scenario transport total")
    add("")
    add("| side | commits | dmabuf | shm | shm rate |")
    add("|---|---|---|---|---|")
    add(
        f"| T183 baseline | {total_base_commits} | 0 | {total_base_commits} | "
        f"{fmt_rate(1.0 if total_base_commits else None)} |"
    )
    add(
        f"| v0.1.1 | {total_cur_commits} | {total_cur_dmabuf} | {total_cur_shm} | "
        f"{fmt_rate(total_cur_shm / total_cur_commits if total_cur_commits else None)} |"
    )
    add("")
    add(
        "The only v0.1.1 SHM commits in the shared set are the Mission Control "
        "overview surface (2 open + 2 close commits, 18 432 000 copy bytes "
        "each window). The Dock, menu bar, and popover rows are all dmabuf with "
        "zero copy bytes. This is why the aggregate Mission Control p99 can be "
        "worse than T183 even though the dmabuf surfaces are much faster: the "
        "16 ms overview fallback frame is a counted fallback, not a dmabuf "
        "frame. The T192c full-rail verification run (overview surface"
        " included) no longer crashes at teardown and reports zero shm"
        " commits; this measured comparison run keeps the overview on the"
        " deliberate fallback row so the fallback rate is visible."
    )
    add("")

    add("## Idle evidence")
    add("")
    add(
        "T183's committed idle window is 5.3 s and produced zero shell scene "
        "frames, zero commits, zero compositor frames, and zero client "
        "wakeups. The v0.1.1 settled idle window is 60 s and is reported in "
        "the main report; it is near-zero (the menu-bar clock crosses a minute "
        "boundary) and has zero client wakeups and zero animation frames. The "
        "longer v0.1.1 window is the stronger no-busy-loop check."
    )
    add("")

    add("## Rows with no T183 counterpart")
    add("")
    add(
        "T183 predates the per-output refresh clock (T187/T188) and the "
        "material-tier ladder (T189/T190), so the 60/120/144/240 Hz matrix and "
        "the Full/Reduced/Minimal rows have no baseline delta. They are in the "
        "v0.1.1 main report: `docs/captures/t21-presentation-bench.md`."
    )
    add("")

    write_native(add, args.native_open)

    add("## Verdict")
    add("")
    dock_base = baseline.get("dock-sweep", {})
    dock_cur = current.get("dock-sweep", {})
    add(
        "The shared-scenario comparison shows the v0.1.1 shell commit path is "
        "materially cheaper than T183: the Dock sweep p50 commit cost is "
        f"{dock_base.get('commit_p50', 0)} us on T183 vs "
        f"{dock_cur.get('commit_p50', 0)} us on v0.1.1 "
        f"({fmt_speedup(dock_base.get('commit_p50', 0), dock_cur.get('commit_p50', 0))} "
        "lower), and the transport moved from 100% fresh SHM commits to dmabuf "
        "with zero copy bytes on every measured Dock/menu/popover surface."
    )
    add("")
    add(
        "Mission Control is the one shared row with a worse aggregate p99. "
        "Its dmabuf surfaces still improve, but the overview surface is the "
        "counted SHM fallback and its ~16 ms commit dominates the aggregate "
        "p99. That is a real regression to file for T193, not an averaging "
        "artifact; the T192b full-rail teardown crash itself is fixed."
    )
    add("")
    add(
        "Native DRM rows remain OPEN. The live session's `presentation stats` "
        "is recorded in the native artifact, but it is a cumulative live "
        "observation, not a controlled Dock-sweep or mode row. A free DRM seat "
        "and the dedicated `dfdev` user are required to close the native rows."
    )
    add("")

    os.makedirs(os.path.dirname(args.out) or ".", exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines).rstrip() + "\n")
    print(f"t21-presentation-compare: wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
