#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.5a absent-daemon and crash matrix verification (premium gate).
#
# Writes docs/captures/t17-robustness-matrix.txt: the premium gate re-runs the
# two robustness contracts on the release tree and adds the one case neither
# predecessor covered — a service that cannot even be *spawned* must not block
# session start.
#
# Rows:
#   * session start  every optional service absent at startup; the session
#                    still reaches Running, an absent gate never deadlocks a
#                    later stage, and only the compositor's absence ends it
#                    (T-17.5a, `services/session/tests/absent_services.rs`)
#   * absent daemons every shipped Settings pane mounts with no settingsd, no
#                    portal, and no bridge host (T-15.16), and every adapter
#                    read projects absent, never an error
#   * crash/kill     the T-16.8a session supervision matrix and the T-16.8b
#                    restart-policy/compositor-death matrix
#   * app + lock     a crashed app leaves the desktop running; a killed lock
#                    UI stays locked (fail-secure)
#
# No host session, no VM, and no real services are needed; the compositor rows
# drive real Wayland clients against the headless backend.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t17-robustness-matrix.txt}"
BUILD="${BUILD_DIR:-build}"
fail=0

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*"; }
command_line() { printf '$ %s\n' "$*"; }

# Run a matrix row, streaming its output into the report and recording the
# exit status so the whole matrix fails if any row does.
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
    echo "T-17.5a absent-daemon and crash matrix verification (premium gate)"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "build:     $BUILD"
    echo "state:     headless; no host session, no VM, no real services"

    section "0. The robustness contract"
    cat <<'TABLE'
| Question | Contract | Proof |
|---|---|---|
| An optional service is absent at startup | degrade; never block start | absent_services::every_absent_optional_service_degrades_and_never_blocks_start |
| Any gate service fails to spawn | the next stage still starts | absent_services::an_absent_gate_service_does_not_block_the_next_stage |
| The compositor (anchor) is absent | the session ends by design | absent_services::only_the_anchor_absence_ends_the_session |
| A settings provider is absent | every shipped pane mounts and stays live | tst_settings_absence (private bus) |
| A host daemon is absent | the adapter projects `present: false`, never an error | system-status / adapters `absent` rows |
| A supervised service is killed | restarts per policy; the session keeps running | kill_matrix |
| A service leaves any way (0/non-zero/signal) | restart decision per policy | restart_policy_matrix |
| The compositor dies | the session ends; no restart | kill_matrix, restart_policy_matrix |
| An app crashes | the desktop and the other app keep running | window_conformance |
| The lock UI is killed while locked | the session stays locked | session_lock_conformance |
TABLE

    section "1. Session start with every optional service absent (T-17.5a)"
    echo "The stand-in plan is derived from SessionPlan::default_session with"
    echo "every non-anchor program replaced by a binary that does not exist."
    echo "The session must reach Running, each absent service reporting Failed"
    echo "exactly once, and only the compositor's absence may end it."
    run "session absent-service matrix" \
        cargo test -p dragonfruit-session --test absent_services -- --nocapture

    section "2. The absent-daemon masking matrix (T-15.16, re-run)"
    if [ -f "$BUILD/CTestTestfile.cmake" ]; then
        run "Settings absent-provider gate" \
            ctest --test-dir "$BUILD" -R 'tst_settings_absence' --output-on-failure
    else
        echo "(build/ not configured; run 'make build', then this row)"
    fi
    run "bridge host adapter absence" \
        cargo test -p dragonfruit-system-status absent -- --nocapture
    run "generic adapter absence" \
        cargo test -p dragonfruit-system-adapters --test subscription absent -- --nocapture

    section "3. The crash/kill matrix (T-16.8a/T-16.8b, re-run)"
    echo "Every restartable shipped service is killed with a real SIGKILL and"
    echo "its documented recovery asserted; the restart policy is crossed with"
    echo "every exit kind, and compositor death ends the session for all three."
    run "session kill matrix" \
        cargo test -p dragonfruit-session --test kill_matrix -- --nocapture
    run "restart policy matrix" \
        cargo test -p dragonfruit-session --test restart_policy_matrix -- --nocapture

    section "4. App crash and the lock UI's fail-secure kill"
    echo "A hard-closed app client leaves the compositor and the survivor"
    echo "running; hard-closing the lock client's socket must leave the"
    echo "session locked, never unlocked."
    run "compositor app-crash" \
        cargo test -p dragonfruit-compositor --test window_conformance \
        a_crashed_app_leaves_the_compositor_and_the_other_app_running -- --nocapture
    run "lock kill-resistance" \
        cargo test -p dragonfruit-compositor --test session_lock_conformance \
        locked_input_targets_the_lock_ui_and_survives_its_death -- --nocapture

    section "5. Live/VM half (not automated here)"
    echo "A real kill -9 of the shipped binaries under a running session, and"
    echo "stopping real daemons in a VM, is the design's real-hardware check."
    echo "No VM and no free session are available in CI, so the live nested"
    echo "desktop check is recorded in docs/captures/t17-robustness-matrix.md;"
    echo "the per-daemon live matrix remains docs/captures/t15-absence-matrix.md"
    echo "and the live kill drill docs/captures/t16-kill-matrix.md."
} >"$OUT" 2>&1

if [ "$fail" -ne 0 ]; then
    echo "t17-robustness-matrix: at least one row FAILED (see $OUT)" >&2
    exit 1
fi
echo "t17-robustness-matrix: wrote $OUT (all rows passed)"