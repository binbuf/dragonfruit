#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Desktop-name gate (T-01, FR-3): fails when any source or build file
# hardcodes a desktop name. `dragonfruit` is the only legal name
# (XDG_CURRENT_DESKTOP is a public contract — docs/ipc-versioning.md).
#
# Design docs (.docs/) are exempt: they legitimately discuss other
# desktops. Code, build files, and scripts are not. A line carrying the
# marker `df-allow-desktop-name` is exempt (negative tests, etc.).
set -uo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

status=0

scan_files() {
    find . \
        \( -path ./.git -o -path ./target -o -path ./build -o -path ./.docs \
           -o -path ./LICENSES \) -prune \
        -o -type f \( -name '*.rs' -o -name '*.cpp' -o -name '*.qml' \
           -o -name '*.h' -o -name '*.sh' -o -name '*.xml' -o -name '*.yml' \
           -o -name '*.yaml' -o -name '*.service' -o -name '*.conf' \
           -o -name '*.desktop' -o -name 'CMakeLists.txt' -o -name 'Makefile' \
           -o -name '*.toml' \) -print0
}

# 1. Known desktop names must not appear in code or build files.
names='gnome|kde|plasma|cinnamon|xfce|lxde|lxqt|budgie|sway|weston|hyprland|cosmic'
while IFS= read -r -d '' f; do
    [ "$f" = "./scripts/check-desktop-names.sh" ] && continue
    hits=$(grep -nwiE "$names" "$f" 2>/dev/null | grep -v 'df-allow-desktop-name' || true)
    if [ -n "$hits" ]; then
        echo "ERROR: hardcoded desktop name in $f:"
        echo "$hits" | head -5
        status=1
    fi
done < <(scan_files)

# 2. Any XDG_CURRENT_DESKTOP assignment in code must set `dragonfruit`.
while IFS= read -r -d '' f; do
    [ "$f" = "./scripts/check-desktop-names.sh" ] && continue
    hits=$(grep -nE 'XDG_CURRENT_DESKTOP[[:space:]]*=' "$f" 2>/dev/null \
        | grep -vE 'XDG_CURRENT_DESKTOP[[:space:]]*=[[:space:]]*("dragonfruit"|\{DESKTOP_NAME\}|dragonfruit)' \
        | grep -v 'df-allow-desktop-name' || true)
    if [ -n "$hits" ]; then
        echo "ERROR: $f assigns XDG_CURRENT_DESKTOP to something other than 'dragonfruit':"
        echo "$hits" | head -5
        status=1
    fi
done < <(scan_files)

if [ "$status" -eq 0 ]; then
    echo "check-desktop-names: OK — only 'dragonfruit' appears in code"
fi
exit "$status"
