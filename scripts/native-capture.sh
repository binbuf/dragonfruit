#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Dev-only native-session capture.
#
# `make native` installs a per-user systemd unit that runs this script with
# `--watch` when the Dragonfruit session starts. It waits for the compositor,
# records a startup snapshot (units, processes, DRM state, journals, and a
# SIGUSR1 render-stats dump), then waits for the session to end and records a
# final snapshot. `native-recover`/`native-return` stop and remove the unit;
# the capture reports themselves stay under the user state directory.
#
# The wiring is intentionally dev-only and disposable. The reports are the
# useful artifact: point them at a bug report, compare two runs, or hand them
# to the next dogfooding session.
#
# Usage:
#   scripts/native-capture.sh            # one snapshot, then exit
#   scripts/native-capture.sh --watch    # startup + final snapshot (unit mode)
#
# Environment:
#   DF_CAPTURE_ROOT    report root (default: $XDG_STATE_HOME/dragonfruit/native-captures)
#   DF_CAPTURE_SETTLE  seconds to let the session settle before the first snapshot (10)
#   DF_CAPTURE_KEEP    number of session directories to keep (20)
#   DF_SESSION_SOCKET  compositor socket name (dragonfruit-wayland)

set -uo pipefail

mode="${1:-once}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
capture_root="${DF_CAPTURE_ROOT:-${XDG_STATE_HOME:-$HOME/.local/state}/dragonfruit/native-captures}"
settle="${DF_CAPTURE_SETTLE:-10}"
keep="${DF_CAPTURE_KEEP:-20}"
socket="${DF_SESSION_SOCKET:-dragonfruit-wayland}"
runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
units=(
    dragonfruit-session.target
    dragonfruit-compositor.service
    dragonfruit-shell.service
    dragonfruit-settingsd.service
    dragonfruit-menu-broker.service
    dragonfruit-app-index.service
    dragonfruit-notifications.service
    dragonfruit-system-status.service
    dragonfruit-wallpaperd.service
    dragonfruit-portal.service
)

log() {
    printf 'native-capture: %s\n' "$*" >&2
}

capture_root="${capture_root%/}"
mkdir -p "$capture_root" || {
    log "cannot create $capture_root"
    exit 1
}
dir="$capture_root/$(date +%Y%m%d-%H%M%S)-$$"
mkdir -p "$dir" || exit 1

main_pid() {
    systemctl --user show -p MainPID --value dragonfruit-compositor.service 2>/dev/null || true
}

wait_for_session() {
    local i
    # A forced/manual run outside the session should still produce a report
    # immediately rather than waiting the full readiness window.
    if [ "${DF_CAPTURE_FORCE:-0}" = "1" ] \
        && ! systemctl --user is-active --quiet dragonfruit-compositor.service; then
        return 1
    fi
    for i in $(seq 1 160); do
        if [ -S "$runtime/$socket" ] \
            && systemctl --user is-active --quiet dragonfruit-compositor.service; then
            return 0
        fi
        sleep 0.25
    done
    return 1
}

journal_for() {
    local tag="$1"
    local args=()
    local unit
    for unit in "${units[@]}"; do
        args+=(-u "$unit")
    done
    timeout 25 journalctl --user -b --no-pager -n 6000 "${args[@]}" \
        >"$dir/journal-$tag.txt" 2>&1 || true
    # A short, greppable index. Keep the broad journal for context.
    grep -aEi \
        'DRM bring-up|Multi-GPU|render stats|frame timing|presentation stats|material stats|degrade stats|client authenticated|start failed|panic|segfault|abort|QML|TypeError|failed|refused|token|session lock|suspend|resume' \
        "$dir/journal-$tag.txt" 2>/dev/null \
        | tail -n 400 >"$dir/key-$tag.txt" || true
}

perf_probe() {
    # Opt-in: this briefly maps an animated test window. It answers the
    # question the stats cannot: what frame rate does a real client actually
    # receive from this session?
    if [ "${DF_CAPTURE_PERF:-0}" != "1" ]; then
        return
    fi
    echo "## Frame-rate probe"
    if command -v vkcube >/dev/null 2>&1; then
        local start end elapsed
        start="$(date +%s%N)"
        if WAYLAND_DISPLAY="$socket" timeout 8 vkcube --wsi wayland --c 120 \
            --suppress_popups >/dev/null 2>&1; then
            end="$(date +%s%N)"
            elapsed="$(( (end - start) / 1000000 ))"
            if [ "$elapsed" -gt 0 ]; then
                awk -v frames=120 -v ms="$elapsed" \
                    'BEGIN { printf "vkcube: 120 frames in %d ms = %.2f fps\n", ms, frames * 1000 / ms }'
            fi
        else
            echo "vkcube: 120 frames did not complete within 8 s (<15 fps)"
        fi
    else
        echo "vkcube: not installed"
    fi
    if command -v glxgears >/dev/null 2>&1 && [ -n "${DISPLAY:-}" ]; then
        echo "glxgears:"
        DISPLAY="$DISPLAY" timeout 6 glxgears -info 2>&1 \
            | grep -aE 'Running synchronized|frames in' | tail -n 3 || true
    fi
}

snapshot() {
    local tag="$1"
    local out="$dir/snapshot-$tag.txt"
    local unit

    {
        echo "# Dragonfruit native capture: $tag"
        echo
        echo "timestamp: $(date -Is)"
        echo "host: $(uname -a)"
        echo "repo: $root"
        echo "session:"
        echo "  XDG_CURRENT_DESKTOP=${XDG_CURRENT_DESKTOP:-<unset>}"
        echo "  XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-<unset>}"
        echo "  XDG_SESSION_ID=${XDG_SESSION_ID:-<unset>}"
        echo "  XDG_VTNR=${XDG_VTNR:-<unset>}"
        echo "  XDG_SEAT=${XDG_SEAT:-<unset>}"
        echo "  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-<unset>}"
        echo "  DISPLAY=${DISPLAY:-<unset>}"
        echo
        echo "## Units"
        timeout 10 systemctl --user list-units --no-pager --all 'dragonfruit-*' 2>&1 || true
        echo
        echo "## Unit properties"
        for unit in "${units[@]}"; do
            echo "### $unit"
            timeout 10 systemctl --user show --no-pager \
                -p Id -p LoadState -p ActiveState -p SubState -p Result \
                -p NRestarts -p MainPID -p ActiveEnterTimestamp "$unit" 2>&1 || true
            echo
        done
        echo "## Session target status"
        timeout 10 systemctl --user status --no-pager dragonfruit-session.target 2>&1 || true
        echo
        echo "## Processes"
        ps -eo pid,ppid,stat,etime,rss,pcpu,comm,args --sort=-rss 2>/dev/null \
            | awk 'NR == 1 || /dragonfruit|Xwayland/' || true
        echo
        echo "## DRM"
        ls -l /dev/dri 2>&1 || true
        for card in /sys/class/drm/card*-*; do
            [ -e "$card/status" ] || continue
            echo "### $card"
            for field in status enabled modes dpms; do
                [ -r "$card/$field" ] && printf '%s: %s\n' "$field" "$(cat "$card/$field")"
            done
        done
        echo
        echo "## Kernel"
        printf 'cmdline: '; cat /proc/cmdline 2>/dev/null || true
        lsmod 2>/dev/null | grep -E '^(i915|amdgpu|nvidia|nouveau|xe|radeon)' || true
        echo
        echo "## Session hand-off files"
        ls -l "$runtime/$socket"* 2>&1 || true
        echo
        echo "## Wayland globals"
        if command -v wayland-info >/dev/null 2>&1; then
            WAYLAND_DISPLAY="$socket" timeout 8 wayland-info 2>&1 | head -n 240 || true
        else
            echo "wayland-info not installed"
        fi
        echo
        perf_probe
    } >"$out" 2>&1

    journal_for "$tag"
    log "wrote $dir ($tag)"
}

prune() {
    # Keep the newest `keep` session directories; each report is small, but
    # dogfooding should not grow the state directory forever.
    find "$capture_root" -mindepth 1 -maxdepth 1 -type d -printf '%T@ %p\n' 2>/dev/null \
        | sort -rn \
        | tail -n +$((keep + 1)) \
        | cut -d' ' -f2- \
        | while read -r old; do
            [ -n "$old" ] && rm -rf -- "$old"
        done
}

if [ "${XDG_CURRENT_DESKTOP:-}" != "dragonfruit" ] \
    && ! systemctl --user is-active --quiet dragonfruit-compositor.service; then
    if [ "${DF_CAPTURE_FORCE:-0}" != "1" ]; then
        log "no active Dragonfruit native session (use DF_CAPTURE_FORCE=1 to record anyway)"
        exit 1
    fi
fi

if wait_for_session; then
    sleep "$settle"
    pid="$(main_pid)"
    if [ -n "$pid" ] && [ "$pid" != "0" ]; then
        # Dump render counters/frame timing into the journal for this snapshot.
        kill -USR1 "$pid" 2>/dev/null || true
        sleep 1
    fi
    snapshot startup
else
    log "the Dragonfruit session did not become ready; recording what exists"
    snapshot startup
fi

if [ "$mode" = "--watch" ] || [ "$mode" = "watch" ]; then
    cleanup() {
        trap - TERM INT
        snapshot final
        prune
        exit 0
    }
    trap cleanup TERM INT
    while :; do
        sleep 30
    done
fi

prune
printf '%s\n' "$dir"
