# 0312 — A lost lock client ends the session

## Status

Accepted (2026-11-22). Supersedes the “a locked session returns to a lock UI”
claim in [ADR 0158](0158-compositor-death-ends-the-session.md) and narrows the
“no in-session recovery” consequence in
[ADR 0069](0069-locked-input-capture-and-kill-resistance.md). The fail-secure
lock state itself is unchanged. [ADR 0317](0317-lock-and-suspend-tear-down-client-grabs.md)
covers the input-grab teardown that happens before the lock is confirmed.

## Context

The session lock is fail-secure inside the compositor: `LockModel::locked` is
set before confirmation, survives a dead lock surface, and is cleared only by
`ext_session_lock_v1.unlock_and_destroy` (ADR 0067/0069). The first-party lock
UI is the shell process.

Two accepted records disagree about what happens when that process dies:

- ADR 0069: “there is no in-session recovery path for a dead lock UI.
  Recovering requires the session manager’s restart policy and, because
  Smithay keeps `locked_outputs`, a compositor restart for a fresh lock.”
- ADR 0158: “the shell / lock UI restarts in place; a locked session returns
  to a lock UI.”

The code follows neither recovery path. `services/session/units/dragonfruit-shell.service`
is `Restart=always`, so the shell restarts, but the compositor’s `LockModel`
stays locked and Smithay’s `SessionLockManagerState.locked_outputs` still
contains the old `wl_output` resources. A restarted shell has no `lock_state`
event to learn that the session is locked, and a new `get_lock_surface` for an
output already in `locked_outputs` is a protocol error. The result is a
locked, inputless session with no lock surface: the desktop is visible but
frozen, and the user must switch VT or reboot.

## Decision

1. **If the trusted shell disconnects while the session is locked, the
   compositor ends the session cleanly.** In `df_core::destroyed`, when the
   disconnecting role is `TrustedRole::Shell` and `state.lock.is_locked()`, the
   compositor stops its event loop and returns from the session. It does **not**
   unlock, and it does not attempt a new lock in the same process.
2. **The session entry returns to the greeter.** The compositor is the session
   anchor; the entry stops the target and removes the runtime hand-off files.
   The next login starts a fresh compositor and a fresh lock state.
3. **The shell’s `Restart=always` remains correct for the unlocked case.** A
   shell crash while unlocked restarts the chrome in place; a shell crash while
   locked ends the session instead. The compositor is the one authority that
   distinguishes the two.
4. **In-session re-lock is a future decision, not a silent recovery.** It
   requires either a separate lock process that survives a shell crash or a
   Smithay change that clears `locked_outputs` on lock-client loss plus a
   private `lock_state` event. A new ADR must record that choice; this ADR does
   not authorize it.
5. **Tests and docs follow the decision.** `session_lock_conformance` asserts
   the compositor stays `locked=1` and exits after the lock client disappears;
   `services/session/tests/kill_matrix.rs` records “session ends safely” rather
   than “returns to a lock UI”; ADR 0158 and 0069 get supersession pointers.

Rejected alternatives:

- **Unlock on shell crash.** It would make a crash an unlock primitive and
  violate the fail-secure invariant.
- **Keep the old lock surfaces alive across the client disconnect.** Wayland
  objects die with the client; there is nothing to keep.
- **Restart the compositor in place.** Wayland clients cannot migrate; the
  session is over either way. Exiting cleanly is the honest path.
- **A shell-side “am I locked?” probe without a protocol event.** The shell has
  no way to ask, and a new lock cannot cover outputs already in
  `locked_outputs`.

## Consequences

- A shell crash while locked logs the user out. This is a deliberate trade:
  losing the session is better than a frozen desktop or an unlock on crash.
- The kill matrix and ADR 0158 are corrected; the README/STATUS language says
  “ends the session safely,” not “returns to the lock UI.”
- A future separate lock process is the clean fix. It would own the
  `ext_session_lock_v1` object and survive a shell restart; the shell would
  draw only the desktop chrome.
- The compositor’s exit path is already the session-anchor path; no new
  service or supervisor behavior is required.
