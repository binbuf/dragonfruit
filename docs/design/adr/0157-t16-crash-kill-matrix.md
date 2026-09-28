# 0157 — The T-16 crash/kill matrix and the app/service split

## Status

accepted

## Context

The T-16 platform-polish track asks for "kill tests for every restartable
component" and documents the outcome of each kill
([16-platform-polish-packaging.md](../tracks/16-platform-polish-packaging.md),
"Crash recovery"). [ADR 0064](0064-session-manager-plan-and-restart-policy.md)
and [ADR 0073](0073-session-policy-keys-and-kill-matrix.md) already froze the
supervision vocabulary and a *subset* matrix: T-12.5b's
`services/session/tests/kill_matrix.rs` asserts shell/lock UI, settingsd, and
the notification service restart, and that a compositor death ends the session.
It copied the policies by hand and omitted the portal backend, the other
shipped daemons, and apps. The architecture table the outcomes come from is
`docs/design/01-architecture.md` ("Process model").

## Decision

- **The matrix is derived, not copied.** The headless suite builds its stand-in
  plan from `SessionPlan::default_session()` (name, stage, policy, anchor/gate
  flags preserved) with `sleep` for the program. A new shipped service or a
  changed policy is exercised without editing the test.
- **Every restartable shipped service is killed and its recovery asserted:**
  shell (`always`), settingsd / menu-broker / app-index / notifications /
  wallpaperd (`on-failure`), and the **portal backend** (`on-failure`, stage 2,
  fails soft). The session must reach `Running` again for each; a kill must
  never end the session. A guard test pins the exact restartable name set, so a
  new service must be added here deliberately.
- **Apps are clients, not session services.** The shipped plan contains no app
  entry; the shell/compositor launches apps. The matrix models an app as a
  non-anchored `RestartPolicy::Never` service: killing it is a plain client
  exit — the app is *not* restarted and no session service is disturbed. The
  deeper property ("a crashed app leaves the compositor and other apps
  running") is proven against real Wayland clients in
  `compositor/tests/window_conformance.rs::
  a_crashed_app_leaves_the_compositor_and_the_other_app_running`: two clients
  map windows, one hard-closes its connection, and only its window disappears.
- **The lock UI's kill stays fail-secure.** Killing the lock UI is a shell
  kill; `compositor/tests/session_lock_conformance.rs` already asserts the
  session remains `locked=1`. That test is a row of the matrix report.
- **The reproducible artifact is `docs/captures/t16-kill-matrix.txt`**, produced
  by `scripts/t16-kill-matrix.sh` (`make t16-kill-matrix`). It is headless — no
  host session, no VM, no real services — and runs each row with its exact
  command. The live/VM half is recorded by hand in
  `docs/captures/t16-kill-matrix.md`.
- **Compositor death is out of scope here.** Its session-ending behavior and
  restart policy are documented by T-16.8b.

## Consequences

- Adding a restartable session service fails `the_restartable_set_is_exactly...
  the_shipped_services` until the matrix covers it; the policies asserted in
  the matrix must match `SessionPlan::default_session`.
- The matrix and the session plan are now one source of truth: the test cannot
  drift while `default_session()` changes.
- The live/VM real-binary kill drill remains manual (no VM in CI); it is
  recorded, not faked. T-17.5a re-verifies a subset as part of the premium gate.