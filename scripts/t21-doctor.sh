#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-21 capture preflight. Run before a nested/host capture task:
#
#   make t21-doctor
#
# It checks the host session, the built binaries, the optional nested-compositor
# helpers, and the screenshot output path. Warnings are non-fatal; failures mean
# the capture cannot run on this host.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0
warn=0

ok()   { printf 'ok    %s\n' "$*"; }
warnf(){ printf 'warn  %s\n' "$*"; warn=$((warn + 1)); }
bad()  { printf 'FAIL  %s\n' "$*"; fail=$((fail + 1)); }

has() { command -v "$1" >/dev/null 2>&1; }

printf 'T-21 capture doctor\n'
printf 'root: %s\n' "$PWD"
printf 'host: %s\n' "$(uname -srm)"
printf 'session: WAYLAND_DISPLAY=%s XDG_RUNTIME_DIR=%s\n' \
    "${WAYLAND_DISPLAY:-<unset>}" "${XDG_RUNTIME_DIR:-<unset>}"
printf '\n'

if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -d "${XDG_RUNTIME_DIR:-}" ]; then
    ok "XDG_RUNTIME_DIR exists"
else
    bad "XDG_RUNTIME_DIR is unset or missing"
fi

if [ -n "${WAYLAND_DISPLAY:-}" ]; then
    if [ -S "${XDG_RUNTIME_DIR:-}/$WAYLAND_DISPLAY" ]; then
        ok "host Wayland socket exists"
    elif [ -S "$WAYLAND_DISPLAY" ]; then
        ok "host Wayland socket exists (absolute)"
    else
        warnf "WAYLAND_DISPLAY=$WAYLAND_DISPLAY is set but the socket was not found"
    fi
else
    bad "WAYLAND_DISPLAY is unset; nested captures will write an OPEN artifact"
fi

for path in \
    build/shell/src/dragonfruit-shell \
    target/debug/dragonfruit-compositor; do
    if [ -x "$path" ]; then
        ok "$path"
    else
        bad "$path missing; run make build"
    fi
done

for bin in python3; do
    if has "$bin"; then ok "$bin"; else bad "$bin missing"; fi
done

if has kwin_wayland; then
    ok "kwin_wayland (virtual nested host)"
else
    warnf "kwin_wayland missing; use the live host session or another nested host"
fi

if has xmessage; then
    ok "xmessage (popover fixtures)"
else
    warnf "xmessage missing; Dock popover fixtures may be empty"
fi

if has spectacle; then
    ok "spectacle (host stills)"
elif has grim; then
    ok "grim (host stills)"
elif has import; then
    ok "import (host stills)"
else
    warnf "no spectacle/grim/import; use the compositor capture fixture"
fi

shots="${SHOTS:-$HOME/Pictures/Screenshots}"
if mkdir -p "$shots" 2>/dev/null && [ -w "$shots" ]; then
    ok "screenshot dir writable: $shots"
else
    warnf "screenshot dir not writable: $shots"
fi

if [ -f scripts/t21-capture-lib.sh ]; then
    ok "scripts/t21-capture-lib.sh"
else
    warnf "scripts/t21-capture-lib.sh missing"
fi

printf '\n'
if [ "$fail" -ne 0 ]; then
    printf 'FAIL: %d blocking issue(s), %d warning(s)\n' "$fail" "$warn"
    exit 1
fi
printf 'PASS: %d warning(s)\n' "$warn"
