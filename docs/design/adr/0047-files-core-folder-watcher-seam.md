# 0047 — files-core folder watcher: an event-driven `FolderWatcher` seam with a marked inotify fallback

## Status

accepted

## Context

T-10.3b adds the folder watcher to `files-core`. The design
([09-files.md](../09-files.md)) requires **one change monitor per visible
directory** (GFileMonitor/inotify locally, GVfs change events remotely) fanned
out to views, "idle windows do zero polling", and optimistic operations
reconciled by the monitor. ADR
[0045](0045-files-core-optimistic-layer.md) recorded that the watcher would
call `confirm`/`revert`, and that a pending op must be resolved exactly once.
The track ([tracks/10-files-mvp.md](../tracks/10-files-mvp.md)) repeats the
backend risk: "GVfs/GIO headers may be absent on the dev host; the fallback is
sanctioned but must be marked for replacement, not silently shipped."

GIO/GVfs headers are not part of the pinned toolchain
(`pkg-config --exists gio-2.0` fails), the same degradation ADR
[0043](0043-files-core-fallback-is-the-shipping-backend.md) and
[0046](0046-files-core-trash-seam-and-spec-fallback.md) record.

## Decision

- **One watch seam, `FolderWatcher` → `WatchReader`, a sibling of
  `DirectorySource`.** `FolderWatcher::watch` opens a `file://` location and
  returns a reader whose `next_batch(timeout)` **blocks** for changes; a named
  worker forwards `WatchEvent`s over an `mpsc` channel, and the consumer drains
  them from its event loop (`DirectoryModel::drain_watch`). Opening runs on the
  caller's thread, so a foreign scheme or missing folder is an immediate error,
  not a silent background failure. No directory I/O runs on the UI thread, and
  an idle folder produces no work beyond the worker's cancellation poll.
- **Events are incremental, never a re-listing.** `WatchEventKind` is
  `Created`/`Modified` (each carrying the freshly stat'd single `Node`),
  `Removed { uri }`, and `Renamed { from_uri, to }`. The backend stats only the
  entry an event named. `DirectoryModel::apply_watch` folds one event:
  `Created`/`Modified` dedupe by URI (`node_id_for_uri`) and refresh rather than
  duplicate; `Removed` drops the row; `Renamed` keeps the existing node's id so
  selection and drag state survive.
- **The shipping backend is `InotifyWatcher`, marked for replacement.** It
  speaks Linux inotify — the local mechanism GIO's `GFileMonitor` wraps —
  directly over `libc`, watching one directory non-recursively.
  `SANCTIONED_WATCHER_FALLBACK_MARKER` names GIO/GVfs and the seam; a
  GFileMonitor-backed watcher slots in behind `FolderWatcher` and removes the
  marker. The watch itself is not a daemon: no process of ours runs, per the
  design's hard rule.
- **The optimistic layer reconciles through the watcher.** 
  `OptimisticModel::apply_watch` decides each pending op against the event:
  a matching target **confirms** it, a contradicting event (the create target
  vanished, or a removed item came back) **reverts** it, and then the event is
  folded. `confirm`/`revert` are idempotent-safe, so a `*_via` call that already
  resolved the op and a later watcher event can never resolve it twice.
- **`MockWatcher` is the headless fixture** of the test plan; it replays
  scripted event batches and then blocks like a real backend.

## Consequences

- T-10.4's Qt bridge drains watch events per frame; it adds no matching logic.
  T-10.4b/c's views consume model changes and need no re-listing path.
- A GFileMonitor backend must remove the marker and pass the same acceptance
  suite; remote (`GVfs`) change events arrive through the same trait.
- The watcher is non-recursive and watches one directory, matching "one change
  monitor per visible directory". A self delete/move of the watched directory
  is not surfaced yet (the watcher goes quiet); navigation re-watches, and a
  follow-up may turn that into an error state.
- Inotify needs `libc`, so the fallback is a Linux-target dependency;
  the seam and `MockWatcher` compile anywhere.