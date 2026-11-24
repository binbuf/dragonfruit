# ADR 0339 — The Files watcher folds bursts into one delta, not one repaint

**Status:** Accepted (T332, 2026-11).

## Context

T-10.3b landed the files-core `FolderWatcher` seam and its inotify fallback,
but the C ABI session never opened a watch, `OptimisticModel` was used only in
`watch-in-progress` text, and no C++ code applied a watch event: a file another
process created, deleted, renamed or moved did not appear until the user
reloaded. The obvious wiring — forward every kernel event — is wrong in two
ways. A `cp -r` or an archive extraction emits thousands of events, and one
model reset (or one `dataChanged` per file) per event stalls the window; and a
kernel rename arrives as `IN_MOVED_FROM` + `IN_MOVED_TO` (and is sometimes
split across `read(2)` calls), so pairing must not be assumed at the model
boundary.

The existing incremental ABI had one shape per poll: a reset or a set of
insertions. A rename that keeps its node id is neither — the row is already
painted, and re-sending it as a new row loses the selection.

## Decision

1. **One watch per session, on the session generation, opened after the
   listing.** `df_files_begin` opens a watch for a local `file://` listing
   (not `trash://`, not the synthetic fixture) with the generation
   `OptimisticModel::begin` assigned. A folder that cannot be watched is a
   static listing, never an error. The watch reports `IN_ISDIR`, so a new empty
   folder arrives as a folder.
2. **The worker coalesces.** `WatchHandle` carries a `WatchBatch` stream:
   after the first change the worker keeps draining the backend for
   `WATCH_COALESCE_WINDOW` (50 ms) and sends one batch per burst. The
   per-event stream stays for existing callers. The loop remains
   event-driven; an idle folder does zero I/O.
3. **One delta per burst.** The C ABI folds the batch into the session model
   and answers with a delta kind: `INSERT` (rows at final ranks),
   `CHANGED` (held rows replaced in place, ids unchanged, ranks carried), or
   `REMOVED` (a row vanished; the receiver re-reads the ordered snapshot as a
   reset, because a deletion shifts every later rank). `FilesDirectoryModel`
   repaints exactly the changed rows, so a rename keeps the selection, which
   is keyed by node id.
4. **No I/O and no busy poll on the UI thread.** The watcher runs on its own
   thread; the facade's poll is one blocking delta request per timer tick and
   never touches the directory itself. `df_files_watching` tells the facade
   whether the folder is watched: a watched, settled window keeps the poll
   armed (woken by the watcher's channel), an unwatched `trash://` listing
   still stops its timer.
5. **A session does not report its own writes.** The rows of in-flight
   operations are remembered, and a per-item paste's unstarted write counts as
   a quiet write; only those wake a dormant window. The pending operation is
   what wakes it otherwise.

## Consequences

- An external create, delete, rename or move paints without a manual refresh;
  a burst costs one paint.
- The delta ABI grows a `kind` plus a `changes` array (`ADR 0051`'s `reset`
  flag is replaced); every consumer (`FilesDirectoryModel`, `ChooserBridge`,
  the shell's listing slice header) is updated in the same change.
- A recursive watch, a remote (GVfs) watch, and live `trash://` refresh stay
  out: the seam accepts them without a model or view change.
