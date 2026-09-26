# 0067 — Session lock protocol and the first-party lock UI

## Status

accepted

## Context

T-12.3a is the safety gate for the whole session track: no real session is
handed to anyone before lock enforcement exists. Smithay already advertised
`ext-session-lock-v1` and confirmed every lock request, but the compositor
ignored the lock surfaces a client created, delivered input to clients while
locked, and there was no first-party lock UI. The shell is the project's trusted
Wayland client (T-07) but its Qt/QML surface stack did not bind the standard
protocol.

## Decision

- **The compositor owns the fail-secure lock state.** A new `LockModel`
  (`compositor/src/lock.rs`) holds one `locked` flag and the lock surfaces keyed
  by output. `SessionLockHandler::lock` enters the locked state and clears
  keyboard focus *before* confirming, so no frame is presented unlocked once a
  lock is requested. `unlock` is reachable only through
  `ext_session_lock_v1.unlock_and_destroy`; a compositor crash ends the session.
- **Input is dropped while locked.** `process_input_event` returns before
  routing any user input (device hotplug still flows), `activate_window_id` and
  `xdg-activation` refuse focus, and a lock surface for every output is
  configured to that output's exact logical size and composited above the
  cursor, windows, and chrome.
- **The shell is the first-party lock UI, over the standard protocol.** The
  shell binds `ext_session_lock_manager_v1` and a `wl_output` per registry
  global, and on `ext_session_lock_v1.locked` paints a `Dragonfruit.Lock` QML
  scene into a lock surface per output. The upstream XML is vendored (MIT) at
  `protocols/wayland-protocols/ext-session-lock-v1.xml` so the shell needs no
  system `wayland-protocols` package; the CMake build runs `wayland-scanner`
  over it exactly like the private protocol XMLs.
- **The trigger is the existing compositor action.** Cmd+Ctrl+Q resolves to
  `InputAction::LockScreen` and is broadcast to the shell over
  `df_toplevel_manager.input_action`; the shell requests the lock. The
  compositor never drives the lock UI itself.
- **Authentication is out of scope here.** The password field is present but
  inert; T-12.3b wires PAM, and T-12.3c owns input capture and kill-resistance.

## Consequences

- T-12.3b adds the authenticator in the shell and calls
  `ext_session_lock_v1.unlock_and_destroy` on success; the compositor needs no
  change to unlock.
- T-12.3c hardens input routing: today *all* input is dropped while locked, so
  the lock surface itself is not yet reachable. That is deliberately the safe
  default until capture is specified.
- The compositor keeps no pixels of the lock UI: it composites the client's
  surface, so a crashed shell keeps the session locked (the surface never
  unlocks) and the compositor renders the clear color rather than leaking the
  desktop.
- `make e2e` gains `session_lock_conformance`, which locks a headless session,
  asserts the `locked` event and a full-output configure for every output, and
  unlocks cleanly.