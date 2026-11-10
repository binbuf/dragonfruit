# 0069 — Locked input capture, kill-resistance, and a trusted lock client

## Status

accepted

## Context

T-12.3a shipped the fail-secure lock by dropping *all* user input while locked,
which made the lock surface itself unreachable; T-12.3b added PAM behind
`ShellController::submitLockPassword` but nothing could call it. T-12.3c closes
that gap without reopening the "no input reaches a client" invariant, and it
also has to survive the lock UI dying: `ext-session-lock-v1` says a crashed lock
client must not unlock the session. Finally, the `ext_session_lock_manager_v1`
global was advertised with an open `|_| true` filter, so any client could
request a lock and then `unlock_and_destroy` the real one (the T-81 TODO).

## Decision

- **Keyboard input is focused on the lock surface.** `SessionLockHandler::
  new_surface` focuses the seat keyboard on the newly created lock surface; the
  compositor's locked-input path (`input::route_locked_keyboard`) forwards key
  events to the focused lock surface with no shortcut filter. Pointer, touch,
  and gesture events stay dropped. The lock UI is therefore the only reader of
  input while locked.
- **No live lock surface means input is dropped.** `LockModel::input_surface`
  returns a surface only while one is alive. A dead lock UI leaves the session
  locked with no input target; `route_locked_keyboard` then discards events.
  Losing surfaces (`LockModel::retain_live`) never clears the `locked` flag —
  only `unlock_and_destroy` does. This is the kill-resistance invariant.
- **Only the trusted shell owns the lock.** `SessionLockHandler::lock` checks
  the requesting client against the T-07 launch-token trust model and drops the
  `SessionLocker` (sending `finished`, creating no lock) for anyone else. An
  untrusted client can neither install a fake lock UI nor release the real one.
- **The shell translates captured keys with the compositor keymap.** The lock
  surface's `wl_keyboard` events are routed to `lockKeyEvent`; `ShellController`
  fills a private `QString` password buffer and exposes only its length to the
  lock scene, which draws bullets. `ShellProtocol` keeps the
  `wl_keyboard.keymap` fd, feeds `wl_keyboard.modifiers` into an xkb state, and
  passes the current-layout text to the controller, so Shift, Caps Lock, AltGr,
  non-US layouts, and the numeric keypad all type correctly. A fixed US/ASCII
  table (`shell/src/lockinput.cpp`) remains only as the fallback for the brief
  window before the keymap arrives (and for the chrome scenes, which do not use
  the password path).

## Consequences

- A shell crash keeps the session locked and inputless; there is no in-session
  recovery path for a dead lock UI. Recovering requires the session manager's
  restart policy and, because Smithay keeps `locked_outputs`, a compositor
  restart for a fresh lock — T-12.5/T-12.6 own that matrix.
- The password buffer lives only in the shell process and is handed to the PAM
  helper's stdin; the lock scene never sees the characters.
- Non-US keyboard layouts type an ASCII approximation until a later task adds
  an xkb state to the shell.
- `session_lock_conformance` now authenticates as the trusted shell (the token
  handshake) and asserts `query lock` stays `locked=1` after the lock client's
  socket is closed.