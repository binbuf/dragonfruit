#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-08 FR-1/FR-2 gate: design-system QML components must express color,
# spacing, and animation values through tokens, never as literals. The
# generated Theme.qml is the only file allowed to contain raw values. The
# app-level "no hand-rolled titlebar/menu/settings-row" gate lands with
# T-16/T-18, which build on these components.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
status=0

check_file() {
    local file="$1"
    local rel="${file#"$root"/}"

    if grep -nE '#[0-9a-fA-F]{3,8}\b' "$file"; then
        echo "check-design-tokens: hardcoded color in $rel" >&2
        status=1
    fi
    if grep -nE '\bduration[[:space:]]*:[[:space:]]*[0-9]' "$file"; then
        echo "check-design-tokens: hardcoded duration in $rel" >&2
        status=1
    fi
    if grep -nE '\bradius[[:space:]]*:[[:space:]]*[0-9]' "$file"; then
        echo "check-design-tokens: hardcoded radius in $rel" >&2
        status=1
    fi
}

for file in "$root"/design-system/components/*.qml "$root"/design-system/gallery/*.qml; do
    [ -e "$file" ] || continue
    check_file "$file"
done

if [ "$status" -ne 0 ]; then
    echo "check-design-tokens: every value must come from design-system/tokens/tokens.json" >&2
    exit 1
fi

echo "check-design-tokens: ok"
