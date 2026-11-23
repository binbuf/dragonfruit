# 0317 — Lock and suspend tear down client input grabs

## Status

Accepted (2026-11-22). Narrows the input-capture claim in
[ADR 0069](0069-locked-input-capture-and-kill-resistance.md) and complements
[ADR 0312](0312-lock-loss-ends-the-session.md).

## Context

The fail-secure lock clears keyboard focus and routes locked input itself, but
a client can hold a grab that outranks focus. An `xdg_popup` keyboard grab
(`PopupKeyboardGrab`) ignores `set_focus` while the grab is alive and rewrites
the focus to the popup on every key, so keys typed into the lock UI reached the
popup client. A move/resize/drag pointer grab likewise keeps consuming motion.
A grab also survived suspend/resume: after the wake the compositor routed input
normally while the old grab was still installed.

Smithay's pointer `unset_grab` restores the grab's pending focus, and a popup
grab's focus is the popup. Simply unsetting the grab is therefore not enough;
the popup must also be dismissed, and the focus must be re-established by the
compositor.

## Decision

1. **One teardown helper owns non-client-initiated grab removal.**
   `DfState::release_client_grabs` unsets the seat keyboard and pointer grabs,
   clears the pointer focus that `unset_grab` restored, dismisses popups rooted
   at the active window and at every live chrome layer surface, and closes the
   compositor-owned window menu. It marks the scene dirty when it changed
   anything. This is the only path that takes a grab away without a client
   request.
2. **The lock path tears down before it clears focus and confirms.**
   `SessionLockHandler::lock` calls the helper after setting `LockModel::locked`
   and before `set_focus(None)` and `confirmation.lock()`. The lock surface is
   then the only possible focus when the lock is confirmed, and
   `route_locked_keyboard` re-focuses it on the first key.
3. **Suspend uses the same teardown.** `suspend_session` releases the grabs
   before the frame loop stops, so a held drag or popup grab cannot resume
   after wake. If a keyboard grab was released, focus returns to the active
   window (the popup is gone) rather than to a dismissed popup.
4. **Popups are dismissed, not merely unfocused.** Dismissal sends
   `xdg_popup.popup_done`, which is the client-visible end of the grab, and
   removes the overlay from the scene. The window menu is chrome and is closed
   the same way.
5. **Conformance covers all three paths.** `session_lock_conformance` maps a
   client popup with a grab and asserts a locked key never reaches it; starts a
   titlebar move grab and asserts locked motion/buttons do not move the window
   or reach the client; and repeats the held drag across suspend/resume.

Rejected alternatives:

- **Rely on `set_focus(None)`.** `PopupKeyboardGrab::set_focus` ignores it
  while the grab is alive; this was the defect.
- **Let the grab's own `has_ended` check clear it.** It only runs on the next
  key, which is exactly the key that must not reach the popup.
- **Dismiss only the active window's popups.** Chrome can root an `xdg_popup`
  at a layer surface, which is not a `Window`; the chrome roots and the
  window menu are covered explicitly.

## Consequences

- Lock and suspend can emit focus/leave and `popup_done` events without a
  client request. Clients see the same protocol events as a click-away or a
  parent-window change.
- Pointer focus is cleared on teardown; the next motion (or the lock surface)
  re-establishes it. A resumed session does not restore the pre-sleep grab.
- The pointer's physically pressed buttons are not synthesised into releases;
  the next real release updates the seat. Clients already lost the grab, so
  they receive no drag continuation.
- The lock path remains fail-secure: no client receives a key while locked,
  and the teardown never clears `LockModel::locked`.
