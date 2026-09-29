#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-16.4 suspend/resume soak.
#
# Writes docs/captures/t16-suspend-resume-soak.txt: the 100-cycle soak and its
# two proofs.
#
#   1. Compositor soak. `suspend_resume_conformance`'s
#      `one_hundred_suspend_resume_cycles_leak_no_state` runs one headless
#      compositor session through 100 suspend→resume cycles with a live
#      Wayland client. Every cycle asserts the scene is intact on both sides of
#      sleep (`outputs=1 windows=1`), the completed-cycle counter advances by
#      exactly one, and the suspend flag returns to awake. After the 100th wake
#      the client connection still round-trips and input still routes, and a
#      clean `SIGTERM` leaves no socket, lock, launch token, or `DISPLAY` file.
#      The suite runs under an isolated `XDG_RUNTIME_DIR`, so the transcript can
#      assert that directory is empty afterwards instead of trusting the run's
#      own report.
#   2. Session soak. `dragonfruit-session`'s
#      `one_hundred_cycles_keep_the_supervised_session_alive` drives the same
#      cycle 100 times through the real `Supervisor` and the recording
#      `MockSuspend`: one platform suspend request per cycle, the supervised
#      child's pid never changes, and no service restarts.
#   3. The real-hardware half is a probe. A true 100-cycle suspend/resume soak
#      needs a disposable machine where logind can actually sleep the box (a
#      suspend on this host would sleep the developer machine), so on this host
#      the row is recorded OPEN (not skipped) and the hardware rail owns it.
#
# This task has no user-visible surface; the live nested check is the companion
# `docs/captures/t16-suspend-resume-soak.md`.
#
# Overrides: OUT, KEEP_SCRATCH.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t16-suspend-resume-soak.txt}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
fail=0

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*"; }
command_line() { printf '$ %s\n' "$*"; }

# Prefer a release build if it exists; fall back to the debug tree `make
# cargo-build` produces. The transcript records which one ran.
if [ -x target/release/dragonfruit-compositor ]; then
    TEST_PROFILE=(--release)
    BUILD_KIND=release
else
    TEST_PROFILE=()
    BUILD_KIND=debug
fi

scratchs=()
new_scratch() { mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t16-sr.XXXXXX"; }

run() {
    local label="$1"
    shift
    command_line "$@"
    if "$@"; then
        printf '\n[PASS] %s\n' "$label"
    else
        printf '\n[FAIL] %s\n' "$label"
        fail=1
    fi
}

{
    echo "T-16.4 suspend/resume soak"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "host:      $(uname -srm)"
    echo "build:     $BUILD_KIND"
    echo "state:     headless; no host session, no VM, no real services"

    section "0. The soak contract"
    cat <<'TABLE'
| Question | Contract | Proof |
|---|---|---|
| 100 cycles in one session | `suspended=0 cycles=100` | `suspend_resume_conformance::one_hundred_suspend_resume_cycles_leak_no_state` |
| Output recovery | `outputs=1` on both sides of every cycle | same test (asserted each cycle) |
| Client recovery | the client connection round-trips after cycle 100 | same test |
| Input recovery | input routes again after the final wake | same test |
| No leaked state on exit | socket, lock, both tokens, `DISPLAY` file all removed | same test's clean-`SIGTERM` teardown check |
| The session survives 100 cycles | same child pid, no restart | `dragonfruit-session::one_hundred_cycles_keep_the_supervised_session_alive` |
| A real machine sleeps 100 times | logind `PrepareForSleep` round trips end to end | hardware rail (OPEN here) |
TABLE

    section "1. Compositor 100-cycle soak (isolated runtime dir)"
    echo "One headless compositor session, one live Wayland client, 100"
    echo "suspend→resume cycles. Artifacts (socket, lock, tokens, DISPLAY) are"
    echo "asserted gone after a clean SIGTERM, and the isolated runtime dir must"
    echo "be empty so the run cannot litter (or mask) the host."
    soak_dir="$(new_scratch)"
    scratchs+=("$soak_dir")
    printf 'isolated XDG_RUNTIME_DIR: %s\n\n' "$soak_dir"
    command_line "XDG_RUNTIME_DIR=$soak_dir cargo test ${TEST_PROFILE[*]} -p dragonfruit-compositor --test suspend_resume_conformance"
    if XDG_RUNTIME_DIR="$soak_dir" cargo test "${TEST_PROFILE[@]}" \
        -p dragonfruit-compositor --test suspend_resume_conformance; then
        leftovers="$(find "$soak_dir" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort)"
        if [ -z "$leftovers" ]; then
            printf '\n[PASS] 100 cycles clean; no leaked runtime files\n'
        else
            printf '\n[FAIL] leaked artifacts after the soak:\n%s\n' "$leftovers"
            fail=1
        fi
    else
        printf '\n[FAIL] the suspend/resume conformance suite failed\n'
        fail=1
    fi

    section "2. Session 100-cycle soak (one supervised child)"
    echo "The same cycle 100 times through the real Supervisor and the"
    echo "recording MockSuspend: one platform request per cycle, the child's pid"
    echo "never changes, and no service restarts."
    run "session 100-cycle soak" \
        cargo test "${TEST_PROFILE[@]}" -p dragonfruit-session --test suspend \
        one_hundred_cycles_keep_the_supervised_session_alive -- --nocapture

    section "3. Real hardware / logind probe"
    logind_note="loginctl not found"
    if command -v loginctl >/dev/null 2>&1; then
        if loginctl list-sessions >/dev/null 2>&1; then
            logind_note="logind reachable ($(loginctl list-sessions 2>/dev/null | awk 'NR>1{n++} END{print n+0}') session(s))"
        else
            logind_note="loginctl present but no bus/session"
        fi
    fi
    echo "logind: $logind_note"
    echo
    echo "[OPEN] A true 100-cycle suspend/resume soak calls logind's suspend"
    echo "       and actually sleeps the machine 100 times, then verifies"
    echo "       output/input/client recovery on wake. On this host that would"
    echo "       sleep the developer's own graphical session, and the production"
    echo "       logind SuspendBackend is not wired (T-12.5b/real-session work)."
    echo "       Marked OPEN, not skipped (docs/SLICING-REVIEW.md). The hardware"
    echo "       rail is T-159…T-161 (free seat / spare GPU / clean VM) plus the"
    echo "       T-16 VM matrix; the automated halves above pin the state machine,"
    echo "       the scene counts, and the leak tripwire without hardware."

    section "4. Notes"
    echo "* The persisted-use ADR is docs/design/adr/0181-t16-suspend-resume-soak.md."
    echo "* The one-cycle recovery test is"
    echo "  suspend_resume_conformance::a_suspend_resume_cycle_recovers_outputs_input_and_clients,"
    echo "  and the compositor's pure model is compositor/src/suspend.rs."
    echo "* The soak's leak list mirrors common::compositor_artifacts and"
    echo "  tools/dragonfruit-dev/src/soak.rs::teardown_artifacts (socket, lock,"
    echo "  both launch tokens, Xwayland DISPLAY file)."
} >"$OUT" 2>&1

if [ "$KEEP_SCRATCH" != "1" ]; then
    for d in "${scratchs[@]}"; do rm -rf "$d"; done
fi

if [ "$fail" -ne 0 ]; then
    echo "t16-suspend-resume-soak: at least one row FAILED (see $OUT)" >&2
    exit 1
fi
echo "t16-suspend-resume-soak: wrote $OUT (all rows passed)"