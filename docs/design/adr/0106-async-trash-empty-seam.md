# 0106 — The Dock's Empty Trash runs asynchronously on the TrashBridge worker

## Status

accepted

## Context

The Dock's Trash badge reads the same freedesktop store Files writes through
`files-core` ([ADR 0052](0052-dock-trash-state-from-files-core.md)). Empty
Trash originally called `df_files_trash_monitor_empty` synchronously from the
shell UI thread: a large Trash blocked the shell for the whole walk, and the
Dock could show neither a busy state nor a result. T-14.7r asks for an
asynchronous operation with a delayed busy indicator, a success result (check
plus removed count), a failure result (message plus Try Again), accessibility
announcements, and reduced-motion behaviour.

`FfiTrashMonitor` is internally synchronized (the C++ watch worker and the UI
thread already call it concurrently), so the empty may run while the watch
worker blocks. The C ABI's session/op path (`df_files_begin_empty_trash`) is for
a Files listing, not the Dock's count-only monitor.

## Decision

- **`TrashBridge` owns the operation state machine.** New `EmptyState`
  (`Idle`/`Emptying`/`Succeeded`/`Failed`), a one-shot `emptyAsync()` that
  starts a worker and returns immediately, and one-shot `emptyStarted` /
  `emptyFinished(ok, removed, error)` signals delivered on the UI thread via a
  queued call. `Succeeded` carries the removed count; `Failed` carries the
  message. The synchronous `empty()` stays for the tests and other callers.
- **One operation at a time.** A second `emptyAsync()` while `Emptying` is
  ignored. `resetEmptyState()` returns to `Idle` when the Dock dismisses the
  result.
- **The result is a one-shot signal, not persistent state.** The shell forwards
  it to the Dock through one QML function (`handleTrashEmptyResult`); the Dock
  owns the visible phase. No new settings key, no polling — the monitor's
  event-driven watch still updates the icon/badge.
- **The Dock renders a dedicated popover** (`DockTrashEmptyPopover.qml`)
  anchored to the Trash entry. The busy indicator is gated behind a short delay
  (`controls.dock.trashEmpty.busyDelay`) so a fast empty never flickers through
  it. The confirmation step stays in the Trash context menu.
- **Determinate `N of M` progress is deferred** to a files-core unit: the
  empty operation exposes no item-count progress today, so the Dock shows an
  indeterminate ring only.

## Consequences

- The shell never blocks on Empty Trash; a service-down or missing monitor is
  reported as an immediate failure, never a hang.
- The Dock's Trash state and its operation state have one owner (the bridge);
  the Dock only renders, and no second Trash reader appears (ADR 0052).
- A later determinate-progress task must first add a progress seam to
  `files-core` (a callback or a streamed count on the empty op) before the
  popover can show `N of M`; the current indeterminate ring is the fallback.