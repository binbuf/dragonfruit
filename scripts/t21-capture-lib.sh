#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Shared helpers for the T-21 nested/host capture scripts.
#
# Source it from a script that already sets `set -euo pipefail`:
#
#   . "$(dirname "$0")/t21-capture-lib.sh"
#   t21_capture_init "t21-example"
#   t21_require_host "$OUT" "T-21 example"
#   t21_start_compositor nested "$SCRATCH/compositor.log"
#   t21_start_shell "$SCRATCH/shell.log" DRAGONFRUIT_CHROME_STATS=1
#   t21_dump_stats "$T21_SHELL_PID" "$SCRATCH/shell.log" SIGUSR1-1
#   t21_capture_still 7000 "$OUT_PNG" "$before_shot"
#   t21_stop_shell
#   t21_stop_compositor
#
# The library owns the compositor/shell PIDs and installs an EXIT trap. New
# scripts should not install their own cleanup trap; use `t21_capture_cleanup`
# if one is unavoidable.
#
# Environment:
#   XDG_RUNTIME_DIR   required for the nested rail
#   WAYLAND_DISPLAY   required for the nested rail (host session)
#   SCRATCH           optional scratch dir (default: mktemp)
#   SOCKET            optional nested socket name (default: unique per PID)
#   KEEP_SCRATCH=1    keep the scratch dir on exit
#   SHOTS             optional screenshot output dir (default: docs/captures)

T21_CAPTURE_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
T21_ROOT="$(cd "$T21_CAPTURE_LIB_DIR/.." && pwd)"
T21_NAME="t21-capture"
T21_SCRATCH=""
T21_SOCKET=""
T21_SYNTH=""
T21_COMPOSITOR_PID=""
T21_SHELL_PID=""
T21_X11_PIDS=""

t21_capture_init() {
    T21_NAME="${1:-t21-capture}"
    T21_SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-${T21_NAME}.XXXXXX")}"
    T21_SOCKET="${SOCKET:-dragonfruit-${T21_NAME}-$$}"
    T21_SYNTH="${XDG_RUNTIME_DIR:-}/$T21_SOCKET.synth"
    mkdir -p "$T21_SCRATCH"
    trap t21_capture_cleanup EXIT
}

t21_capture_cleanup() {
    t21_stop_shell >/dev/null 2>&1 || true
    for pid in $T21_X11_PIDS; do
        kill -TERM "$pid" 2>/dev/null || true
    done
    T21_X11_PIDS=""
    t21_stop_compositor >/dev/null 2>&1 || true
    if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
        rm -f "$XDG_RUNTIME_DIR/$T21_SOCKET" \
              "$XDG_RUNTIME_DIR/$T21_SOCKET.lock" \
              "$XDG_RUNTIME_DIR/$T21_SOCKET.launch-token" \
              "$T21_SYNTH" 2>/dev/null || true
    fi
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$T21_SCRATCH"
}

t21_write_open() {
    local out="$1" title="$2" reason="$3"
    mkdir -p "$(dirname "$out")"
    {
        echo "$title"
        echo "timestamp: $(date -Is)"
        echo "verdict: OPEN"
        echo "reason: $reason"
        echo "rerun: WAYLAND_DISPLAY=<host-socket> make ${T21_NAME}"
    } >"$out"
    echo "$T21_NAME: OPEN ($reason); wrote $out" >&2
}

t21_require_host() {
    local out="$1" title="$2"
    if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
        t21_write_open "$out" "$title" "XDG_RUNTIME_DIR is not set"
        exit 0
    fi
    if [ -z "${WAYLAND_DISPLAY:-}" ]; then
        t21_write_open "$out" "$title" "no host Wayland session (WAYLAND_DISPLAY is unset)"
        exit 0
    fi
}

t21_require_exec() {
    local path="$1" hint="$2"
    if [ ! -x "$path" ]; then
        echo "$T21_NAME: $path missing or not executable; $hint" >&2
        exit 1
    fi
}

t21_wait_for_file() {
    local path="$1" tries="${2:-300}" delay="${3:-0.1}"
    local i
    for i in $(seq 1 "$tries"); do
        [ -e "$path" ] && return 0
        sleep "$delay"
    done
    return 1
}

t21_start_compositor() {
    local backend="${1:-nested}" log="$2"
    rm -f "$XDG_RUNTIME_DIR/$T21_SOCKET" \
          "$XDG_RUNTIME_DIR/$T21_SOCKET.lock" \
          "$XDG_RUNTIME_DIR/$T21_SOCKET.launch-token" \
          "$T21_SYNTH"
    DRAGONFRUIT_SYNTHETIC_INPUT="$T21_SYNTH" \
        "$T21_ROOT/target/debug/dragonfruit-compositor" \
        --backend "$backend" --socket-name "$T21_SOCKET" \
        >"$log" 2>&1 &
    T21_COMPOSITOR_PID=$!
    if ! t21_wait_for_file "$XDG_RUNTIME_DIR/$T21_SOCKET" 300 0.1; then
        echo "$T21_NAME: compositor socket never appeared; log:" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    if ! t21_wait_for_file "$T21_SYNTH" 100 0.1; then
        echo "$T21_NAME: synthetic-input socket never appeared; log:" >&2
        tail -30 "$log" >&2
        exit 1
    fi
    sleep 0.5
}

t21_stop_compositor() {
    [ -n "$T21_COMPOSITOR_PID" ] || return 0
    kill -TERM "$T21_COMPOSITOR_PID" 2>/dev/null || true
    local i
    for i in $(seq 1 150); do
        kill -0 "$T21_COMPOSITOR_PID" 2>/dev/null || break
        sleep 0.1
    done
    wait "$T21_COMPOSITOR_PID" 2>/dev/null || true
    T21_COMPOSITOR_PID=""
}

t21_start_shell() {
    local log="$1"
    shift
    local env_args=("$@")
    env "${env_args[@]}" WAYLAND_DISPLAY="$T21_SOCKET" \
        "$T21_ROOT/build/shell/src/dragonfruit-shell" \
        --socket-name "$T21_SOCKET" >"$log" 2>&1 &
    T21_SHELL_PID=$!
    local i
    for i in $(seq 1 240); do
        grep -aq "menu bar configured" "$log" 2>/dev/null && return 0
        if ! kill -0 "$T21_SHELL_PID" 2>/dev/null; then
            echo "$T21_NAME: shell exited early; log:" >&2
            tail -40 "$log" >&2
            exit 1
        fi
        sleep 0.1
    done
    echo "$T21_NAME: shell never configured its menu bar; log:" >&2
    tail -40 "$log" >&2
    exit 1
}

t21_stop_shell() {
    [ -n "$T21_SHELL_PID" ] || return 0
    kill -TERM "$T21_SHELL_PID" 2>/dev/null || true
    wait "$T21_SHELL_PID" 2>/dev/null || true
    T21_SHELL_PID=""
}

t21_send_synth() {
    python3 - "$T21_SYNTH" "$1" <<'PY'
import socket
import sys

sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
sock.sendto(sys.argv[2].encode() + b"\n", sys.argv[1])
PY
}

t21_dump_stats() {
    local pid="$1" log="$2" label="$3" pattern="${4:-chrome stats}"
    kill -USR1 "$pid" 2>/dev/null || true
    local i
    for i in $(seq 1 80); do
        grep -aq "$pattern ($label)" "$log" 2>/dev/null && break
        sleep 0.1
    done
    sleep 0.4
    grep -a "$pattern ($label)" "$log" || true
}

t21_newest_screenshot() {
    find "$HOME/Pictures/Screenshots" -maxdepth 1 -name 'Screenshot_*.png' \
        -printf '%T@ %p\n' 2>/dev/null | sort -n | tail -1 | cut -d' ' -f2- || true
}

t21_capture_still() {
    local delay_ms="$1" out="$2" before="${3:-}"
    local i
    for i in $(seq 1 $((delay_ms / 100 + 120))); do
        [ -s "$out" ] && return 0
        sleep 0.1
    done
    local saved
    saved="$(t21_newest_screenshot)"
    if [ -n "$saved" ] && [ "$saved" != "$before" ] && [ -s "$saved" ]; then
        cp "$saved" "$out"
        return 0
    fi
    echo "$T21_NAME: no capture appeared for $out" >&2
    return 1
}

t21_field() {
    local name="$1" line="$2"
    sed -n "s/.* ${name}=\\([0-9][0-9]*\\).*/\\1/p" <<<"$line" | tail -1
}

t21_surface_line() {
    local file="$1" surface="$2"
    grep -a "surface=$surface " "$file" | tail -1 || true
}

t21_surface_ok() {
    local file="$1" surface="$2" want="$3"
    local line commits copies
    line="$(t21_surface_line "$file" "$surface")"
    [ -n "$line" ] || return 1
    grep -q "path=$want " <<<"$line" || return 1
    commits="$(t21_field dmabuf_commits "$line")"
    [ -n "$commits" ] || return 1
    if [ "$want" = "dmabuf" ]; then
        [ "$commits" -gt 0 ] || return 1
        copies="$(t21_field copy_bytes "$line")"
        [ "$copies" = "0" ] || return 1
    fi
    return 0
}
