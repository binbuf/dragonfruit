#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
set -euo pipefail

handoff_unit=dragonfruit-t238-handoff.service
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

die() {
    printf 't238-native-round-trip: %s\n' "$*" >&2
    exit 1
}

session_property() {
    loginctl show-session "$1" --property="$2" --value
}

desktop_systemctl() {
    local uid="$1" gid="$2" runtime="/run/user/$1"
    shift 2
    local -a command=(env "XDG_RUNTIME_DIR=$runtime" "DBUS_SESSION_BUS_ADDRESS=unix:path=$runtime/bus"
        timeout 65s systemctl --user "$@")
    if ((EUID != uid)); then
        command=(setpriv --reuid="$uid" --regid="$gid" --init-groups "${command[@]}")
    fi
    "${command[@]}"
}

check_other_graphical_sessions() {
    local allowed="$1" sessions session kind
    sessions="$(loginctl list-sessions --no-legend)" || die "could not list login sessions"
    while read -r session _; do
        [[ -n "$session" && "$session" != "$allowed" ]] || continue
        kind="$(session_property "$session" Type 2>/dev/null)" || continue
        case "$kind" in wayland|x11) ;; *) continue ;; esac
        [[ "$(session_property "$session" Class 2>/dev/null)" == greeter ]] && continue
        die "another graphical session ($session) is present"
    done <<<"$sessions"
}

check_desktop() {
    local session="$1" uid="$2" gid="$3"
    [[ -n "$session" && "$(loginctl show-seat seat0 --property=ActiveSession --value)" == "$session" &&
       "$(session_property "$session" User)" == "$uid" &&
       "$(session_property "$session" Type)" == wayland &&
       "$(session_property "$session" Desktop)" == dragonfruit ]] ||
        die "start this from your active Dragonfruit desktop on seat0"
    systemctl is-active --quiet display-manager.service || die "the display manager is unavailable"
    desktop_systemctl "$uid" "$gid" is-active --quiet dragonfruit-session.target ||
        die "could not reach the active Dragonfruit session target"
    check_other_graphical_sessions "$session"
}

select_console() {
    local sessions session console
    local -A occupied=()
    sessions="$(loginctl list-sessions --no-legend)" || return 1
    while read -r session _; do
        [[ -n "$session" && "$(session_property "$session" Seat 2>/dev/null)" == seat0 ]] || continue
        console="$(session_property "$session" VTNr 2>/dev/null)" || continue
        [[ -z "$console" ]] || occupied[$console]=1
    done <<<"$sessions"
    for console in {8..12}; do
        [[ -c "/dev/tty$console" && -z "${occupied[$console]:-}" ]] || continue
        if systemctl is-active --quiet "getty@tty$console.service" "autovt@tty$console.service" ||
            fuser -s "/dev/tty$console"; then
            continue
        fi
        printf '%s\n' "$console"
        return 0
    done
    return 1
}

restore_login() {
    local attempt active sessions session
    timeout 55s systemctl --job-mode=fail start display-manager.service || return 1
    for ((attempt = 0; attempt < 20; attempt++)); do
        active="$(loginctl show-seat seat0 --property=ActiveSession --value)" || return 1
        if [[ -n "$active" ]]; then
            case "$(session_property "$active" Class 2>/dev/null):$(session_property "$active" Type 2>/dev/null)" in
                greeter:*|*:wayland|*:x11) return 0 ;;
            esac
        fi
        sessions="$(loginctl list-sessions --no-legend)" || return 1
        while read -r session _; do
            [[ -n "$session" ]] || continue
            if [[ "$(session_property "$session" Seat 2>/dev/null)" == seat0 &&
                  "$(session_property "$session" Class 2>/dev/null)" == greeter ]]; then
                loginctl activate "$session" || continue
                break
            fi
        done <<<"$sessions"
        sleep 1
    done
    return 1
}

supervise() {
    local root="$1" evidence="$2" desktop="$3" uid="$4" gid="$5"
    local duration="$6" trials="$7" runtime_limit="$8" library_path="$9" handoff_unit="${10}"
    local bench_unit="dragonfruit-t238-drm-${evidence##*.}.service" bench_session='' runner=''
    local vt attempt pid status=0 restore=0
    export PATH=/usr/sbin:/usr/bin:/sbin:/bin

    cleanup() {
        local result="$1"
        trap - EXIT
        trap '' HUP INT TERM
        if [[ -n "$runner" ]]; then
            # PAM moves the workload children to a logind scope separate from
            # the transient service. Clean up that exact scope as well.
            if [[ -n "$bench_session" ]] && loginctl show-session "$bench_session" >/dev/null 2>&1; then
                timeout 20s loginctl terminate-session "$bench_session" || true
            fi
            timeout 25s systemctl stop "$bench_unit" 2>/dev/null || true
            journalctl -u "$bench_unit" --no-pager -o short-iso -n 60 || true
        fi
        if ((restore)); then
            if restore_login; then
                echo 'Login screen restored on seat0.'
            else
                echo 'Could not verify the login screen on seat0. Check display-manager.service.' >&2
                result=1
            fi
        fi
        printf 'device continuity exit status: %s\n' "$result"
        exit "$result"
    }

    # Open the user-owned output file after dropping privileges.
    exec > >(setpriv --reuid="$uid" --regid="$gid" --init-groups tee -a -- "$evidence/handoff.log") 2>&1
    trap 'cleanup "$?"' EXIT
    trap 'exit 143' HUP INT TERM

    check_desktop "$desktop" "$uid" "$gid"
    vt="$(select_console)" || die "could not find a free virtual console (VT 8–12)"
    printf 'Stopping the desktop before starting the benchmark on VT %s.\n' "$vt"
    restore=1
    desktop_systemctl "$uid" "$gid" stop dragonfruit-session.target
    for ((attempt = 0; attempt < 45; attempt++)); do
        loginctl show-session "$desktop" >/dev/null 2>&1 || break
        sleep 1
    done
    ((attempt < 45)) || die "the original desktop session did not exit"
    check_other_graphical_sessions ''
    timeout 55s systemctl stop display-manager.service

    systemd-run --quiet --collect --wait --service-type=exec --expand-environment=no \
        --description='Dragonfruit native DRM benchmark' \
        --unit="$bench_unit" --uid="$uid" --gid="$gid" \
        --property="BindsTo=$handoff_unit" --property="After=$handoff_unit" \
        --property=PAMName=login --property="TTYPath=/dev/tty$vt" \
        --property=StandardInput=tty-fail --property=TTYReset=yes \
        --property="RuntimeMaxSec=${runtime_limit}s" --property=TimeoutStopSec=15s \
        --setenv=XDG_SESSION_TYPE=tty --setenv=LIBSEAT_BACKEND=logind \
        --setenv="LD_LIBRARY_PATH=$library_path" \
        /usr/bin/bash "$root/scripts/t238-native-round-trip.sh" --worker "$evidence" "$vt" "$duration" "$trials" &
    runner=$!

    # Take the session ID from the worker's cgroup, never from a VT guess.
    for ((attempt = 0; attempt < 20; attempt++)); do
        kill -0 "$runner" 2>/dev/null || break
        pid="$(systemctl show "$bench_unit" --property=MainPID --value 2>/dev/null)" || pid=0
        if [[ "$pid" =~ ^[1-9][0-9]*$ ]]; then
            bench_session="$(sed -nE 's@.*/session-([[:alnum:]]+)\.scope$@\1@p' "/proc/$pid/cgroup" 2>/dev/null)" || bench_session=''
            [[ -z "$bench_session" ]] || break
        fi
        sleep 1
    done
    [[ -n "$bench_session" && "$(session_property "$bench_session" Seat)" == seat0 &&
       "$(session_property "$bench_session" VTNr)" == "$vt" &&
       "$(session_property "$bench_session" User)" == "$uid" ]] || die "the benchmark did not acquire its logind session"
    loginctl activate "$bench_session"
    printf 'Benchmark logind session %s activated on VT %s.\n' "$bench_session" "$vt"
    wait "$runner" || status=$?
    printf 'Benchmark exit status: %s\n' "$status"
    exit "$status"
}

run_worker() {
    local evidence="$1" vt="$2" duration="$3" trials="$4" attempt status=0
    exec >"$evidence/benchmark.log" 2>&1
    [[ "$EUID" != 0 && -n "${XDG_SESSION_ID:-}" && -n "${XDG_RUNTIME_DIR:-}" &&
       "$(session_property "$XDG_SESSION_ID" Seat)" == seat0 &&
       "$(session_property "$XDG_SESSION_ID" VTNr)" == "$vt" ]] || die "missing benchmark seat or runtime directory"
    for ((attempt = 0; attempt < 30; attempt++)); do
        [[ "$(session_property "$XDG_SESSION_ID" Active)" != yes ]] || break
        sleep 1
    done
    ((attempt < 30)) || die "the benchmark VT never became active"

    cd "$root"
    unset DISPLAY WAYLAND_DISPLAY XAUTHORITY
    export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
    export OUT_DIR="$evidence" OUT_TRACE="$evidence/acceptance.txt" OUT_OPEN="$evidence/acceptance.open.txt"
    export DURATION="$duration" TRIALS="$trials"
    bash scripts/t238-native-acceptance.sh || status=$?
    if [[ -f "$OUT_OPEN" ]]; then
        status=2
    fi
    exit "$status"
}

if [[ "${1:-}" == --worker ]]; then
    shift
    run_worker "$@"
fi

duration=60 trials=2 dry_run=0 build=1
for option in "$@"; do
    case "$option" in
        --smoke) duration=5; trials=1 ;;
        --dry-run) dry_run=1 ;;
        --no-build) build=0 ;;
        -h|--help)
            cat <<'HELP'
Usage: t238-native-round-trip.sh [--smoke] [--dry-run] [--no-build]

Run from your active Dragonfruit desktop. Save work before starting.
The runner builds the benchmark, asks for sudo, logs you out, runs the native
DRM workloads as your user, then restores the graphical login. Results and
startup logs are saved in docs/captures/t238-native-round-trip/.

  --smoke     Run one five-second trial per workload to check the test setup.
  --dry-run   Check the active session and print the plan without logging out.
  --no-build  Use the existing shell and release compositor binaries.
HELP
            exit 0 ;;
        *) die "unknown option: $option" ;;
    esac
done

[[ "$EUID" != 0 ]] || die "run as your desktop user so the benchmark runs unprivileged"
for command in loginctl systemctl systemd-run sudo setpriv fuser timeout Xwayland xmessage python3; do
    command -v "$command" >/dev/null || die "missing required command: $command"
done
desktop="$(loginctl show-seat seat0 --property=ActiveSession --value)"
desktop_gid="$(id -g)"
check_desktop "$desktop" "$EUID" "$desktop_gid"
python3 -c 'import tkinter' || die "the scroll workload needs Python Tk support"
if ((dry_run)); then
    printf 'Plan: %s trial(s) per workload, %ss each. Stop the desktop, run on a separate DRM console, then restore graphical login.\n' "$trials" "$duration"
    printf 'Evidence directory: %s\n' "$root/docs/captures/t238-native-round-trip"
    exit 0
fi

cd "$root"
if ((build)); then
    make t238-native-build
fi
[[ -x target/release/dragonfruit-compositor && -x build/shell/src/dragonfruit-shell ]] ||
    die "build the release compositor and shell with make t238-native-build"

umask 077
mkdir -p docs/captures/t238-native-round-trip
evidence="$(mktemp -d "$root/docs/captures/t238-native-round-trip/run.XXXXXX")"
runtime_limit=$((12 * trials * (duration + 8 + 90) + 120))
printf 'Saving results to %s\n' "$evidence"
printf 'The desktop will log out after sudo. From a text console, cancel with sudo systemctl stop %s.\n' "$handoff_unit"

# Snapshot the privileged control flow in the transient unit. The workspace
# benchmark script and binaries are opened only by the unprivileged worker.
supervisor_source="$(declare -f die session_property desktop_systemctl check_other_graphical_sessions check_desktop select_console restore_login supervise)"$'\nsupervise "$@"'
status=0
sudo systemd-run --quiet --collect --wait --service-type=exec --expand-environment=no \
    --description='Dragonfruit native benchmark handoff' \
    --unit="$handoff_unit" --property="RuntimeMaxSec=$((runtime_limit + 300))s" --property=TimeoutStopSec=125s \
    --property=KillMode=mixed \
    --property='ExecStopPost=/usr/bin/systemctl --job-mode=fail start display-manager.service' \
    /usr/bin/bash -c "$supervisor_source" t238-handoff "$root" "$evidence" "$desktop" "$EUID" "$desktop_gid" \
    "$duration" "$trials" "$runtime_limit" "${LD_LIBRARY_PATH:-}" "$handoff_unit" || status=$?
if ((status)); then
    printf 'device continuity exited with status %s. Evidence is in %s\n' "$status" "$evidence" >&2
    if [[ -s "$evidence/handoff.log" ]]; then
        tail -20 "$evidence/handoff.log" >&2
    fi
fi
exit "$status"
