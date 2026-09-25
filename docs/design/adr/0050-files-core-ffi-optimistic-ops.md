# 0050 — files-core C ABI carries the optimistic operations

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Task** | T-10.4c (extends [0049](0049-files-core-c-abi-bridge.md)) |

## Context

[0049](0049-files-core-c-abi-bridge.md) put the `files-core` listing behind a
hand-written C ABI (`services/files-core/src/ffi.rs`) that the Qt
`QAbstractListModel` facade polls. It deliberately shipped only listing and
sort. T-10.4c needs rename / move-to-trash / new-folder with optimistic
rendering: the row must change **before** the real filesystem call finishes,
then confirm or snap back.

The hard rules are "no I/O on the UI thread" and "every mutation flows through
the operations engine". Those two together mean the optimistic edit and the
real operation have to be split across the C ABI: the edit on the Qt thread,
the operation on a worker.

## Decision

The FFI session owns an `OptimisticModel` and one operations worker thread,
and the ABI gains a small optimistic surface:

- `df_files_begin_rename` / `df_files_begin_new_folder` /
  `df_files_begin_trash` apply the edit to the in-memory model
  **synchronously** and enqueue the real `FileOps`/`TrashOps` call to the
  worker (`files-core-ops`). They return an operation id, already painted.
- The worker owns `StdFsOps` / `FreedesktopTrash` and sends its outcome back
  over an `mpsc` channel.
- `df_files_poll` drains outcomes first (confirm or revert), then polls the
  listing; a drained outcome is reported as a fresh `BATCH` snapshot.
- `df_files_pending_ops` lets the facade keep polling after the listing
  completes while an outcome is still owed, and `df_files_take_error` surfaces
  the last snapped-back failure for an inline notice.
- The listing session is **not** freed when the listing completes; only
  polling stops. This is what keeps sort and operations usable on a loaded
  folder (a latent T-10.4b bug: `stop()` freed the session on `DONE`).

Selection stays a Qt-side (QML) concern in this slice; the Rust `Selection`
is available but the app tracks the multi-selection in `FilesShell` because
node ids are per-listing and the views already share the shell's set.

## Consequences

- The bridge owns exactly one worker thread per open location; navigation
  drops the session (and thus `op_tx`), which ends the worker. No process, no
  daemon.
- T-10.5 can replace whole-snapshot delivery with incremental/windowed
  delivery without touching the operations surface: the operation calls and
  the role contract (`nodeId`, `name`, …) are the stable seam.
- Error text crosses the ABI as a caller-owned string
  (`df_files_take_error` / `df_files_string_free`), so a revert can show an
  inline notice.
- Optimistic rename/new-folder confirm with `retarget`, so a name the
  filesystem chose differently is corrected before the pending edit retires.