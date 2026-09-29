#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.2 DRM first bring-up.
#
# Runs the never-executed DRM backend (`dragonfruit-compositor --backend drm`)
# and records what comes back. The DRM/KMS rail needs a *free* logind seat; on
# a workstation already running a desktop the host compositor holds DRM
# master, so the backend reports OPEN and this script records that as an
# explicit, non-skipped open:
#
#   * with a free card:          docs/captures/t03-drm-loop-trace.txt
#   * with no free card/seat:    docs/captures/t03-drm-loop.open.txt
#
# Either way the script exits 0: the hardware rail consumes the artifact, and
# a no-seat host must not fail the pipeline (`docs/SLICING-REVIEW.md` — the
# unit is marked open, not skipped). Re-run `make drm-bringup` on a machine
# with a seat.
#
# Overrides: OUT_TRACE, OUT_OPEN (artifact paths), SETTLE (startup seconds).

set -euo pipefail

cd "$(dirname "$0")/.."

OUT_TRACE="${OUT_TRACE:-docs/captures/t03-drm-loop-trace.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t03-drm-loop.open.txt}"
SETTLE="${SETTLE:-5}"

COMPOSITOR="${COMPOSITOR:-target/debug/dragonfruit-compositor}"
[ -x "$COMPOSITOR" ] || {
    echo "drm-bringup: $COMPOSITOR not built (run make cargo-build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "drm-bringup: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-drm.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

probe="$scratch/probe.json"
# Exit 0 = a card accepted master; 1 = busy; 2 = no card node.
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
    if [ "$probe_rc" -eq 2 ]; then
        reason="no seat: no DRM card node (/dev/dri/card*) on this host"
    fi

    # Run the DRM backend as far as the host allows, without logind (so the
    # host session is never disturbed): the no-op seat opens the card
    # directly, the busy master makes every connector fail, and the backend
    # reaches its own no-output checkpoint. This is the backend's self-report
    # for the artifact.
    socket="dragonfruit-t03-drm-$$"
    self_report="$scratch/self-report.txt"
    LIBSEAT_BACKEND=noop timeout 20 "$COMPOSITOR" --backend drm \
        --socket-name "$socket" >"$self_report" 2>&1 || true
    rm -f "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket" \
          "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket.lock" \
          "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket.launch-token"
    self_marker="$(grep -m1 'DRM bring-up:' "$self_report" || true)"

    mkdir -p "$(dirname "$OUT_OPEN")"
    {
        echo "T-03.2 — DRM first bring-up: OPEN"
        echo "=================================="
        echo
        echo "recorded:   $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "result:     DRM bring-up: OPEN ($reason)"
        echo "host:       $(uname -srm)"
        echo
        echo "DRM master probe:"
        echo "$seat_summary"
        if [ -r /run/systemd/seats/seat0 ]; then
            echo
            echo "seat0 (/run/systemd/seats/seat0):"
            sed 's/^/  - /' /run/systemd/seats/seat0
        fi
        echo
        echo "Backend self-report (no-op seat, host session untouched):"
        if [ -n "$self_marker" ]; then
            echo "  $self_marker"
        else
            echo "  (the compositor did not reach its bring-up checkpoint)"
        fi
        echo
        echo "Why: the host runs a graphical session that owns DRM master, so the"
        echo "     DRM backend cannot scan out. The T-03.2 unit is marked open —"
        echo "     not skipped — and is swept on the hardware rail (T-03.4 owns the"
        echo "     dedicated-user second-VT/VM runbook)."
        echo
        echo "Reproduce on a machine with a free seat / spare GPU / clean VM:"
        echo "  make drm-bringup"
        echo "  # or directly:"
        echo "  $COMPOSITOR --backend drm --socket-name t03-drm"
        echo
        echo "The trace artifact on success is $OUT_TRACE."
    } >"$OUT_OPEN"
    echo "drm-bringup: OPEN ($reason)"
    echo "drm-bringup: wrote $OUT_OPEN"
    exit 0
fi

socket="dragonfruit-t03-drm-$$"
log="$scratch/drm.log"
echo "drm-bringup: free DRM master found; starting the DRM backend ($socket)"
setsid "$COMPOSITOR" --backend drm --socket-name "$socket" >"$log" 2>&1 &
pgid=$!

cleanup() {
    kill -TERM -"$pgid" 2>/dev/null || true
    sleep 1
    kill -KILL -"$pgid" 2>/dev/null || true
}
trap 'cleanup; rm -rf "$scratch"' EXIT

# Wait for the socket (success) or the process to exit (bring-up failure).
for _ in $(seq 1 300); do
    kill -0 "$pgid" 2>/dev/null || break
    [ -e "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$socket" ] && break
    sleep 0.1
done
sleep "$SETTLE"

# Frame trace: SIGUSR1 dumps the render-path counters without ending the
# session (append-only contract, ADR 0009).
if kill -0 "$pgid" 2>/dev/null; then
    kill -USR1 -"$pgid" 2>/dev/null || true
    sleep 1
fi

cleanup
wait "$pgid" 2>/dev/null || true

mkdir -p "$(dirname "$OUT_TRACE")"
cp "$log" "$OUT_TRACE"
marker="$(grep -m1 'DRM bring-up:' "$log" || true)"
if [ -z "$marker" ]; then
    marker="DRM bring-up: OPEN (backend exited before reporting a marker)"
fi
echo "drm-bringup: $marker"
echo "drm-bringup: wrote $OUT_TRACE"
# The interactive T-01 loop capture on DRM (shell + clients + a still) is the
# T-03.4 runbook's step; this script records the bring-up and the frame trace.
if [[ "$marker" == *" READY "* ]]; then
    echo "drm-bringup: READY — run the T-03.4 runbook for the full loop capture"
fi
exit 0