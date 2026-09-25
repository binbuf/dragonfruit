# 0052 — The Dock's Trash state comes from files-core, not a shell watcher

## Status

accepted

## Context

T-10.6a makes the Dock's Trash badge read the same store Files writes. The
original shell `TrashMonitor` was an interim `QFileSystemWatcher` over
`$XDG_DATA_HOME/Trash` with its own trash/empty logic — a second Trash
implementation that could drift from `files-core`'s freedesktop `TrashOps`
([09-files.md](../09-files.md), "the Dock Trash badge watches the **same** GVfs
mount — one source of truth"). GIO/GVfs headers are not pinned (ADR
[0043](0043-files-core-fallback-is-the-shipping-backend.md)), so the shared
source has to be `files-core`'s `FreedesktopTrash`, reached from the shell
process.

## Decision

- **`files-core` owns the Trash source.** `TrashSource` is a `DirectorySource`
  over `trash://` (so Files lists the Trash through the streaming model), and
  `TrashMonitor` is the Dock's read path: a count, a reachability flag, and
  change notifications on the T-10.3b `FolderWatcher` over the store's `info/`
  directory. Both use the one `FreedesktopTrash` store.
- **A small C ABI exposes the monitor** (`df_files_trash_monitor_new/state/
  wait/refresh/trash/empty/take_error/free`), mirrored by
  `shell/src/files_core_trash.h`. `df_files_begin("trash://…")` dispatches to
  `TrashSource`; every other scheme still uses `StdFsSource`.
- **`TrashBridge` wraps the ABI** in the shell: a worker thread blocks on
  `df_files_trash_monitor_wait` and posts changes to the UI thread. `start()`
  creates the store's `files/` and `info/` directories, so a fresh account
  reads as available and empty.
- **The interim shell `TrashMonitor` is deleted**, along with its
  `QFileSystemWatcher` tests. The shell links the `files-core` static library
  (the same artifact the Files app links); the imported CMake target is defined
  once at the top level.
- **Drop-to-trash and Empty Trash are routed through the monitor's
  `trash`/`empty`** so removing the class cannot regress them; T-10.6b owns the
  fuller UX and `trash://` navigation.

## Consequences

- One Trash implementation and one store: the Dock and Files cannot disagree,
  and a deletion by a third-party application moves the badge on the
  filesystem event without a poll.
- The shell now links the `files-core` static library and its C ABI, so the
  shell process grows by the list/model code even though it only uses the trash
  slice. The ABI is additive; the trash header keeps the shell from depending
  on the listing/model symbols.
- `TrashMonitor` reports a missing store as available (it creates the spec's
  `files/`/`info/`), matching the freedesktop contract; a store whose
  directories cannot be created reads unavailable.