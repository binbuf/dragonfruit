#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21.2c refresh-aware hysteresis load trace (T189d).
#
# Runs two deterministic sections:
#
#   1. the pure `DegradeController` tests at 60/120/144 Hz, recording the
#      wall-clock downgrade/recovery windows and a 60 s steady-sweep run at
#      each rate;
#   2. the headless compositor load sequence through the real
#      `observe_rendered_frame` path, recording `query degrade` and the
#      per-role `df_output.material_role_state` events for Full, Reduced,
#      Minimal, the latched sweep, and the idle recovery.
#
# It also runs the shell policy/Dock/menu-bar property tests when the CMake
# tree is configured. The trace is headless and deterministic, so it does not
# need a host Wayland session; run `make t21-doctor` first when the same trace
# is used as host/nested evidence.
#
# Writes docs/captures/t21-degrade-load.txt. Not part of `make e2e`.
#
# Overrides: OUT (report path), CARGO (default `cargo`).
set -euo pipefail

cd "$(dirname "$0")/.."

OUT="${OUT:-docs/captures/t21-degrade-load.txt}"
CARGO="${CARGO:-cargo}"
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t21-degrade.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT

mkdir -p "$(dirname "$OUT")"

# The preflight is advisory here: this trace is a headless simulation. Keep
# the verdict in the artifact so a later host/nested run can see the state.
DOCTOR="$(make t21-doctor 2>&1 || true)"
DOCTOR_VERDICT="$(printf '%s\n' "$DOCTOR" | tail -1)"

# 1. Pure hysteresis trace (60/120/144 Hz + the 60 s steady sweep).
"$CARGO" test -p dragonfruit-compositor --bin dragonfruit-compositor \
    window::degrade::tests -- --nocapture --test-threads=1 >"$SCRATCH/pure.log" 2>&1
# The libtest progress line shares the first trace line; normalize the prefix.
grep -a 'degrade-trace:' "$SCRATCH/pure.log" \
    | sed 's/^.*degrade-trace:/degrade-trace:/' >"$SCRATCH/pure.trace" || true

# 2. Real compositor load sequence (headless synthetic frame feed).
"$CARGO" test -p dragonfruit-compositor --test shell_protocol_conformance \
    material_role_state_follows_a_refresh_aware_load_sequence -- --nocapture \
    --test-threads=1 >"$SCRATCH/load.log" 2>&1
grep -a 'degrade-trace:' "$SCRATCH/load.log" \
    | sed 's/^.*degrade-trace:/degrade-trace:/' >"$SCRATCH/load.trace" || true

# 3. The shell's applied properties for the same states.
SHELL_RESULT="skipped (build/ is not configured)"
if [ -f build/build.ninja ]; then
    if ctest --test-dir build \
        -R '^(tst_materialpolicy|tst_dock|tst_menubar)$' \
        --output-on-failure >"$SCRATCH/shell.log" 2>&1; then
        SHELL_SUMMARY="$(grep -oE '[0-9]+% tests passed, [0-9]+ tests failed out of [0-9]+' \
            "$SCRATCH/shell.log" | tail -1 || true)"
        SHELL_RESULT="PASS (${SHELL_SUMMARY:-ctest passed})"
    else
        SHELL_RESULT="FAIL (see the ctest log)"
    fi
fi

require() {
    local file="$1" pattern="$2" label="$3"
    if ! grep -aqE "$pattern" "$file"; then
        echo "t21-degrade-load: missing $label in $file" >&2
        return 1
    fi
}

verdict=0
require "$SCRATCH/pure.trace" 'rate_hz=60 ' 'the 60 Hz window' || verdict=1
require "$SCRATCH/pure.trace" 'rate_hz=120 ' 'the 120 Hz window' || verdict=1
require "$SCRATCH/pure.trace" 'rate_hz=144 ' 'the 144 Hz window' || verdict=1
require "$SCRATCH/pure.trace" 'steady_sweep_hz=144 .*latched=1' \
    'the latched 144 Hz sweep' || verdict=1
require "$SCRATCH/load.trace" 'phase=steady-over .*roles=0:1/1/0,1:1/1/0,2:1/1/0,3:1/1/0' \
    'the reduced per-role load state' || verdict=1
require "$SCRATCH/load.trace" 'phase=sweep-flap .*latched=1' \
    'the flap latch' || verdict=1
require "$SCRATCH/load.trace" 'phase=forced-minimal .*roles=0:1/0/4,1:1/0/4,2:2/0/0,3:2/0/0' \
    'the forced Minimal persistent floor' || verdict=1

{
    echo "T-21.2c refresh-aware hysteresis load trace (T189d)"
    echo "timestamp: $(date -Is)"
    echo "host: $(uname -srm)"
    echo "session: XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
    echo "preflight: make t21-doctor -> ${DOCTOR_VERDICT:-<no output>}"
    echo "command: cargo test -p dragonfruit-compositor --bin dragonfruit-compositor window::degrade::tests -- --nocapture"
    echo "command: cargo test -p dragonfruit-compositor --test shell_protocol_conformance material_role_state_follows_a_refresh_aware_load_sequence -- --nocapture"
    echo "shell: ctest -R '^(tst_materialpolicy|tst_dock|tst_menubar)$' -> $SHELL_RESULT"
    echo
    echo "## Pure hysteresis: 12/180 frames at 60 Hz = 200 ms down / 3 s up"
    echo "The frame counts scale with the refresh period; the wall-clock windows do not."
    cat "$SCRATCH/pure.trace"
    echo
    echo "## Headless compositor: synthetic rendered-frame load sequence"
    echo "budget 16666 us, GPU path, sampled blur; roles are tier/blur/fallback."
    cat "$SCRATCH/load.trace"
    echo
    echo "## Verdict"
    if [ "$verdict" -eq 0 ]; then
        echo "PASS: 60/120/144 Hz windows are refresh-aware; the steady sweep is latched;"
        echo "      forced Full/Reduced/Minimal keep the per-role floor; shell properties: $SHELL_RESULT."
    else
        echo "FAIL: one or more expected trace lines are missing."
    fi
} >"$OUT"

if [ "$verdict" -ne 0 ]; then
    echo "t21-degrade-load: FAIL; wrote $OUT" >&2
    exit 1
fi

echo "t21-degrade-load: PASS; wrote $OUT"
