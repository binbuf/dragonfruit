# 0132 — The Lock Screen policy adapter projects the session idle engine and the compositor lock

- **Status:** Accepted (T-15.8a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md),
  ADR [0067](0067-session-lock-protocol-and-ui.md),
  ADR [0070](0070-idle-timer-engine-and-policy.md),
  ADR [0126](0126-mission-control-hot-corners-adapter.md)

## Context

T-15.8 splits the Lock Screen surface into an adapter (T-15.8a) and a Settings
pane plus Control Center tile (T-15.8b). Unlike Bluetooth, storage, audio, and
input, this subsystem has **no external daemon**: the session's idle/lock
engine (`services/session/src/idle.rs`) owns the `idle.*` stage delays and the
compositor owns the one fail-secure lock state
(`compositor/src/lock.rs`); the shell mirrors both over the private
`df_toplevel_manager` bridge. `System_Preferences.md` routes the pane to
"settingsd + compositor lock/idle policy" — the same split principle 3 already
gives keyboard/pointer policy and the overview triggers: `settingsd` owns the
durable preference and the session/compositor applies it live.

The task requires an adapter with state, events, and absent-daemon behavior,
unit-tested against a mock. The risk is re-timing an idle stage or
re-implementing a lock transition in a service, which the task forbids
("reuse, never reimplement") and which would fork the one fail-secure lock
state ADR [0067](0067-session-lock-protocol-and-ui.md) freezes.

## Decision

- **A new crate, `dragonfruit-lock-adapter` (`services/lock-adapter`).** It
  implements the T-07 adapter contract (`Adapter`, `AdapterState`,
  `Subscription`) behind its own `LockPolicySource` seam, exactly like
  `dragonfruit-overview` and `dragonfruit-input`. `AdapterId::LOCK` (id
  `lock`) joins the shared ids.
- **It projects; it does not lock or time.** The raw read is
  `LockPolicyData`: the runtime `locked` flag, the session's own `IdlePolicy`
  (dim/blank/lock/suspend), and the lock-screen display options. The typed
  `LockPolicySnapshot` types the lock state (`Unlocked`/`Locked`), names the
  four `LockDisplayOption`s whose stable ids are the settings key suffixes, and
  derives `display_off_after()`/`lock_after()` from the reused policy. The
  adapter depends on `dragonfruit-session` to **reuse**
  `IdlePolicy`/`IdleStage`; it never re-models the chain.
- **One event stream.** `LockPolicySnapshot::changes(previous)` is a pure diff
  — the lock state, each idle stage delay, each display option, and the custom
  message — queued by the adapter and drained through `drain_changes`. A
  lock/unlock is a `StateChanged` event, so nothing above the adapter polls.
- **Read-only.** The compositor owns the lock and the session owns the timing;
  the durable display preferences are `settingsd`'s (the `lock.*` keys land in
  T-15.8b). The adapter has no write method.
- **Absence is a normal state.** A missing compositor bridge (no
  `df_toplevel_manager` global, or an unreachable compositor) is
  `AdapterState::Unavailable` and hides the item; a present bridge that cannot
  be read is `Error`, visible and inert with the message. Neither blocks
  session startup. `MockLockPolicy` drives the states in CI, with
  `kill`/`restart` for the re-subscribe lifecycle and `push` for the state and
  policy stream.

Rejected: placing the adapter in `dragonfruit-session` or the compositor (the
T-15 track is the services layer; the shell bridge is the only consumer path,
and depending on the compositor drags the Smithay stack into a
dependency-light adapter); inventing a second lock state or a second idle
timer (forks ADR [0067](0067-session-lock-protocol-and-ui.md) and
ADR [0070](0070-idle-timer-engine-and-policy.md)); adding a live D-Bus source
(neither the lock state nor the idle policy is on the bus; the concrete source
is the shell's `df_toplevel_manager` client, which T-15.8b wires).

## Consequences

- `dragonfruit-lock-adapter` is a workspace member whose dependencies are the
  adapter contract and `dragonfruit-session` (for the reused idle vocabulary).
  CI drives it with `MockLockPolicy`; no compositor, Wayland socket, or
  hardware is involved. 22 unit tests in-crate plus
  `services/lock-adapter/tests/read_path.rs` (7 integration tests) are run by
  `make e2e` (`cargo test -p dragonfruit-lock-adapter`).
- The concrete bridge source is the shell's `df_toplevel_manager` client,
  which is C++: T-15.8b wires the pane and tile (likely shell-native plus
  settingsd keys, per the T-15.5b/Mission Control precedent) and declares the
  `lock.*` display keys this adapter names. Until then the live path is the
  seam the bridge fills; the mock is the tested path.
- A later task that wants to *apply* a policy change must write the settings
  key and let the idle engine apply it, not add a write to this adapter.