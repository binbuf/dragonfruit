#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# T314 scripted watchdog check.
#
# Runs the shell watchdog test binary in probe mode as a transient user
# service with the shipped unit's notify/watchdog properties. The probe
# sends READY=1, heartbeats from the GUI thread, then blocks that thread on
# its first run. systemd's watchdog must kill it and Restart=on-failure must
# start run 2, which exits cleanly.
#
# This is deliberately not part of `make gates`: it needs a live systemd user
# manager. Run it by hand after building:
#
#   cmake --build build --target tst_shellwatchdog
#   scripts/t314-watchdog-check.sh
#
# Pass a different probe binary as the first argument if needed.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
probe=${1:-"$root/build/shell/tests/tst_shellwatchdog"}

if [ ! -x "$probe" ]; then
    echo "t314: probe not found: $probe" >&2
    echo "t314: build it with: cmake --build build --target tst_shellwatchdog" >&2
    exit 2
fi
if ! command -v systemd-run >/dev/null 2>&1 \
    || ! systemd-run --user --wait --collect /bin/true >/dev/null 2>&1; then
    echo "t314: a live systemd user manager is required for this check" >&2
    exit 2
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/t314-watchdog.XXXXXX")
unit="t314-watchdog-check-$$"
cleanup() {
    systemctl --user stop "$unit" >/dev/null 2>&1 || true
    rm -rf "$tmp"
}
trap cleanup EXIT

systemd-run --user --unit="$unit" \
    --property=Type=notify \
    --property=WatchdogSec=5 \
    --property=NotifyAccess=main \
    --property=Restart=on-failure \
    --property=RestartSec=1 \
    --setenv=T314_WATCHDOG_PROBE=1 \
    --setenv=T314_PROBE_COUNTER="$tmp/count" \
    "$probe"

restarts=0
runs=0
for _ in $(seq 1 60); do
    restarts=$(systemctl --user show "$unit" -p NRestarts --value 2>/dev/null || echo 0)
    runs=$(cat "$tmp/count" 2>/dev/null || echo 0)
    if [ "${restarts:-0}" -ge 1 ] && [ "${runs:-0}" -ge 2 ]; then
        break
    fi
    sleep 0.5
done

echo "t314: NRestarts=$restarts probe-runs=$runs"
systemctl --user show "$unit" -p ActiveState -p Result -p ExecMainStatus \
    --value 2>/dev/null || true

if [ "${restarts:-0}" -lt 1 ] || [ "${runs:-0}" -lt 2 ]; then
    echo "t314: FAIL: systemd did not kill and restart the blocked GUI thread" >&2
    exit 1
fi

echo "t314: PASS: the blocked GUI thread was watchdog-killed and restarted"
