#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-16.8a crash/kill matrix (headless reproduction).
#
# Writes docs/captures/t16-kill-matrix.txt: for every restartable component it
# records the exact reproduction (a cargo invocation) and its output. The
# suite is headless: the session matrix supervises real `sleep` stand-ins
# through the same `Supervisor` the session binary uses, and the compositor
# suite drives real Wayland clients against the headless backend. No host
# session, no VM, and no real services are needed.
#
# Rows:
#   * shell / lock UI  restarts (`always`); the session keeps running
#   * settingsd, menu-broker, app-index, notifications, wallpaperd
#                      restart (`on-failure`); the session keeps running
#   * portal backend   restarts (`on-failure`) and fails soft
#   * app (a client)   a plain client exit: never restarted, not a session event
#   * lock UI          fail-secure: killing it never unlocks
#   * compositor       ends the session; every other service is stopped;
#                      never restarted (the behavior T-16.8b documents)
#
# T-16.8b adds the restart-policy matrix: every policy crossed with every exit
# kind (`exit 0`, non-zero, signal), and compositor death crossed with the
# same three exits (`services/session/tests/restart_policy_matrix.rs`).
#
# The live/VM half (killing the real binaries under a running session) is
# recorded by hand in docs/captures/t16-kill-matrix.md.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t16-kill-matrix.txt}"
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
    echo "T-16.8a crash/kill matrix (headless reproduction)"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "state:     no host session, no VM, no real services"

    section "0. The documented matrix"
    cat <<'TABLE'
| Killed | Outcome |
|---|---|
| shell / lock UI | restarts (`always`); the session keeps running |
| settingsd, menu-broker, app-index, notifications, wallpaperd | restart (`on-failure`); the session keeps running |
| portal backend | restarts (`on-failure`) and fails soft; the session keeps running |
| app (a user-launched client) | a plain client exit: never restarted; the session keeps running |
| lock UI (a crash while locked) | fail-secure: the session stays locked, never unlocks |
| compositor | the session ends; every other service is stopped; never restarted |
TABLE

    section "1. Session supervision — every restartable shipped service"
    echo "The stand-in plan is derived from SessionPlan::default_session, so a"
    echo "new service or a changed policy is exercised without editing the test."
    echo "The matrix kills each service with a real SIGKILL and asserts the"
    echo "shipped recovery, and that an app kill is not a session event."
    run "session kill matrix" \
        cargo test -p dragonfruit-session --test kill_matrix -- --nocapture

    section "1b. The restart-policy matrix (T-16.8b)"
    echo "Every policy is crossed with every exit kind (exit 0, non-zero,"
    echo "signal), and compositor death is crossed with the same three exits."
    echo "The compositor ends the session and is never restarted for any of"
    echo "them; always/on-failure/never restart exactly as documented."
    run "restart policy matrix" \
        cargo test -p dragonfruit-session \
        --test restart_policy_matrix -- --nocapture

    section "2. An app client crash leaves the desktop running"
    echo "Two independent real Wayland clients map windows; one hard-closes its"
    echo "socket (a crash). The compositor must stay alive, drop only the dead"
    echo "app's window, and keep serving the survivor."
    run "compositor app-crash" \
        cargo test -p dragonfruit-compositor \
        --test window_conformance \
        a_crashed_app_leaves_the_compositor_and_the_other_app_running -- --nocapture

    section "3. The lock UI's fail-secure kill"
    echo "The lock UI is the shell; the compositor owns the lock. Hard-closing"
    echo "the lock client's socket must leave the session locked — never"
    echo "unlocked — and this test also blankets the shell/lock restart row."
    run "lock kill-resistance" \
        cargo test -p dragonfruit-compositor \
        --test session_lock_conformance \
        locked_input_targets_the_lock_ui_and_survives_its_death -- --nocapture

    section "4. Live/VM half (not automated here)"
    echo "Killing the real dragonfruit-shell/settingsd/dragonfruit-notifications/"
    echo "xdg-desktop-portal-dragonfruit binaries under a running session and"
    echo "watching each recover is the design's real-hardware check (no VM is"
    echo "available in CI). The nested session itself is exercised by 'make demo'"
    echo "and the T-16 breadth captures; the reviewed live state is recorded by"
    echo "hand in docs/captures/t16-kill-matrix.md."
} >"$OUT" 2>&1

if [ "$fail" -ne 0 ]; then
    echo "t16-kill-matrix: at least one row FAILED (see $OUT)" >&2
    exit 1
fi
echo "t16-kill-matrix: wrote $OUT (all rows passed)"