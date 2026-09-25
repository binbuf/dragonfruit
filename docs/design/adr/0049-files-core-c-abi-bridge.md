# 0049 — A hand-written C ABI bridges files-core into the Qt views

## Status

accepted

## Context

T-10.4b renders the first directory listing in the Files window. `09-files.md`
and the track ([tracks/10-files-mvp.md](../tracks/10-files-mvp.md)) describe the
app embedding `files-core` through a "thin Qt model bridge (cxx-qt)". The
project has no cxx-qt (or other Qt-Rust binding) dependency, the pinned
toolchain is offline, and adding one would put Qt's build tools inside the
Cargo build for every crate. ADR
[0048](0048-files-app-shell-location-provider.md) deferred this choice to
T-10.4b, when the listing is actually needed.

The acceptance is "both views render the model and switch via the toolbar",
tested headless as "view toggle swaps delegates; selection survives".

## Decision

- **`files-core` ships a small hand-written C ABI** (`services/files-core/src/ffi.rs`,
  crate-type `["lib", "staticlib"]`). It is the only place Rust meets the Qt
  app: `df_files_begin` / `df_files_poll` / `df_files_snapshot` /
  `df_files_set_sort` / `df_files_event_free` / `df_files_free`, mirrored by
  `apps/files/ffi/files_core.h`.
- **`FilesDirectoryModel` is a thin `QAbstractListModel` facade** in
  `apps/files/` over that ABI. It owns no filesystem logic: the listing runs on
  the existing `files-core` worker thread, the facade polls the ready events
  from a `QTimer` on the Qt thread (`df_files_poll(session, 0)`), and stops as
  soon as the listing completes, so an idle window does zero polling. Sorting,
  natural collation, and stable `NodeId`s stay in Rust.
- **Each poll returns the model's current ordered snapshot.** This keeps the
  facade a pure pass-through and guarantees one collation
  ([`files-core`'s `SortSpec`]); node ids are per-listing and selection is held
  by the shell (`FilesShell.selectedId`), so a snapshot reset cannot lose it.
- **CMake builds the static library** (`cargo build -p dragonfruit-files-core`
  from an always-rerun custom target) and links it into the QML module with
  `pthread;dl;m`.
- **`DF_FILES_START_VIEW`** (`icon`/`list`) opens a fresh window in a chosen
  view for captures, sibling of `DF_FILES_START_URI`.

## Consequences

- The bridge is a static library, not a process: no D-Bus, no daemon, one
  implementation for Files, the future desktop surface, and the portal
  FileChooser.
- Whole-snapshot delivery is correct but O(n²) over a large directory. T-10.5
  owns the performance budgets and must replace it with incremental/windowed
  delivery through the same `FilesDirectoryModel` roles; the C ABI is the seam
  that changes, not the views.
- The folder watcher (`FolderWatcher`) is not wired yet: a window repaints on
  navigation but not on external change. The next Files task that mutates
  (T-10.4c) or measures (T-10.5) connects it.
- The facade is the only C++ file that includes `ffi/files_core.h`; QML and the
  rest of the app never cross the seam.