# 0101 — Dock activation result and the launched child's display

## Status

accepted

## Context

The macOS-like Dock click tree is a product contract
([04-shell.md](../04-shell.md)): a click on a running app must focus/restore its
most recent window, and a click on a stopped pinned app must launch it. Two
gaps made clicks silently no-op (T-14.7g):

- `df_toplevel_manager.activate_app` was fire-and-forget. When the app had no
  window (closed between the shell's projection and the click, or an
  `app_id` that no window matches), the compositor did nothing and the shell
  could not tell "focused" from "no window", so the click died.
- The shell forces `QT_QPA_PLATFORM=offscreen` for its own chrome and the
  launch helper scrubbed it, but a child only inherited `$WAYLAND_DISPLAY`.
  When the shell was started with `--socket-name` and that variable was never
  exported, the launched client had no display and never mapped a window —
  presenting as "clicking does nothing".

## Decision

- **`activate_app` is answered, additively.** `df_toplevel_manager` v8 adds an
  `activation_result(app_id, found)` event; the compositor emits it on the
  requesting manager right after `activate_app`, with `found = 0` when no
  window was activated. The shell remembers the desktop id it asked about and,
  on `found = 0`, launches that entry (the app exited) or raises a notice — a
  click is never silent. This is the minimal result event the task allows, not
  a state machine; the compositor stays the sole activation authority.
- **Launch environment names the compositor socket.** The one launch helper
  (`appLaunchEnvironment`) additionally takes the socket the shell connected on
  (its `--socket-name`, or the resolved `$WAYLAND_DISPLAY`) and exports it as
  `WAYLAND_DISPLAY`; `QT_QPA_PLATFORM=offscreen` is still replaced with
  `wayland`. `XDG_RUNTIME_DIR` is inherited. Every launch path (Dock, menu bar
  `openApp`, Files reveal) goes through the same helper so a child always
  reaches the display the compositor serves.
- **Tap/drag arbitration shares one slop.** The entry's `TapHandler` and
  `DragHandler` both use `dockEntry.dragSlop` (8 px), so a near-stationary
  click always activates and only a genuine drag lifts into a rearrangement.

## Consequences

- `df_toplevel_manager` is v8; the `since` markers stay non-decreasing and the
  conformance client asserts both a found and a not-found reply
  (`dock_click_tree_activation_conformance`).
- Later tasks adding launch paths must use `appLaunchEnvironment` with the
  shell's `m_waylandDisplay`, not `QProcessEnvironment::systemEnvironment()`
  alone.
- The pending-activation map keyed by app id is bounded by clicks; it is not a
  queue and makes no ordering guarantee beyond "the next reply for that app id".
- The slop alone did not land a tap through the shell's offscreen injection: a
  hand-built `QMouseEvent` with no timestamp made `QQuickDragHandler` grab the
  press before the tap could complete. T-14.7x fixed the synthesis
  ([ADR 0110](0110-dock-pointer-injection-timestamps.md)) and retired the
  `DF_DOCK_ACTIVATION_FIXTURE` capture seam.