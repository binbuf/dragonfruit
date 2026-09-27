# 0105 — The Dock hands the acted-on entry's tile to the compositor at launch

## Status

accepted

## Context

ADRs [0004](0004-window-appear-origin-and-transform.md) /
[0005](0005-minimize-restore-motion-and-ghost.md) put a Dock tile hand-off in
the private protocol (`df_toplevel_manager.set_launch_origin`, additive in v4)
and made the compositor remember it, but no shell code ever called it: real
launches used the centered fallback. ADR
[0103](0103-dock-window-management-and-entry-affordances.md) recorded the gap
("the entry tile rect is never handed to the compositor"). Two questions
remained: what key the compositor stores the tile under, and in what coordinate
space the shell must send it.

The compositor's `dock_tiles` map is keyed by the window's Wayland `app_id` and
compared directly with the window's global geometry, and the tile request
carries no surface reference.

## Decision

- **The Dock reports one rect.** `Dock.qml` emits
  `entryTileRect(desktopId, x, y, w, h)` when the user activates an entry
  (before `entryActivated`) and re-emits only when a *settled* re-layout moves
  the remembered tile (suppressed while magnifying/dragging). It is one rect,
  not a per-frame stream.
- **The rect is in output coordinates.** `df_output.geometry` is now kept by
  the shell; it derives the Dock layer surface's global origin from the primary
  output's geometry, the Dock edge, and the surface thickness and sets
  `outputOriginX/Y` on the Dock item. The signal adds that origin, so the shell
  never has to know the surface-local anchor math.
- **The key is the entry's compositor `app_id`.** `dockLaunchAppId` uses
  `StartupWMClass` when present, else the desktop id without its `.desktop`
  suffix. An unresolvable key (and a missing rect) is a no-op, so the launch
  keeps the centered fallback. If a real launch ever announces an app_id that
  none of these match, the fix is an explicit resolved field on the app-index
  record, not more guessing in the shell.
- **The shell keeps a bounded map.** `DockTileRects` (capacity 32, FIFO
  eviction, refreshed in place) records `desktopId -> rect`, clamped to the
  primary output. It is cleared when the pinned set changes. The shell reads it
  and calls `set_launch_origin` immediately before spawning the child, so the
  first mapped window (and later minimize/restore, which the compositor
  remembers) uses the real icon.

## Consequences

- `set_launch_origin` gains its first caller; the compositor stays the motion
  owner and needs no protocol change.
- The launch-origin map is a cache, not a queue: a launch with no recorded rect
  or no resolvable key is the documented centered fallback, never garbage
  geometry.
- Cross-output launch geometry stays deferred (one primary output); the rect is
  clamped to it.
- A future per-entry geometry stream would replace `entryTileRect`/the map, not
  the protocol request.