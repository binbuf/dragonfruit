#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Desktop-name gate (T-01, FR-3) and user-facing naming gate (T220): fails
# when any source or build file hardcodes another desktop name, and when a
# user-facing string, desktop entry, or packaging manifest uses a forbidden
# Apple term from docs/design/naming-gate.json.
#
# `dragonfruit` is the only legal desktop name (XDG_CURRENT_DESKTOP is a
# public contract — docs/ipc-versioning.md). Design docs (docs/) are exempt:
# they legitimately discuss other desktops and the reference platform. Code,
# build files, and scripts are not. A line carrying the marker
# `df-allow-desktop-name` (or `df-allow-name` for the naming half) is exempt
# (negative tests, etc.).
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

# 1. Known desktop names must not appear in code or build files. A
# `org.<name>.<member>` token is a third-party D-Bus well-known name or
# application id (`org.kde.StatusNotifierItem`, `org.gnome.Calculator.desktop`),
# which is a compatibility contract, not an XDG_CURRENT_DESKTOP hardcode, so
# strip those namespaces before matching. The remaining match is a
# desktop-name literal only (a bare `"KDE"`, an `XDG_CURRENT_DESKTOP=GNOME`,
# or a comment naming another desktop), which is what the gate is for.
names='gnome|kde|plasma|cinnamon|xfce|lxde|lxqt|budgie|sway|weston|hyprland|cosmic'
strip_third_party="s#org\\.($names)\\.[A-Za-z0-9_.-]+##g"
while IFS= read -r -d '' f; do
    [ "$f" = "./scripts/check-desktop-names.sh" ] && continue
    hits=$(sed -E "$strip_third_party" "$f" 2>/dev/null \
        | grep -nwiE "$names" | grep -v 'df-allow-desktop-name' || true)
    if [ -n "$hits" ]; then
        echo "ERROR: hardcoded desktop name in $f:"
        echo "$hits" | head -5
        status=1
    fi
done < <(scan_files)

# 2. Any literal XDG_CURRENT_DESKTOP assignment in code must set
# `dragonfruit`. A read (`${XDG_CURRENT_DESKTOP:-...}`) is not an assignment
# and must not trip the gate; that was the long-standing false positive in
# the diagnostics scripts.
while IFS= read -r -d '' f; do
    [ "$f" = "./scripts/check-desktop-names.sh" ] && continue
    hits=$(grep -nE '(^|[[:space:]])(export[[:space:]]+)?XDG_CURRENT_DESKTOP[[:space:]]*=' "$f" 2>/dev/null \
        | grep -vE 'XDG_CURRENT_DESKTOP[[:space:]]*=[[:space:]]*(\$|\{|"dragonfruit"|dragonfruit)' \
        | grep -v 'df-allow-desktop-name' || true)
    if [ -n "$hits" ]; then
        echo "ERROR: $f assigns XDG_CURRENT_DESKTOP to something other than 'dragonfruit':"
        echo "$hits" | head -5
        status=1
    fi
done < <(scan_files)

# 3. User-facing strings, desktop entries, and packaging metadata must use the
# generic names in docs/design/interface-naming-and-licensing.md. The Python
# helper owns the QML/C++ string extraction and the machine-readable map; the
# shell gate stays the single entry point used by lint and the release gate.
if ! python3 scripts/check-user-facing-names.py --root "$repo_root"; then
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo "check-desktop-names: OK — only 'dragonfruit' appears in code; user-facing names are generic"
fi
exit "$status"
