# 0318 — Focus clearing is one helper; minimize does not refocus

## Status

Accepted (2026-11-22). Implements the focus invariant in
[design 27](../27-stability-and-daily-driver.md) §3.2 and constrains T308 and
T329.

## Context

`active_window` is a cache of the keyboard focus owner, but several paths
cleared the seat focus without clearing the cache. After a Space switch the
keyboard was released while `active_window` still named the hidden window, so
`activate_window` returned early when the user clicked that same window again
and typing never returned. Minimizing had the mirror problem: the focused
window left the layout but kept the keyboard, so keys reached a hidden
surface.

Smithay's `KeyboardHandle::set_focus(None)` sends `wl_keyboard.leave` but does
not call `SeatHandler::focus_changed`, so the model, the shell event, the
menu-bar flag, and the shortcut engine all have to be updated by hand. Close
already did that inline, which is the pattern that drifted.

## Decision

1. **One helper owns window focus clearing.** `DfState::clear_keyboard_focus`
   unsets the seat focus, clears `active_window`, broadcasts `Unfocused` only
   when a window was actually active, dismisses that window's popups, clears
   the shortcut engine's focused app, clears `menu_bar_keyboard_focused`,
   re-applies the menu-bar reveal rule, marks the scene dirty, and refreshes
   the following magnifier. Close, a Space change, and minimize call it
   instead of writing their own sequence.
2. **A Space change clears focus when the active window is no longer on the
   active Space.** The window stays in the model and returns with its Space;
   it does not regain the keyboard until the user clicks it.
3. **Minimize clears focus; it does not choose a replacement.** When the
   active window is in the minimized transient tree, focus becomes `None`.
   Implicitly focusing the next window would be a policy choice the user did
   not make; the next click selects the new owner.
4. **Activation compares the keyboard handle, not the cache.**
   `activate_window` sets focus whenever `keyboard.current_focus()` differs
   from the target surface, so a stale `active_window` can never turn a click
   into a no-op.
5. **Conformance proves typing, not just the report.**
   `window_conformance` binds `wl_keyboard`, switches Spaces away and back,
   minimizes, and restores, asserting that a hidden surface receives no key
   and that a click restores both focus and key delivery.

Rejected alternatives:

- **Repair `active_window` in each caller.** That is the four-ad-hoc-calls
  pattern that produced the drift; a caller can forget one of the six
  side-effects.
- **Keep the cache as the activation authority.** The keyboard handle is the
  authority; the cache is only a mirror.
- **Focus the next window on minimize.** It hides the state change and makes
  focus depend on stacking order, which later window-menu work (T329) would
  have to unwind.

## Consequences

- Any new path that hides, destroys, or otherwise de-focuses a live window
  must call `clear_keyboard_focus` rather than `set_focus(None)`.
- The shell receives exactly one `Unfocused` event per window that loses
  focus; a call with no active window is a silent no-op.
- Minimize leaves the desktop with no keyboard owner until the user clicks a
  window (or chrome takes focus). That is the documented MVP rule.
