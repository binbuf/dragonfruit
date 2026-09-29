#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-03.4 DRM soak and teardown.
#
# Two parts, recorded together:
#
#   1. the automated teardown soak — `dragonfruit dev --soak N` (default 100)
#      runs N compositor sessions and asserts each exit leaves no leaked
#      socket, lock, launch token, or orphaned client. This always runs and is
#      the same gate `make soak` / `make check` use.
#   2. one DRM session cycle — `dragonfruit dev --soak 1 --drm` (the same
#      teardown check on the real backend), which needs a free logind seat.
#      After it, the seat probe is re-run to prove the compositor released DRM
#      master (the VT). With no free seat that half is recorded OPEN, not
#      skipped (docs/SLICING-REVIEW.md).
#
# Artifacts:
#   * free seat:    docs/captures/t03-drm-soak.txt
#   * no free seat: docs/captures/t03-drm-soak.open.txt
#
# Always exits 0: the automated soak result is recorded in the artifact, and a
# no-seat host must not fail the pipeline. Re-run `make drm-soak` on a machine
# with a free seat / spare GPU / clean VM.
#
# Overrides: OUT_SOAK, OUT_OPEN, SOAK_CYCLES, DEV_BIN, SETTLE.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT_SOAK="${OUT_SOAK:-docs/captures/t03-drm-soak.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t03-drm-soak.open.txt}"
SOAK_CYCLES="${SOAK_CYCLES:-100}"
SETTLE="${SETTLE:-5}"
DEV_BIN="${DEV_BIN:-target/debug/dragonfruit}"
COMPOSITOR="${COMPOSITOR:-target/debug/dragonfruit-compositor}"

[ -x "$DEV_BIN" ] || {
    echo "drm-soak: $DEV_BIN not built (run make cargo-build)" >&2
    exit 1
}
command -v python3 >/dev/null 2>&1 || {
    echo "drm-soak: python3 not found" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-drm-soak.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

# --- 1. automated teardown soak (headless) -------------------------------
soak_log="$scratch/soak.log"
soak_rc=0
"$DEV_BIN" dev --soak "$SOAK_CYCLES" >"$soak_log" 2>&1 || soak_rc=$?
soak_summary="$(tail -n 1 "$soak_log" 2>/dev/null || true)"
[ -n "$soak_summary" ] || soak_summary="(no output)"

# --- 2. one DRM session cycle --------------------------------------------
probe_before="$scratch/seat-before.json"
probe_rc=0
python3 scripts/drm-seat-probe.py >"$probe_before" || probe_rc=$?

seat_summary="$(python3 - "$probe_before" "$probe_rc" <<'PY'
import json
import sys

report = json.load(open(sys.argv[1]))
rc = int(sys.argv[2])
if report.get("cards"):
    for card in report["cards"]:
        state = "free" if card["can_set_master"] else (
            f"busy ({card['error'] or 'master held'})" if card["opened"] else
            f"unopenable ({card['error']})")
        print(f"  - {card['path']}: {state}")
elif rc == 2:
    print("  - no /dev/dri/card* node")
else:
    print("  - no DRM card reported")
PY
)"

mkdir -p "$(dirname "$OUT_OPEN")"
recorded="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
host="$(uname -srm)"

write_automated() {
    echo "Automated teardown soak (headless, $SOAK_CYCLES cycles):"
    if [ "$soak_rc" -eq 0 ]; then
        echo "  PASS — $soak_summary"
    else
        echo "  FAIL (exit $soak_rc) — $soak_summary"
    fi
    echo
}

if [ "$probe_rc" -ne 0 ]; then
    # No free seat: record the open, with the backend's own no-op-seat
    # self-report so the artifact carries real bring-up evidence. The no-op
    # seat opens the card directly and never steals master from the host.
    socket="dragonfruit-t03-drmsoak-$$"
    self_report="$scratch/self-report.txt"
    if [ -x "$COMPOSITOR" ]; then
        LIBSEAT_BACKEND=noop timeout "$SETTLE" "$COMPOSITOR" --backend drm \
            --socket-name "$socket" >"$self_report" 2>&1 || true
    fi
    runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
    rm -f "$runtime/$socket" "$runtime/$socket.lock" \
          "$runtime/$socket.launch-token" "$runtime/$socket.desktop-launch-token"
    marker="$(grep -m1 'DRM bring-up:' "$self_report" 2>/dev/null || true)"

    {
        echo "T-03.4 — DRM soak and teardown: OPEN"
        echo "===================================="
        echo
        echo "recorded:  $recorded"
        echo "host:      $host"
        echo "result:    DRM session cycle OPEN (no free logind seat)"
        echo
        write_automated
        echo "DRM master probe (before the cycle):"
        echo "$seat_summary"
        echo
        echo "Backend self-report (no-op seat, host session untouched):"
        if [ -n "$marker" ]; then
            echo "  $marker"
        else
            echo "  (the compositor did not reach its bring-up checkpoint)"
        fi
        echo
        echo "Why: the host runs a graphical session that owns DRM master, so the"
        echo "     compositor cannot take the VT for a real DRM cycle. The T-03.4"
        echo "     unit is marked open — not skipped — and swept on the hardware"
        echo "     rail (a free seat / spare GPU / clean VM)."
        echo
        echo "Reproduce on a machine with a free seat:"
        echo "  make drm-soak"
        echo "  # or directly, one DRM cycle with the teardown check:"
        echo "  $DEV_BIN dev --soak 1 --drm"
        echo
        echo "The automated soak above is the same gate as \`make soak\`."
    } >"$OUT_OPEN"
    echo "drm-soak: OPEN (no free seat); automated soak $([ "$soak_rc" -eq 0 ] && echo PASS || echo FAIL)"
    echo "drm-soak: wrote $OUT_OPEN"
    exit 0
fi

# A free seat: run one real DRM session cycle with the teardown check.
drm_log="$scratch/drm.log"
drm_rc=0
"$DEV_BIN" dev --soak 1 --drm >"$drm_log" 2>&1 || drm_rc=$?
drm_summary="$(tail -n 1 "$drm_log" 2>/dev/null || true)"
[ -n "$drm_summary" ] || drm_summary="(no output)"

# Re-probe: master must be free again (the compositor released the VT).
probe_after="$scratch/seat-after.json"
after_rc=0
python3 scripts/drm-seat-probe.py >"$probe_after" || after_rc=$?
if [ "$after_rc" -eq 0 ]; then
    master_state="RELEASED (a card accepts master again)"
else
    master_state="STILL HELD (no card accepts master — possible leak)"
fi

mkdir -p "$(dirname "$OUT_SOAK")"
{
    echo "T-03.4 — DRM soak and teardown: $([ "$drm_rc" -eq 0 ] && echo PASS || echo FAIL)"
    echo "==========================================================="
    echo
    echo "recorded:  $recorded"
    echo "host:      $host"
    echo "result:    DRM session cycle $([ "$drm_rc" -eq 0 ] && echo PASS || echo "FAIL (exit $drm_rc)")"
    echo
    write_automated
    echo "DRM master probe (before the cycle):"
    echo "$seat_summary"
    echo
    echo "DRM session cycle (\`$DEV_BIN dev --soak 1 --drm\`):"
    if [ "$drm_rc" -eq 0 ]; then
        echo "  PASS — $drm_summary"
    else
        echo "  FAIL (exit $drm_rc) — $drm_summary"
    fi
    echo
    echo "DRM master after the cycle: $master_state"
    echo "  (the teardown check itself asserts no leaked socket, lock, token,"
    echo "   or orphaned client; see the cycle output above)"
    echo
    echo "Reproduce:"
    echo "  make drm-soak"
    echo "  # or directly, one DRM cycle with the teardown check:"
    echo "  $DEV_BIN dev --soak 1 --drm"
} >"$OUT_SOAK"
echo "drm-soak: $([ "$drm_rc" -eq 0 ] && echo PASS || echo FAIL); master $master_state"
echo "drm-soak: wrote $OUT_SOAK"
exit 0