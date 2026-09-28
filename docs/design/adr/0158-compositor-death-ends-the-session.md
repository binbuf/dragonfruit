# 0158 — Compositor death ends the session

## Status

accepted

## Context

The compositor owns the seat, DRM/KMS, and raw input, and is the session
anchor. The process model already says its crash "ends the session by design;
the display manager returns the user to a login screen"
([01-architecture.md](../01-architecture.md#process-model)), and T-16.8a's
crash/kill matrix grew the supervision suite to the whole shipped composition
([ADR 0157](0157-t16-crash-kill-matrix.md)). That ADR deliberately left the
*compositor-death behavior and restart policy* to T-16.8b: the outcomes existed
only as scattered prose across the architecture, session, and lock docs, and
the restart policy itself had no single table or headless matrix.

Two questions needed one answer: what exactly happens when the compositor dies
in each deployment (nested dev, DRM session), and what is the restart policy
for it versus everything else.

## Decision

- **Compositor death ends the session; the compositor is never restarted.**
  In `SessionPlan::default_session()` the compositor is the one
  `ends_session` service and its policy is `RestartPolicy::Never`; the shipped
  systemd unit `services/session/units/dragonfruit-compositor.service` is
  `Restart=no`. A restartable anchor is rejected by `SessionPlan::validate()`,
  because a restarted compositor could not end the session and would strand
  every client.

- **Every other shipped service restarts in place.** The restart policy is:

  | Service | Policy | A death means |
  |---|---|---|
  | compositor | `never` (anchor, `ends_session`) | the session ends; all survivors stop; the greeter returns |
  | shell / lock UI | `always` | it restarts; windows and the session keep running; a locked session returns to a lock UI |
  | settingsd, menu-broker, app-index, notifications, wallpaperd | `on-failure` | they restart in place; clients re-read on reappearance |
  | portal backend | `on-failure` (stage 2) | it restarts and fails soft; portals degrade, the session keeps running |
  | apps (clients) | not supervised | a plain client exit; only that window goes |

- **The display manager is the recovery point.** The T-12.2 session entry
  waits for `dragonfruit-compositor.service` to leave the active state, stops
  `dragonfruit-session.target`, clears failed state, and removes the runtime
  hand-off files, so the greeter returns and the next login starts clean
  ([11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md#the-display-manager-session-entry-t-122)).
  Recovery is a fresh login, never an in-place compositor restart.

- **No live handoff.** Wayland clients are bound to one compositor and cannot
  migrate into a new one. Ending the session is therefore the only honest
  recovery; this is why the anchor must be `Never`.

- **Nested dev ends only the nested session.** Closing the nested compositor
  window (or `make demo` teardown) ends the private session; the host desktop
  was never disturbed.

- **A crash while locked can never unlock.** The lock is fail-secure inside
  the compositor; ending the session leaves the lock unreachable, which is the
  safe outcome (see [ADR 0067](0067-session-lock-protocol-and-ui.md),
  [ADR 0069](0069-locked-input-capture-and-kill-resistance.md)).

- **Subprocess restarts are not session restarts.** The compositor restarts
  Xwayland internally (`compositor/src/xwayland.rs::maybe_restart`); that is a
  process the compositor owns, not the session anchor.

- **Verified headlessly by the restart-policy matrix.**
  `services/session/tests/restart_policy_matrix.rs` crosses all three policies
  with all three exit kinds (`exit 0`, non-zero, signal) and crosses compositor
  death with the same three exits, asserting the session ends and survivors
  stop. The whole composition is covered by
  `services/session/tests/kill_matrix.rs` and the reproducible transcript
  `make t16-kill-matrix`; the real-binary `kill -9` drill is manual/VM and
  recorded in `docs/captures/t16-kill-matrix.md`.

## Consequences

- The restart policy and the compositor-death outcome are now one table
  (above) that the headless matrix enforces; a policy change in `plan.rs` or a
  unit's `Restart=` that disagrees fails `services/session/tests/restart_policy_matrix.rs`
  or `units.rs`.
- A future attempt to make the compositor restartable fails
  `SessionPlan::validate` and the anchor tests; live handoff would have to be a
  new decision, not a policy flip.
- The live/VM real-binary drill remains manual (no VM in CI); T-17.5a
  re-verifies a subset for the premium gate.