# 0104 — Pinned folders are one `FolderStacks` model listed through files-core

## Status

accepted

## Context

ADR 0092 decided that any folder dropped on the Dock becomes a stack, persisted
as a path list in a settingsd-owned key, listed read-only through files-core,
and rendered with the same widget as the Downloads stack. The remaining
question was the runtime shape: who owns the path list, how the listings are
produced without adding a second directory reader, and how the built-in
Downloads member relates to the user's pins.

The shell's previous Downloads stack (`DownloadsMonitor`) listed with `QDir` and
watched with `QFileSystemWatcher`; files-core's shell-facing C ABI only exposes
the streaming **listing** slice (`files_core_list.h`), not its folder watcher.

## Decision

- `settingsd` owns `dock.pinnedFolders`: an ordered list of absolute paths,
  default empty, additive schema revision 7. The shell is the only writer.
- `FolderStacks` (`shell/src/folderstacks.{h,cpp}`) is the one model: it holds
  the ordered paths, lists each through the files-core C ABI, watches each
  folder with `QFileSystemWatcher` for change notification only, and tracks a
  per-folder new-items badge. A missing path is a `missing` entry with an empty
  listing, never a crash.
- The Downloads folder is always the first stack (the built-in default member)
  and is not removable; the user's `dock.pinnedFolders` follow it, immediately
  before the Trash.
- `moveFilesIntoFolder` (same file) is the one move helper for a drop on any
  folder stack (copy+remove across filesystems, name de-duplication).
- Drop classification in `dockdrops` becomes app alias | folder | files, with
  `PinFolder` (folder on the app region) and `MoveToFolder` (files/folder on a
  stack) actions.

Rejected: a settingsd default of the Downloads path (it would make an empty
key impossible and diverge from `dock.pinned` semantics), and a shell-side
recursive directory reader (two listing owners).

## Consequences

- Later tasks must list directories only through `FolderStacks`/files-core; a
  second `QDir` reader in the Dock is a regression.
- `dock.pinnedFolders` and `dock.pinned` stay independent: folder pins must not
  be folded into the app-pin list, or app `desktopId` semantics break.
- The `QFileSystemWatcher` is a notification seam only. When files-core exposes
  its `FolderWatcher` over the shell C ABI, `FolderStacks` should consume it and
  the Qt watcher should be removed.
- `downloadsmonitor.{h,cpp}` were retired with this change.