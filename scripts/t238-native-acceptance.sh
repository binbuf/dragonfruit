#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T238 controlled native acceptance.
#
# On a host with a free DRM seat this compares span decorations (`split`)
# against `split-deco` (shadow + titlebar shaders) at 4K/144 Hz for 2, 10, and
# 30 windows, with real vblank presentation intervals, input latency, frame
# p95/p99, CPU/GPU timings, and RSS. A verdict requires evidence for every
# trial. The systemd app-isolation check has its own reported verdict.
#
# With no free seat it writes an explicit OPEN artifact (the same boundary as
# `make drm-bringup` / `make t21-direct-scanout-native`) and exits 0. The
# artifact records the exact commands to run on a released seat. Once
# trials run, an over-budget result exits 1 and incomplete evidence exits 2.
#
# Outputs:
#   docs/captures/t238-native-acceptance.txt       (free seat)
#   docs/captures/t238-native-acceptance.open.txt  (no free seat)
#
# Overrides: OUT_TRACE, OUT_OPEN, OUT_DIR, DURATION, TRIALS, SETTLE,
#            COMPOSITOR_BIN.
set -euo pipefail

cd "$(dirname "$0")/.."

OUT_TRACE="${OUT_TRACE:-docs/captures/t238-native-acceptance.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t238-native-acceptance.open.txt}"
OUT_DIR="${OUT_DIR:-/tmp/t238-native-acceptance}"
DURATION="${DURATION:-60}"
TRIALS="${TRIALS:-2}"
SETTLE="${SETTLE:-8}"
COMPOSITOR="${COMPOSITOR_BIN:-target/release/dragonfruit-compositor}"

[ -x "$COMPOSITOR" ] || {
    echo "t238-native-acceptance: $COMPOSITOR not built (run make build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "t238-native-acceptance: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t238-native.XXXXXX")"
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

# The systemd check does not need a DRM seat; run it in both paths so the
# OPEN artifact still records the live app-isolation result.
systemd_ok=0
systemd_log="$scratch/systemd.txt"
systemd_check() {
    command -v systemd-run >/dev/null 2>&1 || return 0
    systemctl --user show-environment >/dev/null 2>&1 || return 0
    local unit1 unit2 active1 active2 stopped1 stopped2
    unit1="dragonfruit-accept-$(python3 -c 'import uuid; print(uuid.uuid4().hex[:8])')"
    unit2="dragonfruit-accept-$(python3 -c 'import uuid; print(uuid.uuid4().hex[:8])')"
    systemd-run --user --collect --unit="$unit1" -- sleep 60 >/dev/null 2>&1 || return 0
    systemd-run --user --collect --unit="$unit2" -- sleep 60 >/dev/null 2>&1 || {
        systemctl --user stop "$unit1" >/dev/null 2>&1 || true
        return 0
    }
    active1="$(systemctl --user is-active "$unit1" || true)"
    active2="$(systemctl --user is-active "$unit2" || true)"
    systemctl --user stop "$unit1" "$unit2" >/dev/null 2>&1 || true
    stopped1="$(systemctl --user is-active "$unit1" || true)"
    stopped2="$(systemctl --user is-active "$unit2" || true)"
    {
        echo "unit1=$unit1 active=$active1 stopped=$stopped1"
        echo "unit2=$unit2 active=$active2 stopped=$stopped2"
    } >"$systemd_log"
    [ "$active1" = "active" ] && [ "$active2" = "active" ] \
        && [ "$stopped1" != "active" ] && [ "$stopped2" != "active" ] \
        && systemd_ok=1
}
systemd_check

if [ "$probe_rc" -ne 0 ]; then
    reason="no free logind seat: no DRM card accepted master"
    [ "$probe_rc" -eq 2 ] && reason="no seat: no /dev/dri/card* node"
    mkdir -p "$(dirname "$OUT_OPEN")"
    {
        echo "T238 — controlled native acceptance: OPEN"
        echo "=========================================="
        echo
        echo "recorded: $recorded"
        echo "host:     $host"
        echo "result:   $reason"
        echo
        echo "DRM master probe:"
        echo "$seat_summary"
        echo
        echo "systemd multi-instance: $([ "$systemd_ok" = "1" ] && echo PASS || echo OPEN)"
        cat "$systemd_log" 2>/dev/null || true
        echo
        echo "Checklist on a safely released DRM seat (4K/144 Hz):"
        echo "  1. make build"
        echo "  2. make t238-native-acceptance"
        echo
        echo "The script runs, per window count (2, 10, 30) and arm"
        echo "(split, split-deco):"
        echo "  BACKEND=drm WINDOWS=<n> ARMS='split split-deco' \\"
        echo "    WORKLOADS='scroll drag' DURATION=$DURATION TRIALS=$TRIALS \\"
        echo "    PIN_TIER=full ./scripts/t238-material-workloads.sh"
        echo
        echo "It then checks two transient user services:"
        echo "  systemd-run --user --collect --unit=dragonfruit-accept-<uuid> -- sleep 60"
        echo "  systemctl --user is-active/stop <unit>"
        echo
        echo "Acceptance requires:"
        echo "  * real vblank presentation p95/p99 (not swap completion)"
        echo "  * frame p95/p99 under the 6.944 ms 144 Hz budget"
        echo "  * input latency samples with no dropped input"
        echo "Compare per-arm timings and RSS across trials in the workload reports."
        echo
        echo "The successful artifact is $OUT_TRACE."
    } >"$OUT_OPEN"
    echo "t238-native-acceptance: OPEN ($reason)"
    echo "t238-native-acceptance: wrote $OUT_OPEN"
    exit 0
fi

mkdir -p "$OUT_DIR"
run_dir="$(mktemp -d "$OUT_DIR/run.XXXXXX")"
echo "t238-native-acceptance: free DRM seat; running 4K/144 trials"

workload_failed=0
for windows in 2 10 30; do
    BACKEND=drm \
    COMPOSITOR_BIN="$COMPOSITOR" \
    WINDOWS="$windows" \
    ARMS="split split-deco" \
    WORKLOADS="scroll drag" \
    DURATION="$DURATION" \
    TRIALS="$TRIALS" \
    WARMUP="$SETTLE" \
    PIN_TIER=full \
    SHUFFLE=1 \
    OUT_DIR="$run_dir/windows-$windows" \
        ./scripts/t238-material-workloads.sh \
        >"$run_dir/windows-$windows.log" 2>&1 || workload_failed=1
done

gate_exit=0
python3 scripts/t238-material-workloads-driver.py native-gate "$run_dir" \
    --trials "$TRIALS" >"$run_dir/gate.txt" || gate_exit=$?
if [ "$gate_exit" -eq 1 ]; then
    result="FAIL (frame budget exceeded or input samples dropped)"
elif [ "$gate_exit" -ne 0 ] || [ "$workload_failed" -eq 1 ]; then
    result="OPEN (native workload evidence incomplete)"
    gate_exit=2
else
    result="PASS (all native workload checks passed)"
fi

mkdir -p "$(dirname "$OUT_TRACE")"
{
    echo "T238 — controlled native acceptance"
    echo "===================================="
    echo
    echo "recorded: $recorded"
    echo "host:     $host"
    echo "result:   $result"
    echo "reports:  $run_dir"
    echo
    echo "DRM master probe:"
    echo "$seat_summary"
    echo
    echo "Trials: duration=${DURATION}s trials=${TRIALS} windows=2,10,30 arms=split,split-deco"
    echo
    cat "$run_dir/gate.txt"
    echo
    for windows in 2 10 30; do
        echo "--- windows=$windows ---"
        if ! sed -n '1,80p' "$run_dir/windows-$windows/report.md" 2>/dev/null; then
            echo "(no report.md; workload log follows)"
            tail -40 "$run_dir/windows-$windows.log" 2>/dev/null || true
        fi
        echo
    done
    echo "--- systemd app isolation ---"
    cat "$scratch/systemd.txt" 2>/dev/null || echo "systemd user manager unavailable"
    echo "systemd multi-instance result: $([ "$systemd_ok" = "1" ] && echo PASS || echo OPEN)"
} >"$OUT_TRACE"

echo "t238-native-acceptance: $result; wrote $OUT_TRACE"
echo "t238-native-acceptance: systemd multi-instance $([ "$systemd_ok" = "1" ] && echo PASS || echo OPEN)"
[ "$systemd_ok" = "1" ] || echo "t238-native-acceptance: rerun the systemd check on a host with a user manager" >&2
exit "$gate_exit"
