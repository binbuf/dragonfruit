# T-16.8a crash/kill matrix — reviewed state

This is the human-reviewed companion to the headless reproduction in
`t16-kill-matrix.txt` (`make t16-kill-matrix`). The contract is
[ADR 0157](../design/adr/0157-t16-crash-kill-matrix.md); the outcomes come from
the process model in [01-architecture.md](../design/01-architecture.md).

## Documented outcomes

| Killed | Outcome | Headless proof | Live/VM proof |
|---|---|---|---|
| shell / lock UI | restarts (`always`); the session keeps running | `kill_matrix::killing_the_shell_lock_ui...`, `every_restartable_service...` | not run (no VM) |
| settingsd, menu-broker, app-index, notifications, wallpaperd | restart (`on-failure`); the session keeps running | `kill_matrix::every_restartable_service...` | not run (no VM) |
| portal backend | restarts (`on-failure`) and fails soft; the session keeps running | `kill_matrix::killing_the_portal_backend...` | not run (no VM) |
| app (a user-launched client) | a plain client exit: never restarted; the session keeps running | `kill_matrix::killing_an_app...`, `window_conformance::a_crashed_app...` | not run (no VM) |
| lock UI while locked | fail-secure: the session stays locked, never unlocks | `session_lock_conformance::locked_input_targets_the_lock_ui_and_survives_its_death` | not run (no VM) |
| compositor | the session ends; every other service is stopped; never restarted | `kill_matrix::killing_the_compositor...`; `restart_policy_matrix::the_compositor_exit_ends_the_session_for_every_exit_kind` | not run (no VM) |

## Restart policy matrix (T-16.8b)

The policy behind the rows above is crossed with every way a child can leave in
`services/session/tests/restart_policy_matrix.rs`:

| Policy \ Exit | `exit 0` | non-zero | signal |
|---|---|---|---|
| `always` | restart | restart | restart |
| `on-failure` | stay exited | restart | restart |
| `never` | stay exited | stay exited | stay exited |

The compositor is the anchor: its death ends the session and stops every
survivor for **all three** exits, and it is never restarted. The outcome, the
policy table, and the nested/dev behavior are documented by T-16.8b in
[ADR 0158](../design/adr/0158-compositor-death-ends-the-session.md); the
headless proof is the same suite plus the session `kill_matrix` rows above.

## Live nested check (run for this task)

The nested session was launched (`make demo`), allowed to settle, and captured;
the desktop renders with the menu bar, Dock, wallpaper, Settings, and the demo
client window, with no black/blank/torn regions or stray artifacts:

- `docs/captures/t16-kill-matrix.png` (1920x1080, downscaled from the 3840x2160
  host capture at `/tmp/opencode/t150-desktop.png`).
- Launch/capture harness: `/tmp/opencode/t150-live.sh`; demo log
  `/tmp/opencode/t150-live.log`.

This task changes no user-visible surface, so the check confirms the desktop
still renders rather than exercising a new control.

## Why the real-binary rows are not automated here

A real `kill -9` of `dragonfruit-shell`, `dragonfruit-settingsd`,
`dragonfruit-notifications`, `xdg-desktop-portal-dragonfruit`, or an app under
a running session requires a full session environment (or a VM) and the real
daemons. CI has no VM and no live user session; the headless suites above own
the supervised-recovery contract with the same `Supervisor` the session binary
runs. The real-binary drill is re-verified by T-17.5a (premium gate) and is a
manual VM step for the track. The T-12 capture's
`t12-session-kill-matrix.txt` already records the same `dragonfruit-session`
suite output.