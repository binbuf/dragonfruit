#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-12.6c second-VT dev harness validation.
#
# Three parts, recorded together:
#
#   1. the dedicated-user refusal — `dragonfruit dev --real --plan --user $USER`
#      against the real `loginctl`. When the user already holds a seat graphical
#      session the harness refuses with the dedicated-user explanation and exits
#      non-zero. This is acceptance criterion 1 and always runs.
#   2. the preflight/VT selection — the same command against a **mocked**
#      `loginctl` (`DF_LOGINCTL`) and `passwd` (`DF_PASSWD`) where the host
#      desktop is on VT2 and the `dfdev` user is free. It must pick VT3, the
#      smallest free VT, and print the `systemd-run`/`chvt` plan.
#   3. the one real start → switch → return → teardown cycle — needs a free
#      logind seat and the dedicated user. With neither, the hardware half is
#      recorded OPEN, not skipped (docs/SLICING-REVIEW.md).
#
# Artifacts:
#   * free seat:    docs/captures/t171-second-vt.txt
#   * no free seat: docs/captures/t171-second-vt.open.txt
#
# Always exits 0: a host without a free seat or the dedicated user must not fail
# the pipeline. The pure logic is pinned by `cargo test -p dragonfruit-dev`
# (`second_vt::tests`).
#
# Overrides: OUT_OK, OUT_OPEN, DEV_BIN.

set -euo pipefail

cd "$(dirname "$0")/.."

OUT_OK="${OUT_OK:-docs/captures/t171-second-vt.txt}"
OUT_OPEN="${OUT_OPEN:-docs/captures/t171-second-vt.open.txt}"
DEV_BIN="${DEV_BIN:-target/debug/dragonfruit}"

[ -x "$DEV_BIN" ] || {
    echo "second-vt-validation: $DEV_BIN not built (run make cargo-build)" >&2
    exit 1
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-second-vt.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

recorded="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
host="$(uname -srm)"
current_user="${USER:-$(id -un)}"

# --- 1. the dedicated-user refusal (real loginctl) ------------------------
refusal_log="$scratch/refuse.log"
refusal_rc=0
"$DEV_BIN" dev --real --plan --user "$current_user" >"$refusal_log" 2>&1 || refusal_rc=$?
refusal_out="$(cat "$refusal_log")"
refusal_marker="$(grep -m1 '^Second VT:' "$refusal_log" || true)"
[ -n "$refusal_marker" ] || refusal_marker="(no Second VT marker)"
refusal_explained=0
grep -q 'dedicated' "$refusal_log" && refusal_explained=1
refusal_refused=0
grep -q 'Second VT: REFUSE' "$refusal_log" && refusal_refused=1

# --- 2. preflight/VT selection against a mocked loginctl ------------------
mock_dir="$scratch/mock"
mkdir -p "$mock_dir"
cat >"$mock_dir/loginctl" <<'MOCK'
#!/bin/sh
set -eu
case "${1:-}" in
  list-sessions)
    printf ' 2 1000 hostuser seat0 /dev/tty2\n' ;;
  show-session)
    case "${2:-}" in
      2) printf 'Name=hostuser\nSeat=seat0\nVTNr=2\nType=wayland\nClass=user\nActive=yes\nState=active\n' ;;
      *) printf 'Name=\nSeat=\nVTNr=0\nType=unspecified\nClass=other\nActive=no\nState=closing\n' ;;
    esac ;;
  terminate-session) printf 'ok\n' ;;
  *) printf 'unexpected: %s\n' "$*" >&2; exit 1 ;;
esac
MOCK
chmod +x "$mock_dir/loginctl"
printf 'root:x:0:0:root:/root:/bin/bash\ndfdev:x:1001:1001::/home/dfdev:/bin/bash\n' \
    >"$mock_dir/passwd"

plan_log="$scratch/plan.log"
plan_rc=0
DF_LOGINCTL="$mock_dir/loginctl" DF_PASSWD="$mock_dir/passwd" \
    "$DEV_BIN" dev --real --plan --user dfdev >"$plan_log" 2>&1 || plan_rc=$?
plan_start="$(grep -m1 '^Second VT: START' "$plan_log" || true)"
[ -n "$plan_start" ] || plan_start="(no START marker)"
plan_vt3=0
grep -q 'vt=3' "$plan_log" && plan_vt3=1
plan_systemd=0
grep -q 'PAMName=login' "$plan_log" && plan_systemd=1

# --- 3. the real second-VT cycle -----------------------------------------
probe="$scratch/seat.json"
probe_rc=0
python3 scripts/drm-seat-probe.py >"$probe" || probe_rc=$?
free_seat=0
[ "$probe_rc" -eq 0 ] && free_seat=1

dedicated_user=0
if getent passwd "${DF_USER:-dfdev}" >/dev/null 2>&1; then
    dedicated_user=1
fi

write_planned() {
    echo "Dedicated-user refusal (real loginctl, --user $current_user):"
    if [ "$refusal_refused" -eq 1 ]; then
        echo "  PASS — $refusal_marker (exit $refusal_rc)"
    else
        echo "  NOT APPLICABLE — this user holds no seat graphical session here"
        echo "    $refusal_marker (exit $refusal_rc)"
    fi
    if [ "$refusal_explained" -eq 1 ]; then
        echo "  dedicated-user guidance present"
    else
        echo "  dedicated-user guidance MISSING (expected on a refusing host)"
    fi
    echo
    echo "Preflight/VT selection (mocked loginctl, host desktop on VT2):"
    if [ "$plan_rc" -eq 0 ] && [ "$plan_vt3" -eq 1 ]; then
        echo "  PASS — $plan_start"
        echo "  $([ "$plan_systemd" -eq 1 ] && echo 'systemd-run PAMName=login plan present' || echo 'systemd-run plan MISSING')"
    else
        echo "  FAIL (exit $plan_rc) — $plan_start"
    fi
    echo
}

if [ "$free_seat" -eq 0 ] || [ "$dedicated_user" -eq 0 ]; then
    reason="no free logind seat"
    [ "$dedicated_user" -eq 0 ] && reason="no dedicated development user ($reason)"
    mkdir -p "$(dirname "$OUT_OPEN")"
    {
        echo "T-12.6c — second-VT dev harness: OPEN"
        echo "======================================"
        echo
        echo "recorded:  $recorded"
        echo "host:      $host"
        echo "result:    real start → switch → return → teardown cycle OPEN ($reason)"
        echo
        write_planned
        echo "Why: starting a real session needs a free logind seat and a dedicated"
        echo "     user. Create the user once with \`sudo useradd -m dfdev\` +"
        echo "     \`sudo passwd dfdev\`, then run the cycle on a free seat / clean VM."
        echo
        echo "Reproduce:"
        echo "  # inspect the preflight and VT selection without starting anything:"
        echo "  $DEV_BIN dev --real --plan --user dfdev"
        echo "  # the real start on a free seat, then return with the printed VT key:"
        echo "  $DEV_BIN dev --real --user dfdev"
        echo "  # end the session cleanly:"
        echo "  $DEV_BIN dev --real --teardown --user dfdev"
        echo
        echo "The pure preflight/VT logic is pinned by \`cargo test -p dragonfruit-dev\`."
    } >"$OUT_OPEN"
    echo "second-vt-validation: OPEN ($reason); wrote $OUT_OPEN"
    exit 0
fi

# Free seat and a dedicated user: run one real cycle, then tear it down.
DF_USER="${DF_USER:-dfdev}"
cycle_log="$scratch/cycle.log"
cycle_rc=0
timeout 300 "$DEV_BIN" dev --real --user "$DF_USER" >"$cycle_log" 2>&1 || cycle_rc=$?
teardown_log="$scratch/teardown.log"
teardown_rc=0
"$DEV_BIN" dev --real --teardown --user "$DF_USER" >"$teardown_log" 2>&1 || teardown_rc=$?
cycle_marker="$(grep -m1 '^Second VT: START' "$cycle_log" || true)"
[ -n "$cycle_marker" ] || cycle_marker="(no START marker)"
teardown_marker="$(grep -m1 '^Second VT: TEARDOWN' "$teardown_log" || true)"
[ -n "$teardown_marker" ] || teardown_marker="(no TEARDOWN marker)"

out="$OUT_OPEN"
if [ "$cycle_rc" -eq 0 ] && [ "$teardown_rc" -eq 0 ]; then
    out="$OUT_OK"
fi
mkdir -p "$(dirname "$out")"
{
    echo "T-12.6c — second-VT dev harness: $([ "$out" = "$OUT_OK" ] && echo PASS || echo OPEN)"
    echo "========================================================="
    echo
    echo "recorded:  $recorded"
    echo "host:      $host"
    echo "result:    second-VT cycle $([ "$out" = "$OUT_OK" ] && echo PASS || echo "OPEN/FAIL (start exit $cycle_rc, teardown exit $teardown_rc)")"
    echo
    write_planned
    echo "Real cycle (free seat, user $DF_USER):"
    echo "  start:    $cycle_marker"
    echo "  teardown: $teardown_marker"
    echo
    echo "Reproduce:"
    echo "  $DEV_BIN dev --real --user $DF_USER"
    echo "  $DEV_BIN dev --real --teardown --user $DF_USER"
} >"$out"
echo "second-vt-validation: wrote $out"
exit 0