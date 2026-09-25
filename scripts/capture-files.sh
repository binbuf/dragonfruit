#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# T-10.7 Files slice capture (track boundary).
#
# Produces the T-10 artifacts in docs/captures/:
#
#   * t10-files.png                     representative still (Documents, list)
#   * t10-files-list.png                list view
#   * t10-files-icon.png                icon view
#   * t10-files-context-menu.png        the item context menu
#   * t10-files-multiselect.png         a three-item selection
#   * t10-files-rename.png              inline rename
#   * t10-files-trash-empty.png         the Empty Trash confirmation over a
#                                       non-empty trash:// listing
#   * t10-files-dock-trash.png          the Dock band with the Trash badge
#   * t10-files-reveal.png              a file argument revealed in its parent
#                                       (the Dock's Show in Files path)
#   * t10-files-large-directory.png     the 100k synthetic listing, top
#   * t10-files-large-directory-scrolled.png  after a scroll sweep
#   * t10-files.mp4                     the stills in sequence
#
# and the large-directory scroll trace (raw numbers) at
# docs/captures/t10-files-scroll-trace.txt.
#
# The app is launched through the normal `make demo` nested session with
# DF_DEMO_QT_APP pointing at Files; the stills that need pointer state the
# nested synthetic harness cannot hold (a modifier or a right-click on a Qt
# surface) use the app's capture seams (DF_FILES_START_MENU / _SELECT /
# _RENAME / _EMPTY_TRASH). A scratch fixture and XDG tree keep the capture
# independent of the host's files and trash (HOME is left alone so the build
# still finds the toolchain and cargo cache).
#
# Requires: a host Wayland session, `spectacle`, `ffmpeg`, `gdbus`, python3
# with Pillow, and the built tree (`make build`). Not part of `make e2e`.
set -euo pipefail

cd "$(dirname "$0")/.."

OUTDIR="${OUTDIR:-docs/captures}"
SETTLE="${SETTLE:-12}"
SCRATCH="${SCRATCH:-$(mktemp -d "${TMPDIR:-/tmp}/dragonfruit-t10-files.XXXXXX")}"
SOCKET="${SOCKET:-dragonfruit-t10-files}"

if [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "capture-files: XDG_RUNTIME_DIR unset — the capture needs a host session" >&2
    exit 1
fi
for tool in spectacle ffmpeg python3 gdbus; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "capture-files: $tool not found — the capture needs a host session" >&2
        exit 1
    }
done
python3 -c 'import PIL' 2>/dev/null || {
    echo "capture-files: python3 Pillow not found" >&2
    exit 1
}
[ -x build/apps/files/dragonfruit-files ] || {
    echo "capture-files: build/apps/files/dragonfruit-files not built — run make build" >&2
    exit 1
}

mkdir -p "$OUTDIR"
SYNTH="${XDG_RUNTIME_DIR}/$SOCKET.synth"
rm -f "$SYNTH"

# --- fixture tree + a real trash store. ------------------------------------
# HOME is deliberately left alone: `make demo` compiles the workspace and
# cargo, pkg-config and the project's toolchain discovery all key off it.
FIXTURE="$SCRATCH/fixture/Documents"
XDG_DATA="$SCRATCH/xdg-data"
mkdir -p "$FIXTURE"/{Projects,Invoices}
printf 'Shopping\nMilk\nEggs\n'            >"$FIXTURE/notes.txt"
printf '%%PDF-1.4 report\n'               >"$FIXTURE/report.pdf"
printf 'budget\n'                         >"$FIXTURE/budget.numbers"
printf 'image\n'                          >"$FIXTURE/photo.jpg"
printf 'archive\n'                        >"$FIXTURE/archive.tar.gz"
printf '# Read me\n'                      >"$FIXTURE/README.md"
printf 'plan\n'                           >"$FIXTURE/Projects/plan.md"
printf 'jan\n'                            >"$FIXTURE/Invoices/january.txt"
mkdir -p "$XDG_DATA/Trash/files" "$XDG_DATA/Trash/info"
printf 'deleted report\n' >"$XDG_DATA/Trash/files/report.pdf"
printf 'deleted notes\n'  >"$XDG_DATA/Trash/files/notes.txt"
cat >"$XDG_DATA/Trash/info/report.pdf.trashinfo" <<EOF
[Trash Info]
Path=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$FIXTURE/report.pdf")
DeletionDate=2026-09-25T11:00:00
EOF
cat >"$XDG_DATA/Trash/info/notes.txt.trashinfo" <<EOF
[Trash Info]
Path=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' "$FIXTURE/notes.txt")
DeletionDate=2026-09-25T11:00:00
EOF

BASE_ENV=(
    "XDG_CONFIG_HOME=$SCRATCH/config"
    "XDG_CACHE_HOME=$SCRATCH/cache"
    "XDG_DATA_HOME=$XDG_DATA"
    "DF_DEMO_QT_APP=build/apps/files/dragonfruit-files"
)

DEMO_PGID=""
cleanup() {
    stop_demo
    rm -f "$SYNTH"
    [ "${KEEP_SCRATCH:-0}" = "1" ] || rm -rf "$SCRATCH"
}
trap cleanup EXIT

stop_demo() {
    if [ -n "$DEMO_PGID" ]; then
        kill -TERM -"$DEMO_PGID" 2>/dev/null || true
        sleep 1
        kill -KILL -"$DEMO_PGID" 2>/dev/null || true
        DEMO_PGID=""
    fi
    for _ in $(seq 1 100); do [ ! -e "$SYNTH" ] && break; sleep 0.05; done
}

# run_scene <name> [--scroll] [--dock-crop] [VAR=VALUE ...]
# ONLY=comma,separated,names re-runs a subset (useful when one still needs a
# retake).
run_scene() {
    local name="$1"; shift
    local scroll=0 dock=0
    local extra=()
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --scroll) scroll=1 ;;
            --dock-crop) dock=1 ;;
            *) extra+=("$1") ;;
        esac
        shift
    done
    if [ -n "${ONLY:-}" ] && [[ ",$ONLY," != *",$name,"* ]]; then
        return 0
    fi
    rm -f "$SYNTH"
    env "${BASE_ENV[@]}" "${extra[@]}" DRAGONFRUIT_SYNTHETIC_INPUT="$SYNTH" \
        setsid make demo DEMO_ARGS="--socket-name $SOCKET" \
        >"$SCRATCH/demo-$name.log" 2>&1 &
    DEMO_PGID=$!
    for _ in $(seq 1 600); do [ -e "$XDG_RUNTIME_DIR/$SOCKET" ] && break; sleep 0.1; done
    for _ in $(seq 1 600); do [ -e "$SYNTH" ] && break; sleep 0.1; done
    if [ ! -e "$SYNTH" ]; then
        echo "capture-files: synthetic socket never appeared for $name; log:" >&2
        tail -40 "$SCRATCH/demo-$name.log" >&2
        exit 1
    fi
    sleep "$SETTLE"
    local args=(--synth "$SYNTH" --dest "$OUTDIR/t10-files-$name.png"
                --scratch "$SCRATCH")
    [ "$scroll" = 1 ] && args+=(--scroll)
    [ "$dock" = 1 ] && args+=(--dock-crop "$OUTDIR/t10-files-dock-trash.png")
    python3 scripts/capture-files-driver.py "${args[@]}" || {
        echo "capture-files: still failed for $name; log:" >&2
        tail -20 "$SCRATCH/demo-$name.log" >&2
        exit 1
    }
    stop_demo
}

# --- 1. list and icon views, plus the selection/menu/rename seams. ---------
run_scene list    DF_FILES_START_URI="$FIXTURE" DF_FILES_START_VIEW=list
run_scene icon    DF_FILES_START_URI="$FIXTURE" DF_FILES_START_VIEW=icon
run_scene context-menu DF_FILES_START_URI="$FIXTURE" DF_FILES_START_VIEW=list \
    DF_FILES_START_MENU=item
run_scene multiselect   DF_FILES_START_URI="$FIXTURE" DF_FILES_START_VIEW=list \
    DF_FILES_START_SELECT=3
run_scene rename        DF_FILES_START_URI="$FIXTURE" DF_FILES_START_VIEW=list \
    DF_FILES_START_RENAME=1

# --- 2. the Trash listing + Empty Trash confirmation, and the Dock badge. --
run_scene trash-empty DF_FILES_START_URI=trash:// DF_FILES_START_VIEW=list \
    DF_FILES_START_EMPTY_TRASH=1 --dock-crop

# --- 3. a file argument reveals in its parent (Show in Files path). --------
run_scene reveal  DF_FILES_START_URI="$FIXTURE/notes.txt" DF_FILES_START_VIEW=list

# --- 4. the large directory: 100k rows, top and after a scroll sweep. ------
run_scene large-directory DF_FILES_START_URI=file:///synthetic \
    DF_FILES_START_VIEW=list DF_FILES_SYNTHETIC_COUNT=100000 \
    DF_FILES_SYNTHETIC_BATCH=512
run_scene large-directory-scrolled DF_FILES_START_URI=file:///synthetic \
    DF_FILES_START_VIEW=list DF_FILES_SYNTHETIC_COUNT=100000 \
    DF_FILES_SYNTHETIC_BATCH=512 --scroll

# The representative still is the list view.
cp "$OUTDIR/t10-files-list.png" "$OUTDIR/t10-files.png"

# --- 5. the large-directory scroll trace (raw numbers). --------------------
TRACE="$OUTDIR/t10-files-scroll-trace.txt"
{
    echo "# T-10.7 — large-directory scroll trace (raw numbers)"
    echo
    echo "Recorded $(date -u +%Y-%m-%dT%H:%M:%SZ) on the dev host (debug builds)."
    echo "Budgets: docs/design/09-files.md#performance-budgets."
    echo
    echo "## files-core streaming benchmark"
    echo
    echo "    CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core \\"
    echo "        --test performance -- --nocapture --test-threads=1"
    echo
    CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core \
        --test performance -- --nocapture --test-threads=1 2>&1 | grep -E "T-10.5|test result" || true
    echo
    echo "## QML windowed-rendering / scroll sweep (100k synthetic rows)"
    echo
    echo "    QML2_IMPORT_PATH=build/qml QT_QPA_PLATFORM=offscreen \\"
    echo "        QT_QUICK_BACKEND=software DF_FILES_FIXTURE=1 \\"
    echo "        build/apps/files/tests/tst_files_shell \\"
    echo "        -input apps/files/tests/tst_files_shell.qml -v2"
    echo
    env QML2_IMPORT_PATH=build/qml QT_QPA_PLATFORM=offscreen \
        QT_QUICK_BACKEND=software DF_FILES_FIXTURE=1 \
        build/apps/files/tests/tst_files_shell \
        -input apps/files/tests/tst_files_shell.qml -v2 2>&1 \
        | grep -E "QVERIFY\\(first frame|icon delegates|list delegates|scroll jumps|Totals|PASS   : tst_files_shell::FilesShell::test_large|FAIL" \
        || true
    echo
    echo "Full ctest:  ctest --test-dir build -R tst_files_shell --output-on-failure"
    ctest --test-dir build -R tst_files_shell 2>&1 \
        | grep -E "tests passed|Failed" || true
} >"$TRACE"
echo "capture-files: wrote $TRACE"

# --- 6. a short clip of the stills in sequence. ----------------------------
clip="$SCRATCH/concat.txt"
: >"$clip"
for still in list icon context-menu multiselect rename trash-empty reveal \
             large-directory large-directory-scrolled; do
    path="$OUTDIR/t10-files-$still.png"
    [ -f "$path" ] || continue
    printf "file '%s'\nduration 1.8\n" "$(realpath "$path")" >>"$clip"
done
printf "file '%s'\n" "$(realpath "$OUTDIR/t10-files-large-directory-scrolled.png")" >>"$clip"
encoder="mpeg4"
for candidate in libx264 libopenh264; do
    if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q " $candidate "; then
        encoder="$candidate"
        break
    fi
done
ffmpeg -y -loglevel error -f concat -safe 0 -i "$clip" \
    -vf "scale=1280:-2,format=yuv420p" -c:v "$encoder" -crf 28 -movflags +faststart \
    "$OUTDIR/t10-files.mp4"
echo "capture-files: wrote $OUTDIR/t10-files.mp4 ($encoder)"
echo "capture-files: done"