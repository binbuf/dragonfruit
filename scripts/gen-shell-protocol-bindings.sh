#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Generate Qt/C++ bindings for the private shell protocols (T-07) from the
# same XMLs the Rust compositor and conformance client use. The shell (T-08)
# compiles these; this script is a no-op when the Qt/wayland scanner is not
# installed, so it is safe to run anywhere.
#
# Usage:
#   scripts/gen-shell-protocol-bindings.sh [output-dir]
#
# Preferred tool: Qt's `qtwaylandscanner`, which emits Qt-style client
# wrappers (`-protocol.h` + `.cpp`). Fallback: `wayland-scanner
# client-header` for the plain C bindings.

set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
protocols_dir="$repo_root/protocols"
out_dir="${1:-$repo_root/build/protocols}"

xmls=(
    dragonfruit-core.xml
    dragonfruit-shell.xml
    dragonfruit-toplevel.xml
)

find_tool() {
    command -v "$1" 2>/dev/null || true
}

qt_scanner=$(find_tool qtwaylandscanner)
wl_scanner=$(find_tool wayland-scanner)
if [ -z "$qt_scanner" ] && [ -z "$wl_scanner" ] && [ -x "$HOME/.local/df-toolchain/usr/bin/wayland-scanner" ]; then
    wl_scanner="$HOME/.local/df-toolchain/usr/bin/wayland-scanner"
fi

if [ -z "$qt_scanner" ] && [ -z "$wl_scanner" ]; then
    echo "gen-shell-protocol-bindings: no wayland scanner found; skipping (T-08 will need one)" >&2
    exit 0
fi

mkdir -p "$out_dir"

for xml in "${xmls[@]}"; do
    name="${xml%.xml}"
    if [ -n "$qt_scanner" ]; then
        # Qt Wayland scanner: client-side wrappers for QML/C++.
        "$qt_scanner" client-header "$protocols_dir/$xml" "$out_dir/${name}-client-protocol.h"
        "$qt_scanner" private-code "$protocols_dir/$xml" "$out_dir/${name}-protocol.c"
        echo "generated Qt bindings for $xml"
    else
        # Plain C client bindings (usable from any C/C++/Qt project).
        "$wl_scanner" client-header "$protocols_dir/$xml" "$out_dir/${name}-client-protocol.h"
        "$wl_scanner" private-code "$protocols_dir/$xml" "$out_dir/${name}-protocol.c"
        echo "generated C bindings for $xml"
    fi
done
