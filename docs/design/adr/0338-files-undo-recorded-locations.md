# 0338 — Files undo: a bounded stack of recorded locations, replayed through the one engine

## Status

Accepted (2026-11-23). Implements T331 and the Files side of
[design 27](../27-stability-and-daily-driver.md) §3.5.

## Context

T330 gave Files copy/cut/paste through the files-core engine. Rename, new
folder, paste, and Move to Trash are all reachable, but nothing takes them
back. The design's operations section promises Undo/Redo from the Edit menu
and `Cmd+Z`, and the stability gate names "undo" as a daily-driver gap. The
task's design note is the constraint: a failed undo must leave the current
state intact and say why, and an undo must not replay a stale path.

Two failure modes are easy to fall into. A shell-side stack of paths goes
stale as soon as an item is renamed or moved elsewhere, and a wrong guess can
delete or overwrite the wrong thing. A second "undo engine" that calls
`std::fs` directly would diverge from the operations engine and skip its
cross-device and trash rules.

## Decision

- **The stack lives in files-core** (`undo::UndoStack`), beside the engine it
  replays. The Files facade keeps one stack per directory session (the C ABI
  session), so navigation clears it; the stack never crosses a folder visit.
- **Every entry records locations, never names.** A rename records `from`/`to`
  `Location`s; a move records both ends; a trash records its `TrashedItem` so
  Put Back is the trash spec's own restore. Entries also carry the session row
  id they edited, so the facade updates that row after the replay instead of
  re-listing.
- **Replay goes through `FileOps`/`TrashOps` only.** Undo grows no second
  filesystem engine.
- **Check-then-act, all-or-nothing.** Every refusal condition (item gone, the
  original spot taken, the parent folder gone, a folder that has gained
  contents, a trash entry emptied) is checked before the first change, so a
  refusal leaves the state untouched and returns an `UndoConflict` carrying a
  `kind`, a user sentence, and the item. A filesystem failure part way through
  a multi-item step reverses the actions already replayed.
- **No data loss:** undoing a new folder deletes it only while it is still
  empty; undoing a copy removes only the copy.
- **Bounds and grouping:** at most 20 operations (oldest first out), and one
  clipboard paste is one step (consecutive items fold by a batch token). An
  undo pushes a matching redo entry; a new operation clears the redo tail.
- **The ABI** adds `df_files_undo` (queue on the existing operations worker),
  `df_files_await_ops` (fold the outcome on the caller's thread so the
  repainted rows land before the key or menu handler returns),
  `df_files_undo_status`/`df_files_undo_state` (availability, labels, last
  message), and `df_files_note_trash`/`df_files_put_back` for a trash the shell
  performed and for the Trash listing's Put Back.

## Consequences

- `Cmd+Z` / `Shift+Cmd+Z` are command keys in the shell's one handler; the
  content views forward a key they did not use, and a focused text field keeps
  its own `Cmd+Z`. The toolbar's Edit button opens the same rows.
- A failure is a visible sentence (`"report.txt" already exists`) both in the
  status/inline notice and in the model, and the step stays on the stack so it
  can be retried after the conflict is cleared.
- Pastes with no row in the current folder (a cut into a watched folder, or a
  paste elsewhere) record no row id; the replay still changes the filesystem
  and the row appears through the T332 watcher.
- The stacks do not persist across restarts. Trash restore remains the durable
  half (Put Back).
