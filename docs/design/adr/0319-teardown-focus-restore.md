# 0319 — Teardown restores focus from the `df_core` path; layer ownership is recorded

## Status

Accepted (2026-11-22). Implements the focus invariant in
[design 27](../27-stability-and-daily-driver.md) §3.2 and constrains later
teardown work (T314 and the remaining Phase-1 tasks).

## Context

Two teardown paths could leave the keyboard on a dead surface:

1. `remove_window` cleared the cached `active_window` but did not unset the
   seat keyboard, so a client crash (or a completed close ghost) left the
   keyboard handle naming a destroyed `wl_surface`. `query focus` only read
   the cache, so the disagreement was invisible.
2. `forget_client` removed a disconnecting client's layer entries before
   `DfLayerSurface::destroyed` ran. That callback used the entry to decide
   whether the dying layer held the keyboard, so it could not restore focus.
   The old removal used `resource.client()`, which does not reliably resolve
   while the client is disconnecting.

Wayland destroys a client's objects in ascending protocol-id order, so the
`df_core` object (created during the handshake) is destroyed before the
`df_shell`/`df_layer_surface` objects. The `df_core` callback is therefore the
authoritative disconnect hook, not the layer callback.

## Decision

1. **Window teardown clears the keyboard.** `remove_window` calls
   `clear_keyboard_focus("window removed")` when the removed window was the
   active window, or when the seat still names its surface. It never leaves
   the keyboard handle on a surface that is about to be destroyed.
2. **Layer entries record their owning client at creation.** `LayerEntry`
   stores the `ClientId` captured in `df_shell.get_layer_surface`;
   `forget_client` uses that field instead of `resource.client()`.
3. **`df_core::destroyed` owns disconnect focus restore.** Before
   `forget_client`, it captures whether the client's layer held the keyboard;
   after removal it calls `restore_window_keyboard_focus()` and
   `refresh_reserved_zones()`. `DfLayerSurface::destroyed` stays the path for a
   live client that destroys one layer.
4. **The synthetic `query focus` report exposes the seat target.** A
   `keyboard none|window <id>|chrome|other|dead` line sits beside the cached
   `focus` line. `keyboard dead` means the keyboard handle still names a
   destroyed surface and is the observable the conformance suite rejects.

Rejected alternatives:

- **Rely on `DfLayerSurface::destroyed` alone.** Object-destruction order and
  `resource.client()` invalidation make it unreliable during a disconnect.
- **Refocus the next visible window on teardown.** T307's rule keeps focus
  `None` until the user clicks; teardown does not invent a new owner.
- **Only clear `active_window`.** The cache and the keyboard handle must agree;
  clearing one without the other is the bug this ADR fixes.

## Consequences

- Any new layer owner must populate `LayerEntry::client`; any teardown code
  must not depend on `resource.client()` once a client is disconnecting.
- The shell can crash while its chrome holds the keyboard and the active
  window receives keys again without a click; a restart re-creates the chrome
  without stealing that focus.
- `keyboard dead` must never appear in a conformance run; a new teardown path
  that forgets the keyboard is caught by `window_conformance` and
  `shell_protocol_conformance`.
