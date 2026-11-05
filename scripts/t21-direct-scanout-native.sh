#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.3 native direct-scanout trace (T190d / T191a hardware rail).
#
# Starts the DRM compositor on a free logind seat with no shell chrome, maps a
# fullscreen client, and asserts the real `direct_scanouts` counter advances.
# The counter is only recorded from `DrmCompositor`'s primary-plane result; a
# host with no free seat writes an explicit OPEN artifact instead of faking it.
#
# Outputs:
#   docs/captures/t21-direct-scanout.txt       (free seat, PASS/FAIL stats)
#   docs/captures/t21-direct-scanout.open.txt  (no free seat)
#
# Overrides: OUT_TRACE, OUT_OPEN, SETTLE, COMPOSITOR, CLIENT.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT_TRACE="${OUT_TRACE:-docs/captures/t21-direct-scanout.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t21-direct-scanout.open.txt}"
SETTLE="${SETTLE:-8}"
COMPOSITOR="${COMPOSITOR:-target/debug/dragonfruit-compositor}"
[ -x "$COMPOSITOR" ] || {
    echo "t21-direct-scanout: $COMPOSITOR not built (run make cargo-build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "t21-direct-scanout: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-scanout.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

recorded="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
host="$(uname -srm)"
probe="$scratch/seat.json"
probe_rc=0
python3 scripts/drm-seat-probe.py >"$probe" || probe_rc=$?

seat_summary="$(python3 - "$probe" <<'PY'
import json, sys
report = json.load(open(sys.argv[1]))
if report["cards"]:
    for card in report["cards"]:
        state = "free" if card["can_set_master"] else (
            f"busy ({card['error'] or 'master held'})" if card["opened"] else
            f"unopenable ({card['error']})")
        print(f"  - {card['path']}: {state}")
else:
    print("  - no /dev/dri/card* node")
PY
)"

if [ "$probe_rc" -ne 0 ]; then
    reason="no free logind seat: no DRM card accepted master"
    [ "$probe_rc" -eq 2 ] && reason="no seat: no /dev/dri/card* node"
    mkdir -p "$(dirname "$OUT_OPEN")"
    {
        echo "T-21.3 — native direct-scanout trace: OPEN"
        echo "=============================================="
        echo
        echo "recorded: $recorded"
        echo "host:     $host"
        echo "result:   direct_scanouts>0 OPEN ($reason)"
        echo
        echo "DRM master probe:"
        echo "$seat_summary"
        echo
        echo "Why: the live graphical session owns DRM master, so a second"
        echo "     compositor cannot take the primary plane. This is the same"
        echo "     boundary as T-03.2/T-03.4. The nested T190d trace proves the"
        echo "     fullscreen policy; this artifact is the hardware half."
        echo
        echo "Reproduce on a free seat / VM:"
        echo "  make t21-direct-scanout-native"
        echo
        echo "The successful artifact is $OUT_TRACE."
    } >"$OUT_OPEN"
    echo "t21-direct-scanout: OPEN ($reason); wrote $OUT_OPEN"
    exit 0
fi

socket="dragonfruit-t21-scanout-$$"
log="$scratch/compositor.log"
client_log="$scratch/client.log"
echo "t21-direct-scanout: free DRM seat; starting the DRM compositor ($socket)"
setsid "$COMPOSITOR" --backend drm --socket-name "$socket" >"$log" 2>&1 &
pgid=$!
client_pid=""
cleanup() {
    [ -n "$client_pid" ] && kill -TERM "$client_pid" 2>/dev/null || true
    kill -TERM -"$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -"$pgid" 2>/dev/null || true
}
trap 'cleanup; rm -rf "$scratch"' EXIT

for _ in $(seq 1 300); do
    kill -0 "$pgid" 2>/dev/null || break
    [ -e "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket" ] && break
    sleep 0.1
done
if ! kill -0 "$pgid" 2>/dev/null; then
    marker="$(grep -m1 'DRM bring-up:' "$log" || true)"
    [ -n "$marker" ] || marker="DRM bring-up: OPEN (backend exited before reporting a marker)"
    mkdir -p "$(dirname "$OUT_OPEN")"
    {
        echo "T-21.3 — native direct-scanout trace: OPEN"
        echo "=============================================="
        echo
        echo "recorded: $recorded"
        echo "host:     $host"
        echo "result:   $marker"
        echo
        echo "DRM master probe:"
        echo "$seat_summary"
        echo
        echo "The backend did not reach a ready output. Full log:"
        tail -n 40 "$log"
    } >"$OUT_OPEN"
    echo "t21-direct-scanout: OPEN ($marker); wrote $OUT_OPEN"
    exit 0
fi

# A fullscreen client. ffplay's SDL Wayland backend asks for fullscreen; the
# vkcube fallback maps an output-sized window. No shell means no chrome, so
# the client can own the primary plane.
CLIENT="${CLIENT:-}"
if [ -z "$CLIENT" ]; then
    if command -v ffplay >/dev/null 2>&1; then
        CLIENT="ffplay"
    elif command -v vkcube >/dev/null 2>&1; then
        CLIENT="vkcube"
    fi
fi
case "$CLIENT" in
    ffplay)
        WAYLAND_DISPLAY="$socket" timeout 25 ffplay -fs -f lavfi \
            -i testsrc=size=1280x720:rate=30 -t 20 -autoexit -loglevel error \
            >"$client_log" 2>&1 &
        client_pid=$!
        ;;
    vkcube)
        WAYLAND_DISPLAY="$socket" timeout 25 vkcube --wsi wayland \
            --width 3840 --height 2160 --c 100000 --suppress_popups \
            >"$client_log" 2>&1 &
        client_pid=$!
        ;;
    *)
        echo "t21-direct-scanout: no ffplay/vkcube client available" >&2
        exit 1
        ;;
esac

sleep "$SETTLE"
kill -USR1 "$pgid" 2>/dev/null || true
sleep 1
scanout_line="$(grep -a 'scanout stats' "$log" | tail -1 || true)"
direct="$(printf '%s\n' "$scanout_line" | sed -n 's/.*direct_scanouts=\([0-9][0-9]*\).*/\1/p')"
direct="${direct:-0}"

mkdir -p "$(dirname "$OUT_TRACE")"
{
    echo "T-21.3 — native direct-scanout trace"
    echo "======================================"
    echo
    echo "recorded: $recorded"
    echo "host:     $host"
    echo "command:  $COMPOSITOR --backend drm --socket-name $socket"
    echo "client:   $CLIENT (fullscreen/output-sized, no shell chrome)"
    echo
    echo "DRM master probe:"
    echo "$seat_summary"
    echo
    echo "scanout stats:"
    echo "  $scanout_line"
    echo
    if [ "$direct" -gt 0 ]; then
        echo "result: PASS — direct_scanouts=$direct (real DRM primary plane)"
    else
        echo "result: FAIL — direct_scanouts=0 (the client was composited)"
    fi
    echo
    echo "Compositor log tail:"
    tail -n 40 "$log"
} >"$OUT_TRACE"

echo "t21-direct-scanout: direct_scanouts=$direct; wrote $OUT_TRACE"
[ "$direct" -gt 0 ] || exit 1
