#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# T334 live visual check: the Files Open With picker in the nested session and
# the global menu while Files is focused. The broker's ResolveFocused reply is
# recorded beside the stills so the Files menu, accelerators, and the live
# Open With submenu are inspectable as data as well as pixels.
#
# Needs a host Wayland session, `spectacle`, `gdbus`, `python3` with Pillow,
# and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out_dir=${OUTDIR:-"$repo/docs/captures"}
settle=${SETTLE:-12}
scratch=${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t334-live.XXXXXX")}
socket=${SOCKET:-dragonfruit-t334-live}

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-t334-live: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle gdbus python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-t334-live: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-t334-live: python3 Pillow not found" >&2
    exit 1
}
[ -x "$repo/build/apps/files/dragonfruit-files" ] || {
    echo "capture-t334-live: build/apps/files/dragonfruit-files not built — run make build" >&2
    exit 1
}

mkdir -p "$out_dir" "$scratch/fixture/Documents" \
    "$scratch/xdg-data" "$scratch/xdg-config" "$scratch/xdg-cache"
printf 'alpha\n' >"$scratch/fixture/Documents/alpha.txt"
printf 'beta\n' >"$scratch/fixture/Documents/beta.txt"

synth="${XDG_RUNTIME_DIR}/${socket}.synth"
rm -f "$synth"

demo_pgid=""
cleanup() {
    if [ -n "$demo_pgid" ]; then
        kill -TERM -"$demo_pgid" 2>/dev/null || true
        sleep 1
        kill -KILL -"$demo_pgid" 2>/dev/null || true
    fi
    rm -f "$synth"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$scratch"
}
trap cleanup EXIT

cd "$repo"
env XDG_CONFIG_HOME="$scratch/xdg-config" \
    XDG_CACHE_HOME="$scratch/xdg-cache" \
    XDG_DATA_HOME="$scratch/xdg-data" \
    DF_DEMO_QT_APP=build/apps/files/dragonfruit-files \
    DF_FILES_START_URI="$scratch/fixture/Documents" \
    DF_FILES_START_MENU=openWith \
    DF_FILES_OPEN_WITH_SELECT=1 \
    DRAGONFRUIT_SYNTHETIC_INPUT="$synth" \
    setsid make demo DEMO_ARGS="--socket-name $socket" \
    >"$scratch/demo.log" 2>&1 &
demo_pgid=$!

for _ in $(seq 1 600); do [ -e "${XDG_RUNTIME_DIR}/${socket}" ] && break; sleep 0.1; done
for _ in $(seq 1 600); do [ -e "$synth" ] && break; sleep 0.1; done
if [ ! -e "$synth" ]; then
    echo "capture-t334-live: synthetic socket never appeared; log:" >&2
    tail -40 "$scratch/demo.log" >&2
    exit 1
fi
sleep "$settle"

gdbus call --session --dest org.dragonfruit.MenuBroker1 \
    --object-path /org/dragonfruit/MenuBroker1 \
    --method org.dragonfruit.MenuBroker1.ResolveFocused \
    >"$out_dir/t334-live-global-menu.txt" 2>"$scratch/resolve.err" || {
        echo "capture-t334-live: ResolveFocused failed:" >&2
        cat "$scratch/resolve.err" >&2
        exit 1
    }

python3 "$repo/scripts/capture-files-driver.py" --synth "$synth" \
    --dest "$out_dir/t334-live-open-with.png" --scratch "$scratch"
python3 "$repo/scripts/capture-files-driver.py" --synth "$synth" \
    --dest "$out_dir/t334-live-global-menu.png" --scratch "$scratch" \
    --keep-pointer --no-trim

cat >"$out_dir/t334-live.txt" <<EOF
T334 Files Open With and global-menu live check
date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
source: nested session (make demo), Files on a temporary Documents fixture
window: nested output at 1x; the global-menu still keeps the host window chrome
scenarios:
  open with:   right-click-equivalent File ▸ Open With opens the picker for
               alpha.txt; the installed handlers are listed and the second
               handler is preselected (DF_FILES_OPEN_WITH_SELECT=1)
  global menu: Files is focused; the bar shows the Files application menu and
               File/Edit/View/Go/Window/Help resolved from the native tier
broker:
  org.dragonfruit.MenuBroker1.ResolveFocused is recorded verbatim in
  docs/captures/t334-live-global-menu.txt (appId org.dragonfruit.Files.desktop,
  tier native, accelerators, the live Open With submenu).
artifacts:
  docs/captures/t334-live-open-with.png
  docs/captures/t334-live-global-menu.png
  docs/captures/t334-live-global-menu.txt
EOF

echo "wrote $out_dir/t334-live-{open-with,global-menu}.png and t334-live-global-menu.txt"
