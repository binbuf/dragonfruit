# 0337 — Files copy/cut/paste: one app-level clipboard, one paste rule

## Status

Accepted (2026-11-23). Implements T330 and the Files side of
[design 27](../27-stability-and-daily-driver.md) §3.5.

## Context

Files could list, sort, rename, trash, and empty the Trash, but the Rust
operations seam's `copy`/`move_to` had no UI. The design promises Copy, Cut,
and Paste with per-item progress and a conflict policy, and the T312 gate
requires a cross-filesystem move to be all-or-nothing (ADR 0323). The shell's
Dock drop path already has that move rule in C++; Files is a separate process
that embeds `files-core`, so it cannot call the shell's `FileOperations`.

The tempting shortcut is a QML copy loop that walks the clipboard and calls
`FilesDirectoryModel` per file. That would duplicate the engine, split the
conflict policy, and leave the cross-device rule in two languages.

## Decision

1. **The clipboard is app-level state; the paste is files-core's.** A small
   QML `FilesClipboard` holds the mode (`copy`/`cut`) and `{uri, name, isDir}`
   items. It lives in the window shell, not the per-location listing session,
   so a copy survives navigation. No filesystem work happens in QML.
2. **Cut marks; paste moves.** A cut removes nothing until a paste. A cut
   pasted into the folder the item already lives in is a skip, not a move onto
   itself.
3. **One paste rule in `files-core`.** `clipboard::paste_one` resolves the
   conflict policy and calls `FileOps::copy_as` / `FileOps::move_as`. The C ABI
   `df_files_begin_paste` takes one item at a time; consecutive calls form one
   batch with per-item progress counters. One item's failure never aborts the
   rest.
4. **Three conflict policies.** `Replace` removes the existing destination
   first (folder-over-folder is a replace; merge is deferred). `Skip` leaves
   it and reports the item skipped. `KeepBoth` numbers the incoming item with
   the same `generated_name` helper as New Folder and Duplicate. The shell
   pre-flights conflicts against the current listing and opens one sheet
   (Replace / Skip / Keep Both / Cancel) for the batch.
5. **Optimistic only for the current directory.** A paste into the visible
   folder inserts a provisional row (or hides a cut source) before the worker
   runs. Confirm keeps it; failure or skip reverts it and surfaces
   `lastError`. A paste into any other location is not painted here; that
   window's listing or watcher owns it.
6. **Cross-filesystem moves reuse ADR 0323 in Rust.** `StdFsOps::move_as`
   tries one `rename(2)`; only `EXDEV` selects the copy-to-hidden-temp-sibling,
   rename, remove-source path. Any copy or rename failure removes the temp and
   leaves the source complete. A source-removal failure after the destination
   rename keeps the move successful. Permissions and mtime are best-effort.
7. **Symlinks are copied as links, never followed.** A directory copy therefore
   cannot escape the tree it was asked to copy, and a moved link does not turn
   into a copy of its target.
8. **Progress is per item.** The Rust session counts `active/total/done` plus
   copied/skipped/failed; the Qt facade exposes them and the status bar shows
   "Copying 2 of 3…", then the summary. `pasteFinished` fires once per batch so
   the shell can clear a cut clipboard and show the result.

Rejected alternatives:

- **A QML copy loop.** It would duplicate the engine and the conflict policy.
- **A second C++ operations core in Files.** The shell's core is in the shell
  process; Files embeds `files-core`, and two move implementations would drift.
- **Copy directly to the final name across filesystems.** A failure leaves a
  partial destination under the real name; the temp sibling is why a retry
  reuses the original name.
- **Follow symlinks while copying.** A copy would silently become a duplicate
  of the target tree.
- **Per-conflict modal dialogs.** The design asks for one sheet with an
  apply-to-all decision, not a stack of modals.

## Consequences

- Replace is not atomic: it deletes the destination before placing the source.
  A failure between the two leaves the destination gone, which is the same
  trade the design's Replace semantics accept; merge is deferred.
- Copying a large tree is synchronous on the files-core worker; there is no
  cancel mid-copy yet. The operation is off the UI thread and the progress
  counters still advance per item, not per byte.
- The clipboard is per-window until T333's tabs/windows share one app-wide
  clipboard; the model is already a separate object so that move is local.
- Hard links, ACLs, extended attributes, and ownership are not preserved;
  permissions and mtime are the contract, as in ADR 0323.
