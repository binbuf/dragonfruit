#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-17.5b leak and lock enforcement verification (premium gate).
#
# Writes docs/captures/t17-leak-lock-soak.txt: the premium gate re-proves two
# contracts on the release tree.
#
#   1. Repeated loops leak nothing. `dragonfruit dev --soak N` runs N
#      compositor sessions and asserts each exit removes its socket, lock,
#      both launch tokens, and the Xwayland `DISPLAY` file, and leaves no
#      orphaned client. The soak here runs against an **isolated
#      `XDG_RUNTIME_DIR`**, so the transcript can assert the directory is
#      empty afterwards instead of trusting the run's own report.
#   2. Lock enforcement. `session_lock_conformance` locks a headless session,
#      proves an untrusted client cannot lock, kills the lock UI (the session
#      stays locked, fail-secure), and loops 25 lock/unlock cycles asserting
#      the compositor's `query lock` state returns to `locked=0 surfaces=0`
#      every time (no lock or lock-surface leak). Also isolated-runtime-dir
#      so the run cannot litter the host.
#   3. The VT/DRM master half is a probe: a free logind seat is required for a
#      real DRM cycle, so on a busy host the row is recorded OPEN (not
#      skipped) and `make drm-soak` owns the hardware rail (T-03.4/T-17.2).
#
# This task changes no user-visible surface; the live nested check is the
# companion `docs/captures/t17-leak-lock-soak.md`.
#
# Overrides: OUT, SOAK_CYCLES, DEV_BIN, KEEP_SCRATCH.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t17-leak-lock-soak.txt}"
SOAK_CYCLES="${SOAK_CYCLES:-100}"
KEEP_SCRATCH="${KEEP_SCRATCH:-0}"
fail=0

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*"; }
command_line() { printf '$ %s\n' "$*"; }

# Prefer a release (packaged-tree) build if it exists; fall back to the debug
# tree the `make check` gate builds. The transcript records which one ran.
if [ -z "${DEV_BIN:-}" ]; then
    if [ -x target/release/dragonfruit ] && [ -x target/release/dragonfruit-compositor ]; then
        DEV_BIN=target/release/dragonfruit
        BUILD_KIND=release
    else
        DEV_BIN=target/debug/dragonfruit
        BUILD_KIND=debug
    fi
else
    BUILD_KIND=custom
fi

# The lock suite is a `cargo test`; run it optimized too when the release tree
# exists, so the conformance proof is against the same artifact the soak uses.
if [ "$BUILD_KIND" = "release" ]; then
    TEST_PROFILE=(--release)
else
    TEST_PROFILE=()
fi

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

scratchs=()
# Print a fresh scratch dir; the caller appends it to `scratchs` so cleanup
# runs in this shell (a command substitution would append in a subshell).
new_scratch() {
    mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t17-ll.XXXXXX"
}

{
    echo "T-17.5b leak and lock enforcement verification (premium gate)"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "dev binary: $DEV_BIN ($BUILD_KIND build)"
    echo "soak cycles: $SOAK_CYCLES"
    echo "state:     headless; no host session, no VM, no real services"

    section "0. The leak and lock contracts"
    cat <<'TABLE'
| Question | Contract | Proof |
|---|---|---|
| A compositor session exits | no socket, lock, launch token, DISPLAY file, or orphan | `dragonfruit dev --soak N` (isolated runtime dir) |
| The process leaked none of the above | the isolated runtime dir is empty | T-17.5b's own find over the scratch dir |
| A trusted client locks | `locked=1 surfaces=1` on every output | `session_lock_conformance::lock_covers_every_output_and_confirms` |
| An untrusted client requests a lock | refused; the session never locks | `session_lock_conformance::untrusted_client_cannot_lock` |
| The lock UI is killed while locked | the session stays locked (fail-secure) | `session_lock_conformance::locked_input_targets_the_lock_ui_and_survives_its_death` |
| Lock/unlock repeats 25 times | `locked=0 surfaces=0` after each; no lock-surface leak | `session_lock_conformance::repeated_lock_cycles_never_leak_lock_state` |
| A real DRM session exits | DRM master (the VT) is released | probe here; full cycle is `make drm-soak` (hardware) |
TABLE

    section "1. Repeated loops with the leak tripwire (isolated runtime dir)"
    echo "The tripwire is soak::teardown_artifacts: socket, socket.lock,"
    echo ".launch-token, .desktop-launch-token, and .x11-display. The soak runs"
    echo "with XDG_RUNTIME_DIR set to a scratch dir so the check is hermetic"
    echo "(the host's own stale test files cannot mask or inflate the result)."
    soak_dir="$(new_scratch)"
    scratchs+=("$soak_dir")
    printf 'isolated XDG_RUNTIME_DIR: %s\n\n' "$soak_dir"
    if [ ! -x "$DEV_BIN" ]; then
        echo "[FAIL] $DEV_BIN is not built (run 'make cargo-build')"
        fail=1
    else
        command_line "XDG_RUNTIME_DIR=$soak_dir $DEV_BIN dev --soak $SOAK_CYCLES"
        if XDG_RUNTIME_DIR="$soak_dir" "$DEV_BIN" dev --soak "$SOAK_CYCLES"; then
            leftovers="$(find "$soak_dir" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort)"
            if [ -z "$leftovers" ]; then
                printf '\n[PASS] no leaked socket/token/DISPLAY file after %s cycles\n' "$SOAK_CYCLES"
            else
                printf '\n[FAIL] leaked artifacts after %s cycles:\n%s\n' "$SOAK_CYCLES" "$leftovers"
                fail=1
            fi
        else
            printf '\n[FAIL] the teardown soak reported a dirty cycle\n'
            fail=1
        fi
    fi

    section "2. Lock enforcement (packaged/release tree)"
    echo "The lock suite runs with its own isolated XDG_RUNTIME_DIR; afterwards"
    echo "the directory must be empty (the harness now removes the socket, both"
    echo "tokens, and the Xwayland DISPLAY file on every path, including a"
    echo "hard kill)."
    lock_dir="$(new_scratch)"
    scratchs+=("$lock_dir")
    printf 'isolated XDG_RUNTIME_DIR: %s\n\n' "$lock_dir"
    command_line "XDG_RUNTIME_DIR=$lock_dir cargo test ${TEST_PROFILE[*]} -p dragonfruit-compositor --test session_lock_conformance"
    if XDG_RUNTIME_DIR="$lock_dir" cargo test "${TEST_PROFILE[@]}" -p dragonfruit-compositor --test session_lock_conformance; then
        leftovers="$(find "$lock_dir" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort)"
        if [ -z "$leftovers" ]; then
            printf '\n[PASS] lock suite clean; no leaked runtime files\n'
        else
            printf '\n[FAIL] lock suite left runtime files:\n%s\n' "$leftovers"
            fail=1
        fi
    else
        printf '\n[FAIL] lock enforcement suite failed\n'
        fail=1
    fi

    section "3. VT / DRM master probe"
    if command -v python3 >/dev/null 2>&1; then
        probe_json="$(python3 scripts/drm-seat-probe.py 2>&1)"
        probe_rc=$?
        printf '$ python3 scripts/drm-seat-probe.py\n%s\n\n' "$probe_json"
        if [ "$probe_rc" -eq 0 ]; then
            echo "[PASS] a card accepted (and immediately released) DRM master;"
            echo "       a free seat is available for a real DRM cycle (make drm-soak)."
        else
            echo "[OPEN] no free logind seat: the host session holds DRM master,"
            echo "       so the real DRM cycle — and its post-cycle VT-release probe —"
            echo "       is the T-03.4 hardware rail (make drm-soak / runbook)."
            echo "       Marked OPEN, not skipped (docs/SLICING-REVIEW.md)."
        fi
    else
        echo "[OPEN] python3 not found; the DRM seat probe could not run."
    fi

    section "4. Notes"
    echo "* The soak's own tripwire (tools/dragonfruit-dev/src/soak.rs) now"
    echo "  includes the Xwayland DISPLAY file, and every compositor integration"
    echo "  test harness removes its artifacts in Drop (compositor/tests/common),"
    echo "  so repeated loops no longer accumulate in \$XDG_RUNTIME_DIR."
    echo "* VT master after a clean session is asserted by the compositor's own"
    echo "  backend teardown and re-probed by scripts/drm-soak.sh on hardware."
    echo "* The packaged RPM/DEB re-verification is deferred to T-16.9/T-16.10"
    echo "  (they have not landed); this unit verifies the release tree, which is"
    echo "  what those packages are built from."
} >"$OUT" 2>&1

if [ "$KEEP_SCRATCH" != "1" ]; then
    for d in "${scratchs[@]}"; do rm -rf "$d"; done
fi

if [ "$fail" -ne 0 ]; then
    echo "t17-leak-lock-soak: at least one row FAILED (see $OUT)" >&2
    exit 1
fi
echo "t17-leak-lock-soak: wrote $OUT (all rows passed)"