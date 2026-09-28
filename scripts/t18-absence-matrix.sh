#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-18.3 absence/state matrix (headless reproduction).
#
# Writes docs/captures/t18-absence-matrix.txt: for each row of the track's
# absence matrix it records the exact reproduction (a CLI invocation or the
# named test) and its output. Offline is simulated with a dead proxy, so the
# script needs no network and no host session. The bus/settingsd/portal rows
# are the QML and Rust suites `make e2e` runs; the live network/gallery halves
# are recorded by hand in docs/captures/t18-absence-matrix.md.
set -uo pipefail

cd "$(dirname "$0")/.."

OUT="${1:-docs/captures/t18-absence-matrix.txt}"
BIN="target/debug/dragonfruit-wallpaperd"
DEAD_PROXY="http://127.0.0.1:1"

if [ ! -x "$BIN" ]; then
    echo "t18-absence-matrix: $BIN not built; run 'make cargo-build' first" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t18-matrix.XXXXXX")"
trap 'rm -rf "$SCRATCH"' EXIT

section() { printf '\n================================================================\n## %s\n================================================================\n\n' "$*"; }
command_line() { printf '$ %s\n' "$*"; }

# offline <cache> <extra args...>: run the provider with a dead proxy so every
# network call fails immediately (the offline state), and an empty cache.
offline() {
    local cache="$1"; shift
    env XDG_CACHE_HOME="$cache" HOME="$SCRATCH" \
        HTTPS_PROXY="$DEAD_PROXY" HTTP_PROXY="$DEAD_PROXY" ALL_PROXY="$DEAD_PROXY" \
        https_proxy="$DEAD_PROXY" http_proxy="$DEAD_PROXY" all_proxy="$DEAD_PROXY" \
        "$BIN" "$@"
}

{
    echo "T-18.3 wallpaper absence/state matrix (headless reproduction)"
    echo "generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "provider:  $BIN"
    echo "offline proxy: $DEAD_PROXY (accepts nothing)"

    section "1. Lazy launch (cold cache, no display of the pane)"
    echo "The service rebuilds/loads an empty cache and must not fetch until a"
    echo "warm or Preload asks. Status stays idle and the shipped default"
    echo "resolves with no network call."
    command_line "XDG_CACHE_HOME=<empty> $BIN --print-builtin"
    XDG_CACHE_HOME="$SCRATCH/cold" HOME="$SCRATCH" "$BIN" --print-builtin
    command_line "XDG_CACHE_HOME=<empty> $BIN --print-status"
    XDG_CACHE_HOME="$SCRATCH/cold" HOME="$SCRATCH" "$BIN" --print-status

    section "2. Cold cache + no network (Preload)"
    echo "An empty cache and a dead network: the eager refresh fails, the"
    echo "service reports offline, no items are served, and the shipped"
    echo "default still resolves. Nothing blocks."
    command_line "HTTPS_PROXY=$DEAD_PROXY XDG_CACHE_HOME=<empty> $BIN --preload"
    offline "$SCRATCH/cold-offline" --preload
    echo
    echo "cache after the failed fetch (must hold no fetched image):"
    find "$SCRATCH/cold-offline" -type f -printf '  %P\n' 2>/dev/null || echo "  (empty)"
    echo
    echo "the shipped default is read in place, never copied into the cache:"
    ls -l "$(XDG_CACHE_HOME="$SCRATCH/cold-offline" HOME="$SCRATCH" "$BIN" --print-builtin)"

    section "3. Warm cache + no network (headless: the cache survives)"
    echo "A populated cache plus a dead network keeps every item; a failed"
    echo "fetch is not recorded as a fetch."
    command_line "cargo test -p dragonfruit-wallpaperd --test read_path an_offline_refresh_keeps_the_cache -- --nocapture"
    cargo test -p dragonfruit-wallpaperd --test read_path \
        an_offline_refresh_keeps_the_cache -- --nocapture

    section "4. Provider restart (headless: rebuild from the cache)"
    echo "A fresh provider over the same cache rebuilds the catalogue without"
    echo "touching the network."
    command_line "cargo test -p dragonfruit-wallpaperd --test read_path a_cold_start_rebuilds_the_catalogue_from_the_cache_without_network -- --nocapture"
    cargo test -p dragonfruit-wallpaperd --test read_path \
        a_cold_start_rebuilds_the_catalogue_from_the_cache_without_network -- --nocapture

    section "5. Weekly re-check (headless)"
    echo "Within the week no fetch; once the cache is stale a re-check runs."
    command_line "cargo test -p dragonfruit-wallpaperd --test read_path a_fresh_cache_is_not_refetched -- --nocapture"
    cargo test -p dragonfruit-wallpaperd --test read_path \
        a_fresh_cache_is_not_refetched -- --nocapture

    section "6. API error / rate limit (headless: failure keeps the cache)"
    echo "Any non-2xx or transport error is a category failure; all categories"
    echo "failing yields an offline service that keeps the old cache. A rate"
    echo "limit (HTTP 429) takes the same path as any other HTTP error."
    command_line "cargo test -p dragonfruit-wallpaperd an_empty_fetch_result_keeps_the_cache_and_goes_offline -- --nocapture"
    cargo test -p dragonfruit-wallpaperd \
        an_empty_fetch_result_keeps_the_cache_and_goes_offline -- --nocapture
    command_line "cargo test -p dragonfruit-wallpaperd the_service_state_warm_is_offline_safe -- --nocapture"
    cargo test -p dragonfruit-wallpaperd \
        the_service_state_warm_is_offline_safe -- --nocapture

    section "7. Lazy launch vs open-pane Preload (headless)"
    echo "Loading the provider touches neither network nor disk; Preload does"
    echo "exactly one refresh, fills the cache, and selects the Featured default."
    command_line "cargo test -p dragonfruit-wallpaperd --test read_path preload_fetches_fills_the_cache_and_selects_the_featured_default -- --nocapture"
    cargo test -p dragonfruit-wallpaperd --test read_path \
        preload_fetches_fills_the_cache_and_selects_the_featured_default -- --nocapture

    section "8. Session-bus round-trip and absent network (headless)"
    echo "The interface answers offline and serves the shipped default; a"
    echo "second Preload is a no-op."
    command_line "cargo test -p dragonfruit-wallpaperd --test session_bus -- --nocapture"
    cargo test -p dragonfruit-wallpaperd --test session_bus -- --nocapture

    section "9. settingsd restart and portal absent (QML shell, headless)"
    echo "The Settings shell tolerates an absent settingsd and an absent"
    echo "FileChooser portal without surfacing an error; the shipped default"
    echo "and built-in rows stay live while only the Custom (portal) row is"
    echo "disabled."
    if [ -x build/apps/settings/tst_settings_absence ] || [ -f build/CTestTestfile.cmake ]; then
        command_line "ctest --test-dir build -R 'tst_settings_absence|tst_wallpaperpolicy|tst_settings_wallpaper' --output-on-failure"
        ctest --test-dir build -R 'tst_settings_absence|tst_wallpaperpolicy|tst_settings_wallpaper' --output-on-failure
    else
        echo "(build/ not configured; run 'make build', then this row)"
    fi

    section "10. Package artifact holds only the shipped original (no fetched bytes)"
    echo "The install seam copies Default.jpg to share/dragonfruit/wallpapers/"
    echo "and ships no fetched image."
    DEST="$SCRATCH/package"
    command_line "make install DESTDIR=$DEST PREFIX=/usr"
    ( make install DESTDIR="$DEST" PREFIX=/usr >/dev/null 2>&1 ) || echo "(run 'make install' manually)"
    echo "installed images under the prefix:"
    find "$DEST" -type f \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' -o -iname '*.webp' \) -printf '  %p\n' 2>/dev/null || true
    installed="$DEST/usr/share/dragonfruit/wallpapers/Default.jpg"
    if [ -f "$installed" ]; then
        echo "  Default.jpg byte-identical to the in-tree asset:"
        if cmp -s "$installed" assets/graphics/wallpapers/Default.jpg; then
            echo "    yes"
        else
            echo "    NO — the install drifted"
        fi
    else
        echo "  (Default.jpg not installed in this environment)"
    fi
} >"$OUT" 2>&1

echo "t18-absence-matrix: wrote $OUT"