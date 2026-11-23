# 0332 — Xwayland disconnect teardown follows the WM event channel

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-23 |
| **Scope** | Xwayland crash/restart cleanup, the X11 window model, and the xwayland-shell association map |
| **Contract** | [27-stability-and-daily-driver.md](../27-stability-and-daily-driver.md) §3.4 |

## Context

`space.refresh()` drops a dead `Window` from the scene but not from
`WindowModel`, which is the shell's source. An Xwayland crash therefore left
ghost entries, a stale override-redirect map, and sometimes keyboard focus on a
dead X11 surface. A restarted server reuses X11 window ids, so the next map
could match the old model entry instead of creating a new window (S-22).

Restarting on `XWaylandEvent::Error` alone is also too early. That event comes
from the Xwayland display fd and can fire while the old X11 WM source still has
queued messages. Those messages are then applied to the *new* `X11Wm` (same
XwmId lookup, reused ids), so a late association can attach the new window to
the dead server's `wl_surface`. Smithay's `xwayland_shell` state compounds this:
its serial → `wl_surface` map is never pruned when the client disconnects, so
the restarted client can look up the previous server's surface.

## Decision

- **One removal path.** `remove_all_x11_windows` routes every tracked X11
  window through `remove_window` with `WindowEventKind::Unmapped`, releases
  keyboard focus if an X11 surface owns it, unmaps and clears
  `override_windows`, and leaves the model, scene, and workspaces empty.
- **The WM channel owns the restart.** `XWaylandEvent::Error` only marks the
  server not running. When an `X11Wm` exists, the restart waits for
  `XwmHandler::disconnected`, which calloop delivers after the source's queue
  is fully drained (the source then removes itself). With no WM, `Error`
  cleans and schedules directly.
- **A fresh association map per server.** `maybe_restart` removes the old
  `xwayland_shell` global and creates a new `XWaylandShellState` before
  spawning Xwayland, so a new surface cannot be associated with a serial left
  by the dead client.
- **A reused X11 id is a new window.** With the model entry and override map
  gone, `x11_window_for` finds nothing and `Window::new_x11_window` plus
  `WindowModel::insert` assign a new compositor id.

## Consequences

- The shell receives exactly one `Unmapped` per X11 window and no ghost
  entries survive a crash; focus never stays on a dead X11 surface.
- Restart is delayed until the old WM source drains. A wedged X11 connection
  would delay the restart, but a dead Xwayland process closes that socket, so
  the drain is immediate in the crash case.
- X11 window ids are only meaningful within one server lifetime. Later X11
  work (frame extents, XDnD) must not cache an id across a restart and must
  tolerate a new `XWaylandShellState` global.
- Covered by
  `cargo test -p dragonfruit-compositor --test xwayland_conformance`
  (`xwayland_disconnect_removes_x11_windows_and_reuses_ids`,
  `xwayland_restart_after_kill_is_clean`).
