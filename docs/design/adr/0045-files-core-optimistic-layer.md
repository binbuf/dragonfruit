# 0045 — files-core optimistic layer: apply to the model now, confirm or revert

## Status

accepted

## Context

T-10.2a added the synchronous `FileOps` operations seam but deliberately left
optimistic rendering and reconciliation to T-10.2b. The design
([09-files.md](../09-files.md)) requires that **rename, new-folder, and trash
render within one frame** and are reconciled by the change monitor, with the
item snapping back if reconciliation fails. It also requires that a node's
stable per-session id survives a rename so **selection and drag state never
glitch**, and that sorting survive re-sorts. No folder watcher exists yet
(T-10.3b), so today the operation's own result is the reconciliation signal.

## Decision

- **One optimistic layer, `OptimisticModel`**, wraps a `DirectoryModel` and a
  `Selection`. It mutates the model **synchronously** in `begin_rename`,
  `begin_new_folder`, and `begin_delete`, so the next paint already shows the
  change; each returns an `OpId`.
- **The model is the source of truth during the optimistic window.** Edits are
  applied to `DirectoryModel` itself (new `insert_node`, `remove_node`,
  `restore_node`, `replace_node`, `rename_node`) rather than an overlay, so a
  view keeps one thing to render. Node ids and the sort spec are never
  renumbered by an edit; each edit re-sorts the affected node.
- **Two-step reconciliation.** `confirm(op)` retires the pending record and
  leaves the painted result; `revert(op)` restores the captured node, its
  arrival position, and its selection membership/position. The `*_via`
  convenience methods sequence "paint, run `FileOps`, confirm on `Ok`/revert on
  `Err`" in one synchronous, off-UI-thread call.
- **Selection lives in `files-core`** as an insertion-ordered set of `NodeId`.
  Because ids are stable, selection survives rename, new folder, and re-sort
  with no view fix-up; only a confirmed delete drops an id, and a revert puts
  it back. `Selection::prune` reconciles against the model after a new listing.
- **Trash reuses the delete path.** `begin_delete` is the permanent-delete
  path; T-10.3a's trash will call the same optimistic remove and reconciliation
  rather than a parallel path.

## Consequences

- T-10.3b's watcher calls `confirm`/`revert` (or a reconcile step) instead of
  the operation result; the model invariant it must preserve is "a pending op
  is eventually confirmed or reverted exactly once."
- T-10.4's Qt bridge marshals `OpId`, `NodeId`, and `Location` across the seam;
  no optimistic logic lives in C++/QML.
- Undo/redo, progress, conflict policy, the journal, and crash consistency are
  still later tasks; this layer only makes the paint-vs-reality window
  reversible. The `_via` methods are synchronous and blocking, so callers must
  run them off the UI thread, unchanged from `FileOps`.
- Pending removals store an arrival position, so reverting several at once
  should be done in reverse order for exact positions; the list is otherwise
  clamped and safe.