#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-15.16 absent-daemon masking matrix (headless reproduction).
#
# Writes docs/captures/t15-absence-matrix.txt: for every shipped Wave-2/3 pane
# it records the exact reproduction (a ctest / cargo invocation) and its
# output. The suite runs under a private bus with no settingsd, no
# xdg-desktop-portal, and no system-status bridge host, so absence is the state
# under test; no host session, no VM, and no daemons are needed.
#
# The reviewed matrix with the live/VM halves is
# docs/captures/t15-absence-matrix.md; the design contract is
# "The absent-daemon masking matrix (T-15.16)" in docs/design/08-settings.md.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t15-absence-matrix.txt}"
BUILD="${BUILD_DIR:-build}"

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*"; }
command_line() { printf '$ %s\n' "$*"; }

{
    echo "T-15.16 absent-daemon masking matrix (headless reproduction)"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "build:     $BUILD"
    echo "state:     private dbus-run-session, no settingsd, no portal, no bridge host"

    section "0. The shipped pane set (the matrix's rows)"
    echo "Every shipped pane must mount and stay interactive with both providers"
    echo "absent (the routing sweep), and every pane with an adapter-read half"
    echo "must expose its absence note. The catalog is the source of truth."
    command_line "grep -c 'shipped: true' apps/settings/SettingsPanes.qml"
    grep -c 'shipped: true' apps/settings/SettingsPanes.qml
    command_line "awk '/\\{ id: \"/{id=\$0} /shipped: true/{print id}' apps/settings/SettingsPanes.qml (ids)"
    awk '/\{ id: "/{id=$0} /shipped: true/{print id}' apps/settings/SettingsPanes.qml \
        | sed 's/^[[:space:]]*/  /'

    section "1. The Settings absent-provider gate (QML, private bus)"
    echo "No settingsd and no portal: both providers are absent, every shipped"
    echo "pane has a body and none of the unshipped ids do, every shipped pane"
    echo "mounts and stays interactive, and each pane's absent case holds."
    if [ -f "$BUILD/CTestTestfile.cmake" ]; then
        command_line "ctest --test-dir $BUILD -R 'tst_settings_absence' --output-on-failure"
        ctest --test-dir "$BUILD" -R 'tst_settings_absence' --output-on-failure
    else
        echo "(build/ not configured; run 'make build', then this row)"
    fi

    section "2. The per-pane suites (adapter absent values via fixtures)"
    echo "Each pane's own suite asserts the adapter absent value and the live"
    echo "settingsd-backed/host-fallback controls through the fixture seam."
    if [ -f "$BUILD/CTestTestfile.cmake" ]; then
        command_line "ctest --test-dir $BUILD -R '^tst_settings_' --output-on-failure"
        ctest --test-dir "$BUILD" -R '^tst_settings_' --output-on-failure
    else
        echo "(build/ not configured; run 'make build', then this row)"
    fi

    section "3. The bridge host's adapter absence matrix (Rust)"
    echo "Every Wave-2/3 adapter read through the system-status bridge host:"
    echo "the masked daemon projects a hidden slot (\`present: false\`), never an"
    echo "error, and a write while absent answers absent. This is the headless"
    echo "half of the masking matrix (T-07.6a grown for T-15)."
    command_line "cargo test -p dragonfruit-system-status absent -- --nocapture"
    cargo test -p dragonfruit-system-status absent -- --nocapture

    section "4. The generic adapter subscription (Rust)"
    echo "The shared adapter seam: an absent daemon at startup is hidden and"
    echo "never errors; a new subscription is silent."
    command_line "cargo test -p dragonfruit-system-adapters --test subscription absent -- --nocapture"
    cargo test -p dragonfruit-system-adapters --test subscription absent -- --nocapture

    section "5. The per-adapter sources (Rust, one representative per subsystem)"
    echo "Each host adapter's own mock asserts an absent read is absent and a"
    echo "write while absent is a no-op. The set below covers every Wave-2/3"
    echo "subsystem: BlueZ, UDisks, PipeWire, libinput, the compositor overview,"
    echo "UPower+ppd, notify, session lock, the menu bar, updates, accounts,"
    echo "CUPS/SANE, the portal PermissionStore, AT-SPI, and NetworkManager VPN."
    for crate in dragonfruit-bluetooth dragonfruit-storage dragonfruit-audio \
                 dragonfruit-input dragonfruit-overview dragonfruit-power \
                 dragonfruit-notify-adapter dragonfruit-lock-adapter \
                 dragonfruit-menubar-adapter dragonfruit-update-adapter \
                 dragonfruit-account-adapter dragonfruit-printer-adapter \
                 dragonfruit-privacy-adapter dragonfruit-accessibility-adapter \
                 dragonfruit-networkmanager; do
        command_line "cargo test -p $crate absent -- --nocapture"
        cargo test -p "$crate" absent -- --nocapture
    done

    section "6. Live/VM half (not automated here)"
    echo "Actually stopping BlueZ/UDisks/PipeWire/CUPS/NetworkManager in a VM and"
    echo "watching the pane and its Control Center tile is the design's real-VM"
    echo "check (14-risks.md). No VM is available in CI, so the reviewed state"
    echo "matrix in docs/captures/t15-absence-matrix.md records it by hand."
    echo "On a live session, 'no bridge host' is the same hidden state because"
    echo "the host is a separate process; the nested breadth capture confirms"
    echo "the desktop and Settings render with no host at all."
} >"$OUT" 2>&1

echo "t15-absence-matrix: wrote $OUT"