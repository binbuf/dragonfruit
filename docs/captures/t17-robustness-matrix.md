# T-17.5a absent-daemon and crash matrix — reviewed state

This is the premium-gate companion to the headless verification transcript
`t17-robustness-matrix.txt` (`make t17-robustness-matrix`). It re-runs the two
robustness contracts on the release tree and adds the one absence case neither
predecessor covered: a service whose program cannot be spawned must not block
session start. The contract is
[ADR 0177](../design/adr/0177-absent-services-never-block-session-start.md).

## The contract

| Question | Contract | Headless proof |
|---|---|---|
| An optional service is absent at startup | degrade; never block start | `absent_services::every_absent_optional_service_degrades_and_never_blocks_start` |
| Any gate service fails to spawn | the next stage still starts | `absent_services::an_absent_gate_service_does_not_block_the_next_stage` |
| The compositor (anchor) is absent | the session ends by design | `absent_services::only_the_anchor_absence_ends_the_session` |
| A Settings provider is absent | every shipped pane mounts and stays live | `tst_settings_absence` (private bus, no settingsd/portal/host) |
| A host daemon is absent | the adapter projects absent, never an error | `dragonfruit-system-status absent`, `dragonfruit-system-adapters subscription absent` |
| A supervised service is killed | restarts per policy; the session keeps running | `kill_matrix` (T-16.8a) |
| A service leaves any way (`0`/non-zero/signal) | restart decision per policy | `restart_policy_matrix` (T-16.8b) |
| The compositor dies | the session ends; never restarted | `kill_matrix`, `restart_policy_matrix` |
| An app crashes | the desktop and the other app keep running | `window_conformance::a_crashed_app_leaves_the_compositor_and_the_other_app_running` |
| The lock UI is killed while locked | the session stays locked (fail-secure) | `session_lock_conformance::locked_input_targets_the_lock_ui_and_survives_its_death` |

The pane half is the T-15.16 absent-daemon masking matrix
([0148](../design/adr/0148-t15-absent-daemon-masking-matrix.md)); the kill half
is T-16.8a/T-16.8b
([0157](../design/adr/0157-t16-crash-kill-matrix.md),
[0158](../design/adr/0158-compositor-death-ends-the-session.md)). The new
session-start half is [ADR 0177](../design/adr/0177-absent-services-never-block-session-start.md).

## Why "no absence blocks start" holds structurally

The supervisor records a spawn failure as `ServiceState::Failed` **once** and
never retries it, and its `stage_ready` withholds a stage only while a `gate`
service is *running but not signalled* — an absent gate is already non-pending,
so the next stage starts. The compositor is the only `gate` and the only
`ends_session` anchor in `SessionPlan::default_session`; every other service is
optional, so its absence is a degraded desktop and only the compositor's absence
ends the session. `absent_services::the_only_gate_in_the_shipped_plan_is_the_compositor`
pins that invariant, so a future non-anchor `gate` fails the suite before it
could reintroduce a start deadlock.

## Live nested check (run for this task)

The nested session was launched (`make demo --socket-name dragonfruit-t156`),
allowed to settle, and captured:

- `docs/captures/t17-robustness-matrix.png` — the nested desktop, downscaled
  from the 3840×2160 host capture at `/tmp/opencode/t156-desktop.png`.
- Launch/capture harness `/tmp/opencode/t156-live.sh`; demo log
  `/tmp/opencode/t156-live.log`.

The desktop renders with the wallpaper, menu bar, Dock, the Settings window,
and the X11 demo window; no black/blank/torn regions and no stray artifacts.
This task changes no user-visible surface, so the check confirms the desktop
still renders rather than exercising a new control.

## Why the real-binary rows are not automated here

A real `kill -9` of the shipped binaries under a running session, and stopping
real daemons in a VM, is the design's real-hardware check. CI has no VM and no
free live session; the headless suites above own the supervised-recovery and
start-absence contracts with the same `Supervisor` the session binary runs. The
per-daemon live matrix remains `t15-absence-matrix.md`; the live kill drill
remains `t16-kill-matrix.md`. T-17.5b verifies leaks and lock enforcement.