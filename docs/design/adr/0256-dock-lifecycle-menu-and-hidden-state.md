# 0256 — Dock lifecycle menu, hidden state, and temporary-entry removal

## Status

accepted

## Context

The Dock click tree (pinned/running/temporary/minimized/stack/Trash/divider)
and the window chooser were already in the tree, but the app context menu only
offered Keep/Remove, Options, Show All Windows, and Quit. macOS also exposes
Hide/Show and Force Quit from the Dock, and a quit removes an unpinned app's
temporary tile rather than leaving it until the compositor finishes its close
ghost. T236 completes that lifecycle surface.

Two constraints shape the implementation. First, the compositor remains the
activation authority; the Dock menu rows are requests, never local model
mutations. Second, the private protocol has no app-level hide or force-kill
request: `df_toplevel` exposes per-window `close`, `minimize`, and `unminimize`.

## Decision

- **Hide is a tracked app-level minimize.** The shell's existing Hide path
  (`hideAppWindows`, shared with the fixed application menu) minimizes every
  visible window and records the app id in `m_hiddenApps`. The Dock receives
  that set as `hiddenApps`; the app menu shows **Show** instead of **Hide**, and
  the running dot uses the inactive opacity. A per-app Show restores only the
  windows that app hid, falling back to the app's minimized set when the tracked
  ids are gone. A manual window minimize never marks the app hidden.
- **Quit closes every window through the compositor.** The row carries the app
  id, the desktop id, and whether the entry is temporary. A temporary app is
  inserted into `m_quittingApps` and the pure
  `filterDockQuittingEntries` drops its running group and minimized rows from
  the next Dock build immediately. The mark clears when the app leaves the
  projection, or after eight seconds if it refused to close (an unsaved-changes
  prompt must not hide the tile forever).
- **Force Quit signals the process the shell launched.** The shell records the
  pid returned by `QProcess::startDetached` per desktop id. Force Quit sends
  `SIGKILL` only when that pid is still alive, then asks the compositor to close
  every window; an app the shell did not launch degrades to the close request,
  which is the strongest operation the protocol offers. A stale pid is never
  signalled after a liveness check.
- **The menu's Settings row is real.** `open_dock_settings` now opens
  `settings://pane/desktop-dock` through the one Settings deep-link entry point
  instead of logging a placeholder.

## Consequences

- The Dock menu and the fixed application menu share one hidden-state source,
  so Hide from either surface dims the Dock dot and flips the same Show row.
- `filterDockQuittingEntries` is pure and unit-tested; the pid kill remains a
  thin, non-testable OS seam. T243 owns the global Force Quit list, warning,
  and confirmation; this ADR only owns the Dock's row and its process seam.
- An app that ignores a quit keeps its tile; the pending state clears after the
  fallback window, which is honest: the projection, not the request, is the
  truth.

## Amendment (2026-11) — stable Force Quit identity

The original decision recorded the `QProcess::startDetached` PID and checked it
for liveness before `SIGKILL`. Liveness does not prove the PID still belongs to
the recorded launch, and the managed path counted a detached `systemctl stop`
helper's startup as success. The Dock reliability review §5 replaced both
assumptions:

- **Direct launches record a stable handle.** `DockProcessHandle` opens a pidfd
  while the shell owns the launch (or keeps the PID plus the `/proc` start time
  when pidfd is unavailable) and revalidates immediately before signalling, so
  a recycled PID is never targeted. Handles are pruned when the instance exits.
- **Managed launches observe the stop.** The validated transient-unit name is
  retained and `systemctl --user stop` runs asynchronously; only a normal zero
  exit counts as success, and a failure is reported and retained for retry.
- **Force Quit is capability-gated.** The menu disables the row with a reason
  when neither identity is tracked, and dispatch revalidates before acting. An
  app the shell did not launch is no longer described as force-quittable when
  only a close request is possible.

## Amendment (2026-11) — quit keeps the tile until exit

The original decision removed an unpinned app's temporary entry immediately
when Quit was requested, with an eight-second fallback that made it visible
again. That optimistic removal hid the app's affordance exactly when a close
could raise an unsaved-document prompt. The Dock reliability review §6 changed
it:

- **Quit is pending, not removal.** The tile (and its minimized rows) stays
  visible and interactive with a restrained pending state until the compositor
  projection confirms the app exited. The entry model is not rebuilt for the
  pending state; it rides the per-entry phase map.
- **The pending state clears on refusal or attention.** An attention request or
  the eight-second fallback clears the pending state without removing the tile;
  the projection remains the truth.
- **Timeouts carry a request generation.** A newer quit request supersedes an
  older one, so a stale fallback timeout cannot clear the newer pending state.
- `filterDockQuittingEntries` is retired. The pending state is published through
  the per-entry phase map, and `DockQuitRequests` owns the per-app generation.
