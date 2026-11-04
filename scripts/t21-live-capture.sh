#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.1c live visual capture (T185a).
#
# Runs one nested demo with the shell's capture-only compositor seam
# (`DF_SCREENSHOT_CAPTURE_FIXTURE`) and writes one still:
#
#   motion   a Dock sweep still captured while the spring is moving
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
    motion|menu|popover|lock) ;;
    *) echo "usage: $0 motion|menu|popover|lock" >&2; exit 2 ;;
esac

SETTLE="${SETTLE:-2}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
SOCKET="${SOCKET:-dragonfruit-t21-live-$STATE-$$}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-live.XXXXXX")}"
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
    menu)
        DELAY=8000
        EXTRA_ENV=(DF_MENUBAR_MENU_FIXTURE=0)
        ;;
    popover)
        DELAY=16000
        EXTRA_ENV=(DF_DOCK_CHOOSER_FIXTURE=1)
        ;;
    lock)
        DELAY=6000
        EXTRA_ENV=(DF_LOCK_FIXTURE=1000)
        ;;
esac

echo "t21-live: $STATE capture on $SOCKET"
env "${EXTRA_ENV[@]}" \
    DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
    DF_SCREENSHOT_DIR="$SHOTS" \
    DF_SCREENSHOT_CAPTURE_FIXTURE="$DELAY:$CAPTURE" \
    DF_SHELL_LOG="$SHELL_LOG" \
    setsid make demo DEMO_ARGS="--socket-name $SOCKET --private-bus" \
    >"$LOG" 2>&1 &
DEMO_PGID=$!

for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
for _ in $(seq 1 2400); do
    if [ -f "$SHELL_LOG" ] && grep -q "menu bar configured" "$SHELL_LOG"; then
        break
    fi
    sleep 0.1
done
sleep "$SETTLE"

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
if state == "motion":
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
    SAVED="$(find "$SHOTS" -maxdepth 1 -name 'Screenshot_*.png' -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2-)"
    [ -n "$SAVED" ] && break
    sleep 0.5
done
if [ -z "$SAVED" ]; then
    echo "t21-live: the compositor capture did not appear" >&2
    tail -30 "$SHELL_LOG" >&2 || true
    exit 1
fi

cp "$SAVED" "docs/captures/t21-chrome-$STATE.png"
echo "t21-live: wrote docs/captures/t21-chrome-$STATE.png"
