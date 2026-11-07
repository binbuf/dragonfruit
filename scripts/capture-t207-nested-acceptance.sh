#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T207 nested and fallback acceptance capture.
#
# Runs the checked-in case matrix (tests/glass/t207-cases.json) on the nested
# compositor. Every A/B case runs twice with the same scene, geometry, settings
# and output scale: once with DRAGONFRUIT_GLASS_ROLLOUT=legacy and once with
# =replacement. Single rows exercise overlays, software/SHM/dmabuf, forced
# material tiers, live output change, lock/fullscreen transitions and idle
# settle. The captured frame is the compositor's own render, not a screencopy
# client.
#
# Outputs:
#   docs/captures/t207-<id>.png / .txt / -stats.txt / t207-logs/<id>.log
#   docs/captures/t207-nested-acceptance-capture.json
#   docs/captures/t207-nested-acceptance.json / .md (via the scorer)
#
# Requires a host Wayland session (or kwin_wayland --virtual), the built tree,
# python3 with Pillow, and gdbus. Not part of `make e2e`.
#
# Overrides: ONLY=<comma-separated ids>, SETTLE, OUTDIR, KEEP_SCRATCH,
# FORCE_KWIN, SKIP_SCORE.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-3}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t207.XXXXXX")}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
KWIN_SOCKET="df-t207-kwin-$$"
KWIN_PID=""
HOST_DISPLAY="${WAYLAND_DISPLAY:-}"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-}"
FORCE_KWIN="${FORCE_KWIN:-0}"
SKIP_SCORE="${SKIP_SCORE:-0}"
LOG_DIR="$OUTDIR/t207-logs"
JSONL="$SCRATCH/capture.jsonl"
NEUTRAL="$SCRATCH/neutral.png"
CHECKER="$SCRATCH/checker.png"
MOVING_PANEL="$SCRATCH/t207-moving-panel"
mkdir -p "$OUTDIR" "$LOG_DIR" "$SCRATCH"
: >"$JSONL"

cleanup() {
    if [ -n "$KWIN_PID" ]; then
        kill -TERM "$KWIN_PID" 2>/dev/null || true
        sleep 1
        kill -KILL "$KWIN_PID" 2>/dev/null || true
    fi
    rm -f "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET" \
          "${XDG_RUNTIME_DIR:-/tmp}/$KWIN_SOCKET.lock" 2>/dev/null || true
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

if [ -z "$HOST_RUNTIME" ]; then
    echo "capture-t207: XDG_RUNTIME_DIR is unset — writing OPEN" >&2
    cat >"$OUTDIR/t207-nested-acceptance.open.txt" <<EOF
T207 nested acceptance — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: XDG_RUNTIME_DIR is not set; no nested compositor can run.
rerun: XDG_RUNTIME_DIR=<runtime> WAYLAND_DISPLAY=<host> make nested-acceptance-capture
EOF
    exit 0
fi

if [ "$FORCE_KWIN" = "1" ] || [ -z "$HOST_DISPLAY" ] \
        || ! [ -S "$HOST_RUNTIME/$HOST_DISPLAY" ]; then
    if command -v kwin_wayland >/dev/null 2>&1; then
        echo "capture-t207: starting kwin_wayland --virtual (socket $KWIN_SOCKET)"
        XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY= \
            kwin_wayland --virtual --fullscreen 1 --socket "$KWIN_SOCKET" \
            --width 1920 --height 1200 --scale 1 --no-lockscreen \
            --no-global-shortcuts --no-kactivities \
            >"$SCRATCH/kwin.log" 2>&1 &
        KWIN_PID=$!
        for _ in $(seq 1 100); do
            [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ] && break
            sleep 0.1
        done
        if ! [ -S "$HOST_RUNTIME/$KWIN_SOCKET" ]; then
            echo "capture-t207: kwin_wayland did not create its socket" >&2
            tail -20 "$SCRATCH/kwin.log" >&2 || true
            exit 1
        fi
        HOST_DISPLAY="$KWIN_SOCKET"
    else
        echo "capture-t207: no host Wayland session and no kwin_wayland — writing OPEN" >&2
        cat >"$OUTDIR/t207-nested-acceptance.open.txt" <<EOF
T207 nested acceptance — live capture
timestamp: $(date -Is)
verdict: OPEN
reason: no usable host Wayland session and kwin_wayland is not installed.
rerun: WAYLAND_DISPLAY=<host-socket> make nested-acceptance-capture
EOF
        exit 0
    fi
fi

[ -x target/debug/dragonfruit-compositor ] || {
    echo "capture-t207: target/debug/dragonfruit-compositor missing; run make build" >&2
    exit 1
}
[ -x build/shell/src/dragonfruit-shell ] || {
    echo "capture-t207: build/shell/src/dragonfruit-shell missing; run make build" >&2
    exit 1
}
command -v gdbus >/dev/null 2>&1 || {
    echo "capture-t207: gdbus not found" >&2
    exit 1
}

python3 scripts/t207-nested-acceptance.py plan >"$SCRATCH/plan.txt"

python3 - "$NEUTRAL" "$CHECKER" <<'PY'
import sys
from PIL import Image, ImageDraw

neutral = Image.new("RGB", (1920, 1200), (128, 128, 128))
neutral.save(sys.argv[1])
checker = Image.new("RGB", (1920, 1200), (128, 128, 128))
draw = ImageDraw.Draw(checker)
for x in range(0, 1920, 16):
    draw.line((x, 0, x, 1200), fill=(238, 238, 238), width=1)
checker.save(sys.argv[2])
PY

cc -O2 -o "$MOVING_PANEL" scripts/t207-moving-panel.c -lX11
export T207_MOVING_PANEL="$MOVING_PANEL"

record_case() {
    local id="$1" rollout="$2" state="$3" surface="$4" scheme="$5" \
          scale="$6" tier="$7" material_path="$8" chrome_gpu="$9" \
          wallpaper="${10}" transparency="${11}" reduce="${12}" \
          contrast="${13}" motion="${14}" capture_delay="${15}" \
          stats_interval="${16}" stats_snapshots="${17}" assert_idle="${18}" \
          reference="${19}" expect="${20}" session="${21}" command="${22}"
    local capture="$OUTDIR/t207-${id}-${rollout}.png"
    local query="$OUTDIR/t207-${id}-${rollout}.txt"
    local stats="$OUTDIR/t207-${id}-${rollout}-stats.txt"
    local log="$LOG_DIR/${id}-${rollout}.log"
    local present=false
    [ -s "$capture" ] && present=true
    ID="$id" ROLLOUT="$rollout" STATE="$state" SURFACE="$surface" \
    SCHEME="$scheme" SCALE="$scale" TIER="$tier" MATERIAL_PATH="$material_path" \
    CHROME_GPU="$chrome_gpu" WALLPAPER="$wallpaper" TRANSPARENCY="$transparency" \
    REDUCE="$reduce" CONTRAST="$contrast" MOTION="$motion" \
    CAPTURE_DELAY="$capture_delay" STATS_INTERVAL="$stats_interval" \
    STATS_SNAPSHOTS="$stats_snapshots" ASSERT_IDLE="$assert_idle" \
    REFERENCE="$reference" EXPECT="$expect" SESSION="$session" \
    COMMAND="$command" PRESENT="$present" \
    CAPTURE_PATH="$capture" QUERY_PATH="$query" STATS_PATH="$stats" LOG_PATH="$log" \
    python3 - "$JSONL" <<'PY'
import json
import os
import sys

keys = (
    "ID", "ROLLOUT", "STATE", "SURFACE", "SCHEME", "SCALE", "TIER",
    "MATERIAL_PATH", "CHROME_GPU", "WALLPAPER", "TRANSPARENCY", "REDUCE",
    "CONTRAST", "MOTION", "CAPTURE_DELAY", "STATS_INTERVAL",
    "STATS_SNAPSHOTS", "ASSERT_IDLE", "REFERENCE", "EXPECT", "SESSION",
    "COMMAND", "PRESENT", "CAPTURE_PATH", "QUERY_PATH", "STATS_PATH",
    "LOG_PATH",
)
entry = {key.lower(): os.environ.get(key, "") for key in keys}
entry["present"] = entry["present"] == "true"
with open(sys.argv[1], "a", encoding="utf-8") as handle:
    handle.write(json.dumps(entry) + "\n")
PY
}

run_case() {
    local id="$1" rollout="$2" state="$3" surface="$4" scheme="$5" \
          scale="$6" tier="$7" material_path="$8" chrome_gpu="$9" \
          wallpaper="${10}" transparency="${11}" reduce="${12}" \
          contrast="${13}" motion="${14}" capture_delay="${15}" \
          stats_interval="${16}" stats_snapshots="${17}" assert_idle="${18}" \
          reference="${19}" expect="${20}" session="${21}" pre_capture="${22}"
    if [ -n "${ONLY:-}" ] && [[ ",$ONLY," != *",$id,"* ]]; then
        return 0
    fi
    local socket="df-t207-${id}-${rollout}-$$"
    local out="$OUTDIR/t207-${id}-${rollout}.png"
    local query="$OUTDIR/t207-${id}-${rollout}.txt"
    local stats="$OUTDIR/t207-${id}-${rollout}-stats.txt"
    local log="$LOG_DIR/${id}-${rollout}.log"
    local case_scratch="$SCRATCH/${id}-${rollout}"
    local source="$NEUTRAL"
    [ "$wallpaper" = "checker" ] && source="$CHECKER"
    local full=1 app=0
    case "$session" in
        app) full=0; app=1 ;;
        demo) full=0; app=0 ;;
    esac
    local chrome_env=()
    if [ "$chrome_gpu" = "1" ]; then
        chrome_env=(DRAGONFRUIT_CHROME_GPU=1
                    DRAGONFRUIT_CHROME_GPU_SURFACES=dock,menubar,menubar-popup,dock-popup)
    else
        chrome_env=(DRAGONFRUIT_CHROME_GPU=0)
    fi
    local pre=""
    case "$pre_capture" in
        moving-panel)
            pre='for i in $(seq 1 120); do [ -f "$XDG_RUNTIME_DIR/$SOCKET.x11-display" ] && break; sleep 0.1; done; DISPLAY=$(cat "$XDG_RUNTIME_DIR/$SOCKET.x11-display") "$T207_MOVING_PANEL" 25 & echo $! > "$SCRATCH/moving-panel.pid"'
            ;;
        scale-change)
            pre='BUS=$(sed -n "s/.*private session bus at //p" "$SCRATCH/demo.log" | head -1); echo "bus=$BUS" > "$SCRATCH/scale-change.log"; DBUS_SESSION_BUS_ADDRESS="$BUS" gdbus call --session --dest org.dragonfruit.Settings1 --object-path /org/dragonfruit/Settings1 --method org.dragonfruit.Settings1.Set display.scale "<1.5>" >> "$SCRATCH/scale-change.log" 2>&1; sleep 6'
            ;;

    esac
    echo "capture-t207: $id/$rollout ($state, $scheme, scale $scale, $tier, $material_path)"
    mkdir -p "$case_scratch"
    env XDG_RUNTIME_DIR="$HOST_RUNTIME" WAYLAND_DISPLAY="$HOST_DISPLAY" \
        DRAGONFRUIT_NESTED_SIZE="1920x1200" \
        DRAGONFRUIT_GLASS_ROLLOUT="$rollout" \
        DRAGONFRUIT_CHROME_STATS=1 \
        "${chrome_env[@]}" \
        FULL_SESSION="$full" APP_SESSION="$app" SCALE="$scale" SOCKET="$socket" \
        WALLPAPER_SOURCE="$source" WALLPAPER_FIT=fill \
        COLOR_SCHEME="$scheme" TRANSPARENCY="$transparency" \
        REDUCE_TRANSPARENCY="$reduce" INCREASE_CONTRAST="$contrast" \
        REDUCE_MOTION="$motion" \
        FORCE_TIER="$tier" FORCE_MATERIAL_PATH="$material_path" \
        CAPTURE_DELAY="$capture_delay" \
        QUERY_OUT="$query" SETTLE="$SETTLE" OUT="$out" \
        DUMP_COMPOSITOR_STATS=1 STATS_OUT="$stats" \
        STATS_INTERVAL="$stats_interval" STATS_SNAPSHOTS="$stats_snapshots" \
        ASSERT_IDLE_FRAMES="$assert_idle" \
        PRE_CAPTURE_CMD="$pre" \
        SCRATCH="$case_scratch" KEEP_SCRATCH=1 \
        bash scripts/t21-live-capture.sh "$state" >"$case_scratch/run.log" 2>&1 || true
    if [ -f "$case_scratch/moving-panel.pid" ]; then
        kill -TERM "$(cat "$case_scratch/moving-panel.pid")" 2>/dev/null || true
    fi
    cp "$case_scratch/demo.log" "$log" 2>/dev/null || true
    if [ -f "$case_scratch/shell.log" ]; then
        cp "$case_scratch/shell.log" "$LOG_DIR/${id}-${rollout}-shell.log" 2>/dev/null || true
    fi
    if [ -f "$case_scratch/scale-change.log" ]; then
        cp "$case_scratch/scale-change.log" \
            "$LOG_DIR/${id}-${rollout}-scale-change.log" 2>/dev/null || true
    fi
    rm -rf "$HOST_RUNTIME/$socket.session" "$HOST_RUNTIME/$socket" \
           "$HOST_RUNTIME/$socket.lock" "$HOST_RUNTIME/$socket.launch-token" \
           "$HOST_RUNTIME/$socket.desktop-launch-token" 2>/dev/null || true
    if [ ! -s "$out" ]; then
        echo "capture-t207: $id/$rollout produced no capture" >&2
        tail -40 "$case_scratch/run.log" >&2 || true
    fi
    record_case "$id" "$rollout" "$state" "$surface" "$scheme" "$scale" \
        "$tier" "$material_path" "$chrome_gpu" "$wallpaper" "$transparency" \
        "$reduce" "$contrast" "$motion" "$capture_delay" "$stats_interval" \
        "$stats_snapshots" "$assert_idle" "$reference" "$expect" "$session" \
        "make nested-acceptance-capture ONLY=$id"
}

while IFS=$'\x1f' read -r id rollout state surface scheme scale tier \
        material_path chrome_gpu wallpaper transparency reduce contrast \
        motion capture_delay stats_interval stats_snapshots assert_idle \
        reference expect session pre_capture; do
    [ -n "$id" ] || continue
    run_case "$id" "$rollout" "$state" "$surface" "$scheme" "$scale" "$tier" \
        "$material_path" "$chrome_gpu" "$wallpaper" "$transparency" "$reduce" \
        "$contrast" "$motion" "$capture_delay" "$stats_interval" \
        "$stats_snapshots" "$assert_idle" "$reference" "$expect" "$session" \
        "$pre_capture"
done <"$SCRATCH/plan.txt"

python3 - "$JSONL" "$OUTDIR/t207-nested-acceptance-capture.json" \
    "$HOST_DISPLAY" "$HOST_RUNTIME" <<'PY'
import json
import os
import platform
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

jsonl, out, display, runtime = sys.argv[1:]
rows = []
with open(jsonl, encoding="utf-8") as handle:
    for line in handle:
        line = line.strip()
        if line:
            rows.append(json.loads(line))
host = {
    "hostname": platform.node(),
    "kernel": platform.platform(),
    "wayland_display": display,
    "xdg_runtime_dir": runtime,
}
try:
    host["session"] = os.environ.get("XDG_SESSION_TYPE", "")
    host["desktop"] = os.environ.get("XDG_CURRENT_DESKTOP", "")
except Exception:
    pass
report = {
    "schema": 1,
    "task": "T207",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "generated_by": "scripts/capture-t207-nested-acceptance.sh",
    "host": host,
    "environment": {
        "backend": "nested",
        "output": "follows the host window; the per-case query records the live geometry",
        "scale": "per case",
        "refresh_hz": "per case (nested override)",
        "rollouts": ["legacy", "replacement"],
        "material_paths": ["gpu", "software"],
        "transports": ["shm", "dmabuf"],
    },
    "cases": rows,
}
Path(out).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(f"capture-t207: wrote {out} ({len(rows)} rows)")
PY

if [ "$SKIP_SCORE" != "1" ]; then
    python3 scripts/t207-nested-acceptance.py score
fi

echo "capture-t207: done"
