# 0359 — The stability close-out is agent-green, not a daily-driver sign-off

## Status

accepted

## Context

T348 is the only task that runs `make gates` and owns the release verdict for
the stability set (T304–T348). By 2026-11-25 the 46 implementation tasks
landed: the lock/focus/session defects, idle and power wiring, launch and
recovery bounds, multi-monitor and X11 compatibility, Files completeness, and
the bring-up/packaging path. The code-side gate is green.

The T347 reference-hardware matrix is the other half of the contract. It is a
PASS/FAIL/`OPEN` report with one row per native check; a host without a free
DRM seat gets `OPEN` rows with the exact reference-laptop command, never a
skipped check. The close-out host has no free DRM seat and no native intake, so
all 17 native rows are `OPEN` even though all 14 headless checks pass.

Two QML absence tests were also stale: T289/T292b moved the Displays rotation
control, the Menu Bar clock options, and the Workspace Overview gestures/corners
into sheets, but `tst_settings_displays` and `tst_settings_absence` still
interacted with the controls while the sheet was closed. A closed sheet
disables its content, so the toggles were no-ops. The gate could not be green
until the tests opened the sheet they were testing.

## Decision

- **The stability set is agent-done.** `make gates` and `make verify-stability`
  are green on the final tree, and every implementation task has its targeted
  test and progress note.
- **The daily-driver claim stays unflipped.** The README's "not yet intended as
  a stable daily-driver desktop" note remains and names the T347 blocker. A
  headless check cannot supply a native row; a P0 `OPEN` row blocks the claim.
- **The human verdict is batched and remains `OPEN`.** No agent task is marked
  human-signed-off. The six-step walkthrough (login, launch, lock/unlock,
  suspend/resume, hotplug, logout) is recorded as `OPEN` until it runs on
  reference A.
- **Stale evidence is fixed, not waived.** The two absence tests open the
  owning sheet before asserting the settingsd-backed control is live. This is a
  test/evidence correction, not a product behavior change.
- **The next set is hardware evidence plus the deferred register.** Reference-A
  native evidence is the blocker; the T346 register is the input to later
  promotion. No new surface work is promoted by the close-out.
- **A green `make gates` updates the agent rows only.** The README status line
  changes when T347 is PASS or every row is an explicit `OPEN` whose reason
  does not contradict the claim.

## Consequences

- `STATUS.md`, `14-risks.md`, `docs/design/README.md`, the task-set README, and
  design 27 §5.3/§6.1 record the same verdict; the T347 report is the hardware
  evidence and the local intake is the native record.
- A no-seat CI run can be green while the release gate stays `OPEN`; that is the
  intended safe outcome, not a missing test.
- The next session's first action is `make t347-daily-driver-matrix` on
  reference A, not a new feature.
- The close-out can be re-run without changing the contract: the report,
  intake, and README rule are stable.
