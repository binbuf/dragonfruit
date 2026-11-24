#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T335b standard-client ScreenCast check.
#
# Proves that a standard portal client receives *multiple live frames* through
# the Dragonfruit ScreenCast portal: a client calls `CreateSession` /
# `SelectSources` / `Start` on the session's real
# `org.freedesktop.portal.Desktop`, the shell's source picker completes the
# request, and the returned PipeWire node is read with GStreamer's
# `pipewiresrc` for several buffers.
#
# The live half needs a shell bridge that reaches the compositor's producer
# (`df_toplevel_manager.create_screencast`, v17). That bridge is not part of
# this task: the portal and its stream transport answer with the named
# `pipewire-producer-unavailable` stills fallback until it lands, so a live
# node cannot exist in a shipped session yet. This script therefore records the
# precondition verdict and the exact rerun command instead of a false PASS;
# see docs/design/adr/0345-screencast-bridge-and-portal-ownership.md.
#
# The backend's fallback half (a standard client's `Start` answering in mode
# `stills` with `df_fallback = pipewire-producer-unavailable`) is covered by
# `cargo test -p dragonfruit-portal --test screencast` on every run.
#
# Outputs:
#   docs/captures/t335b-screencast-client.txt   verdict and rerun command
#
# Requires: `xdg-desktop-portal` (the frontend), the built tree, a session bus,
# `gdbus` and `gst-launch-1.0` with `pipewiresrc`. Not part of `make e2e`.
set -u

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

OUTDIR="${OUTDIR:-docs/captures}"
mkdir -p "$OUTDIR"
VERDICT="$OUTDIR/t335b-screencast-client.txt"

record() {
    local verdict="$1"
    local reason="$2"
    {
        echo "T335b standard-client ScreenCast check"
        echo "timestamp: $(date -Is)"
        echo "verdict: $verdict"
        echo "reason: $reason"
        echo "frontend: /usr/libexec/xdg-desktop-portal"
        echo "client: gst-launch-1.0 pipewiresrc path=<node> ! fakesink"
        echo "rerun: make t335b-screencast-client"
    } >"$VERDICT"
    echo "t335b-screencast-client: $verdict — $reason"
}

missing=""
command -v dbus-daemon >/dev/null 2>&1 || missing="dbus-daemon"
[ -x /usr/libexec/xdg-desktop-portal ] || missing="xdg-desktop-portal"
command -v gst-launch-1.0 >/dev/null 2>&1 || missing="gst-launch-1.0"
command -v gdbus >/dev/null 2>&1 || missing="gdbus"
[ -x target/debug/xdg-desktop-portal-dragonfruit ] \
    || [ -x target/release/xdg-desktop-portal-dragonfruit ] || missing="a built portal backend"
if [ -n "$missing" ]; then
    record OPEN "missing $missing"
    exit 0
fi

record OPEN "the shell bridge that reaches the compositor producer (df_toplevel_manager.create_screencast, v17) is not wired; the portal answers the named stills fallback (ADR 0345). Wire it, then rerun this check with a live nested/Flatpak session and expect several frames from pipewiresrc."
exit 0
