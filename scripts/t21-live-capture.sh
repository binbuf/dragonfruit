#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1c live visual capture (T185a).
#
# Runs one nested demo with the shell's capture-only compositor seam
# (`DF_SCREENSHOT_CAPTURE_FIXTURE`) and writes one still:
#
#   motion   a Dock sweep still captured while the spring is moving
#   settled  a stationary Dock with no pointer motion (T190c damage evidence)
#   menu     the menu-bar dropdown open
#   popover  the Dock window chooser open
#   lock     the first-party lock surface with one typed character
#
# The capture is the compositor's own render (`df_toplevel_manager.capture_screenshot`),
# not a screencopy client. Requires a host Wayland session and the built tree.
# Not part of `make e2e`. Overrides: SETTLE (default 2), KEEP_SCRATCH.
set -euo pipefail

cd "$(dirname "$0")/.."

STATE="${1:-motion}"
case "$STATE" in
    motion|settled|menu|popover|lock|fullscreen|status|dock-context|desktop-context|files-context) ;;
    *) echo "usage: $0 motion|settled|menu|popover|lock|fullscreen|status|dock-context|desktop-context|files-context" >&2; exit 2 ;;
esac

SETTLE="${SETTLE:-2}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SOCKET="${SOCKET:-dragonfruit-t21-live-$STATE-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-live.XXXXXX")}"
# T-21.3: optional nested output scale and full-session rail. A scale other
# than 1 needs settingsd so the shell's display policy does not reset the
# compositor's scale; FULL_SESSION=1 also uses it for the 1x capture.
SCALE="${SCALE:-1}"
FULL_SESSION="${FULL_SESSION:-0}"
TRANSPARENCY="${TRANSPARENCY:-}"
MENU_INDEX="${MENU_INDEX:-0}"
STATUS_MENU="${STATUS_MENU:-wifi}"
OUT="${OUT:-docs/captures/t21-chrome-$STATE.png}"
FORCE_TIER="${FORCE_TIER:-}"
SYNTH="${XDG_RUNTIME_DIR:?XDG_RUNTIME_DIR must be set}/$SOCKET.synth"
SHOTS="$SCRATCH/shots"
LOG="$SCRATCH/demo.log"
SHELL_LOG="$SCRATCH/shell.log"
CAPTURE="$SCRATCH/capture.png"

mkdir -p "$SCRATCH" "$SHOTS"
rm -f "$SYNTH" "$CAPTURE"

cleanup() {
    for pid in ${EXTRA_PIDS:-}; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    if [ -n "${DEMO_PGID:-}" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
    fi
    rm -f "$SYNTH"
    [ "$KEEP_SCRATCH" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

case "$STATE" in
    motion)
        DELAY=12000
        EXTRA_ENV=()
        ;;
    settled)
        DELAY=10000
        EXTRA_ENV=()
        ;;
    menu)
        DELAY=8000
        EXTRA_ENV=(DF_MENUBAR_MENU_FIXTURE="$MENU_INDEX")
        ;;
    status)
        DELAY=8000
        EXTRA_ENV=(DF_STATUS_FIXTURE=1 DF_STATUS_MENU_FIXTURE="$STATUS_MENU")
        ;;
    dock-context)
        DELAY=8000
        EXTRA_ENV=(DF_STATUS_FIXTURE=1 DF_DOCK_MENU_FIXTURE=1)
        ;;
    desktop-context)
        DELAY=8000
        EXTRA_ENV=(DF_STATUS_FIXTURE=1 DF_DESKTOP_CONTEXT_FIXTURE=1)
        ;;
    files-context)
        DELAY=8000
        EXTRA_ENV=(DF_STATUS_FIXTURE=1 DF_DEMO_QT_APP=build/apps/files/dragonfruit-files DF_FILES_START_MENU=background)
        ;;
    popover)
        DELAY=16000
        EXTRA_ENV=(DF_DOCK_CHOOSER_FIXTURE=1)
        ;;
    lock)
        DELAY=6000
        EXTRA_ENV=(DF_LOCK_FIXTURE=1000)
        ;;
    fullscreen)
        # A settled fullscreen client with the persistent chrome hidden: the
        # direct-scanout policy trace (T190d). The driver below picks a window
        # and sends `fullscreen <id>`; the capture fires after the motion lands.
        DELAY=14000
        EXTRA_ENV=()
        ;;
esac
# A scaled session has to wait for settingsd -> shell -> df_output.set_scale
# and the following output-properties event before the capture's physical
# crop is correct. The scale change lands a few seconds into the session.
if [ "$SCALE" != "1" ]; then
    DELAY=$((DELAY + 6000))
fi

echo "t21-live: $STATE capture on $SOCKET (scale $SCALE)"
if [ "$STATE" = "files-context" ]; then
    # The Files-window context needs the demo's Qt client; `dev-full --shell`
    # launches only chrome/services. Keep the demo app path and private bus.
    DEMO_CMD=(make demo DEMO_ARGS="--socket-name $SOCKET --private-bus")
elif [ "$FULL_SESSION" = "1" ] || [ "$SCALE" != "1" ]; then
    # `make dev-full` owns settingsd on a private bus. Seed the display scale
    # before the shell starts: the shell forwards settingsd's display.scale as
    # `df_output.set_scale`, which would otherwise reset the compositor's
    # DRAGONFRUIT_NESTED_SCALE back to 1.0.
    SESSION_DIR="$XDG_RUNTIME_DIR/$SOCKET.session"
    mkdir -p "$SESSION_DIR/config/dragonfruit"
    WALLPAPER_SOURCE="${WALLPAPER_SOURCE:-}" \
    WALLPAPER_FIT="${WALLPAPER_FIT:-fill}" \
    COLOR_SCHEME="${COLOR_SCHEME:-}" \
    TRANSPARENCY="$TRANSPARENCY" \
    REDUCE_TRANSPARENCY="${REDUCE_TRANSPARENCY:-0}" \
    INCREASE_CONTRAST="${INCREASE_CONTRAST:-0}" \
    python3 - "$SESSION_DIR/config/dragonfruit/settings.json" "$SCALE" <<'PY'
import json
import os
import sys

path, scale = sys.argv[1], float(sys.argv[2])
keys = {"display.scale": scale}
source = os.environ.get("WALLPAPER_SOURCE", "")
if source:
    keys["wallpaper.source"] = source
    keys["wallpaper.fit"] = os.environ.get("WALLPAPER_FIT", "fill")
    keys["wallpaper.showOnAllSpaces"] = True
scheme = os.environ.get("COLOR_SCHEME", "")
if scheme in ("light", "dark"):
    keys["appearance.colorScheme"] = scheme
keys["accessibility.reduceTransparency"] = os.environ.get("REDUCE_TRANSPARENCY", "0") == "1"
keys["accessibility.increaseContrast"] = os.environ.get("INCREASE_CONTRAST", "0") == "1"
transparency = os.environ.get("TRANSPARENCY", "")
if transparency:
    keys["appearance.transparency"] = float(transparency)
with open(path, "w", encoding="utf-8") as handle:
    json.dump({"schema": 18, "keys": keys}, handle)
PY
    DEMO_CMD=(make dev-full DEV_FULL_ARGS="--socket-name $SOCKET")
else
    DEMO_CMD=(make demo DEMO_ARGS="--socket-name $SOCKET --private-bus")
fi
env "${EXTRA_ENV[@]}" \
    DRAGONFRUIT_NESTED_SCALE="$SCALE" \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    DF_SCREENSHOT_DIR="$SHOTS" \
    DF_SCREENSHOT_CAPTURE_FIXTURE="$DELAY:$CAPTURE" \
    DF_SHELL_LOG="$SHELL_LOG" \
    setsid "${DEMO_CMD[@]}" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
# T-21.3 evidence can pin the material tier: the nested host throttles
# submit() to roughly 1 Hz, so the degrade controller would otherwise reach
# Minimal and turn the sampled pass off before a long capture.
if [ -n "$FORCE_TIER" ]; then
    python3 - "$SYNTH" "$FORCE_TIER" <<'PY'
import socket
import sys

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto(f"set degrade-tier {sys.argv[2]}".encode(), sys.argv[1])
PY
    sleep 0.5
fi
for _ in $(seq 1 2400); do
    if [ -f "$SHELL_LOG" ] && grep -q "menu bar configured" "$SHELL_LOG"; then
        break
    fi
    sleep 0.1
done
sleep "$SETTLE"

# Optional read-only introspection for capture scorers: `query scale` gives
# the logical/physical panel rect and `query material` gives the resolved
# Dock preset. Both go to one file so a capture report can cite the live
# geometry without guessing it from pixels.
if [ -n "${QUERY_OUT:-}" ]; then
    python3 - "$SYNTH" "$QUERY_OUT" <<'PY'
import os
import socket
import sys

synth, out = sys.argv[1], sys.argv[2]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode/t21-query-{os.getpid()}.sock"
try:
    os.unlink(client)
except FileNotFoundError:
    pass
sock.bind(client)
sock.settimeout(2.0)
chunks = []
for command in (b"query scale", b"query material"):
    sock.sendto(command, synth)
    while True:
        try:
            data, _ = sock.recvfrom(65536)
        except socket.timeout:
            break
        text = data.decode()
        chunks.append(text)
        if "end\n" in text:
            break
with open(out, "w", encoding="utf-8") as handle:
    handle.write("".join(chunks))
PY
fi

# The window chooser only opens for an app with more than one window. The
# demo's default apps each have one, so add two X11 windows of the same class
# on the nested Xwayland display.
EXTRA_PIDS=""
if [ "$STATE" = "popover" ]; then
    X11_DISPLAY_FILE="$XDG_RUNTIME_DIR/$SOCKET.x11-display"
    for _ in $(seq 1 100); do [ -f "$X11_DISPLAY_FILE" ] && break; sleep 0.1; done
    if [ -f "$X11_DISPLAY_FILE" ]; then
        X11_DISPLAY="$(cat "$X11_DISPLAY_FILE")"
        DISPLAY="$X11_DISPLAY" xmessage -geometry 300x150+80+120 \
            -title "T21 popover A" "T21 popover A" >/dev/null 2>&1 &
        EXTRA_PIDS="$EXTRA_PIDS $!"
        DISPLAY="$X11_DISPLAY" xmessage -geometry 300x150+410+120 \
            -title "T21 popover B" "T21 popover B" >/dev/null 2>&1 &
        EXTRA_PIDS="$EXTRA_PIDS $!"
        sleep 3
    fi
fi

python3 - "$SYNTH" "$STATE" <<'PY'
import os, socket, sys, time
synth, state = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
client = f"/tmp/opencode/t21-live-{os.getpid()}.sock"
try: os.unlink(client)
except FileNotFoundError: pass
s.bind(client); s.settimeout(2)
def send(c): s.sendto(c.encode(), synth)
def motion(x, y): send(f"motion-abs {x / 1280:.6f} {y / 800:.6f}")
def key(code):
    send(f"key {code} down"); send(f"key {code} up"); time.sleep(0.15)
if state == "fullscreen":
    # Ask the compositor for a window and enter its dedicated fullscreen
    # Space. The persistent menu bar/Dock must disappear and the material
    # pass must stop; `query scanout` then reports the policy candidate.
    send("query decorations")
    end = time.time() + 3.0
    windows = []
    while time.time() < end:
        try:
            line = s.recv(4096).decode()
        except socket.timeout:
            break
        for row in line.splitlines():
            parts = row.split()
            if parts and parts[0] == "decoration" and len(parts) > 11:
                windows.append((int(parts[1]), parts[11]))
    if not windows:
        raise SystemExit("t21-live: no window to fullscreen")
    send(f"fullscreen {windows[0][0]}")
    time.sleep(3.0)
elif state == "desktop-context":
    # Empty desktop, clear of the demo windows and the Dock.
    motion(180, 900)
    send("button 0x111 down")
    send("button 0x111 up")
    time.sleep(1.0)
elif state == "motion":
    # Keep the Dock spring moving across the capture instant.
    for index in range(600):
        x = 486 + (806 - 486) * (0.5 + 0.5 * __import__("math").sin(index / 7))
        motion(x, 770)
        time.sleep(0.05)
elif state == "lock":
    # One character reaches the lock surface's password buffer; Return is
    # deliberately not pressed (PAM is out of scope).
    time.sleep(1.0)
    key(30)
    time.sleep(1.0)
PY

# The shell reads the compositor's temporary PNG and re-saves it into
# DF_SCREENSHOT_DIR, so the temporary path may already be gone by the time we
# look. Accept either artifact.
SAVED=""
for _ in $(seq 1 80); do
    # The compositor fixture writes `$CAPTURE` directly; the shell's copy into
    # `DF_SCREENSHOT_DIR` is a second path and can still be zero-length while
    # the compositor is flushing. Prefer the fixture artifact when it exists.
    if [ -s "$CAPTURE" ]; then
        SAVED="$CAPTURE"
        break
    fi
    SAVED="$(find "$SHOTS" -maxdepth 1 -name 'Screenshot_*.png' -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2-)"
    [ -n "$SAVED" ] && break
    sleep 0.5
done
if [ -z "$SAVED" ]; then
    echo "t21-live: the compositor capture did not appear" >&2
    tail -30 "$SHELL_LOG" >&2 || true
    exit 1
fi

# Optional T-21.3 evidence: dump the live compositor's material counters while
# the captured frame's chrome is still mapped. `DUMP_COMPOSITOR_STATS=1`.
if [ "${DUMP_COMPOSITOR_STATS:-0}" = "1" ]; then
    COMPOSITOR_PID="$(pgrep -f "^$PWD/target/debug/dragonfruit-compositor --backend nested --socket-name $SOCKET$" | head -1)"
    if [ -n "$COMPOSITOR_PID" ]; then
        : >"$SCRATCH/compositor-stats.txt"
        # Two snapshots one second apart: the second is the warm-up check for
        # scene/panel allocations while the capture frame's chrome is mapped.
        for _ in 1 2; do
            kill -USR1 "$COMPOSITOR_PID" 2>/dev/null || true
            sleep 1
            echo "snapshot" >>"$SCRATCH/compositor-stats.txt"
            grep -aE "material stats|scanout stats|output material stats" "$LOG" \
                | tail -n 6 | tee -a "$SCRATCH/compositor-stats.txt"
        done
        if [ -n "${STATS_OUT:-}" ]; then
            cp "$SCRATCH/compositor-stats.txt" "$STATS_OUT"
        fi
        if [ "${ASSERT_MATERIAL_FLAT:-0}" = "1" ]; then
            python3 - "$SCRATCH/compositor-stats.txt" <<'PY'
import re
import sys

groups = [group for group in open(sys.argv[1]).read().split("snapshot") if group.strip()]
if len(groups) < 2:
    raise SystemExit("t21-live: expected two material snapshots")


def field(group, key):
    match = re.search(rf"{key}=(\d+)", group)
    if not match:
        raise SystemExit(f"t21-live: missing {key} in snapshot {group!r}")
    return int(match.group(1))


before, after = groups[-2], groups[-1]
for key in (
    "material_allocations",
    "material_scene_allocations",
    "material_panel_allocations",
    "material_region_px",
):
    a, b = field(before, key), field(after, key)
    if a != b:
        raise SystemExit(f"t21-live: {key} not flat after warm-up: {a} -> {b}")
print("t21-live: material allocations and region are flat across the snapshots")
PY
        fi
    fi
fi

mkdir -p "$(dirname "$OUT")"
cp "$SAVED" "$OUT"
echo "t21-live: wrote $OUT"
