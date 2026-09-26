# 0073 — Session policy keys, the kill matrix, and the T-12 capture

## Status

accepted

## Context

T-12.4a froze the idle chain's key *names* (`idle.dim`, `idle.blank`,
`idle.lock`, `idle.suspend`) and its parser (`IdlePolicy::from_keys`) in
[ADR 0070](0070-idle-timer-engine-and-policy.md), leaving the settingsd
registration, the kill matrix, and the track capture to T-12.5b. The legacy
design (T-26) also requires a kill drill: every plausible crash during lock
must leave the session locked or end it — never unlocked.

Two choices had to be made once so the Settings pane (T-15.8b), the future
idle service, and the hardening tasks (T-16.8a/T-17.5a) do not each invent a
different shape: the D-Bus type of the idle keys, and which kill outcomes the
in-repo tests own versus the compositor.

## Decision

- **The four idle keys are `x` (int64) whole seconds, `0` disables a stage,
  0–86400 s.** `services/settingsd` declares them in a new `session` key group
  with defaults dim 150 / blank 300 / lock 600 / suspend 0, matching
  `IdlePolicy::new()`. A consumer's snapshot bridge is
  `Value::to_string()` into `IdlePolicy::from_keys`; the daemon stays the one
  owner of the values and the engine stays dependency-free.
- **The keys are registered, but the production reader is still future
  work.** There is no idle service in the tree yet; T-12.5b proves the keys
  move the chain headlessly (`services/session/tests/session_policy.rs`) and
  registers them for the service T-12.6/portals bring up. The Settings pane
  (T-15.8b) owns writes; the daemon seeds the defaults.
- **The kill matrix is split by owner.** The compositor owns the fail-secure
  lock invariant — killing the lock UI never unlocks — and asserts it in
  `compositor/tests/session_lock_conformance.rs`. `dragonfruit-session` owns
  supervision and asserts it in `services/session/tests/kill_matrix.rs`: the
  shell/lock UI restarts (`always`), settingsd and the notification service
  restart (`on-failure`), and a compositor death ends the session and stops
  everything else, never restarting the anchor.
- **The T-12 capture is `docs/captures/t12-session.*`**: the nested desktop,
  the first-party lock UI after the real Cmd+Ctrl+Q chord, a short clip, a
  `query lock`/`query session` transcript, and the kill-matrix test output.
  It is produced by `make session-capture`; the greeter/login ends of the
  demo remain DRM-gated (T-12.6).

## Consequences

- T-15.8b reads the `x` keys directly; "Never" is `0`, not a text spelling.
  The `from_keys` `never` spelling remains a parser leniency only.
- Bumping `SCHEMA_VERSION` to 5 migrates older settings files by filling the
  new defaults; `docs/settings-keys.md` and the shell's `MockSettingsClient`
  defaults are kept in lockstep by their own tests.
- The no-live-under-lock kill matrix on real hardware (VM, deliberate crash)
  is T-16.8a/T-17.5a; this ADR only fixes the in-repo half and the vocabulary.