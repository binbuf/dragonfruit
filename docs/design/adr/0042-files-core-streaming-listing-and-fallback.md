# 0042 — files-core is a library with a pluggable directory source; T-10.1a streams through a marked `std::fs` fallback

## Status

accepted

## Context

Files is the second flagship first-party app. The design
([09-files.md](../09-files.md)) keeps the browsing core as a **Rust library,
not a process**, shared by the Files window, the future desktop surface, and
the portal FileChooser, with GIO/GVfs as the platform backend (`trash://`,
`recent://`, the udisks2 volume monitor, change events). The track
([tracks/10-files-mvp.md](../tracks/10-files-mvp.md)) then notes a risk:
"GVfs/GIO headers may be absent on the dev host; the fallback is sanctioned
but must be marked for replacement, not silently shipped."

T-10.1a is the first slice: the streaming directory listing and the file/
folder model. Later slices add sorting (T-10.1b), operations (T-10.2), trash
(T-10.3a), and the folder watcher (T-10.3b), all on top of the same model.
The model shape, the I/O seam, and the fallback marker therefore constrain
every later slice, so they are fixed here rather than per task.

GIO headers are not part of the pinned toolchain on this host
(`pkg-config --exists gio-2.0` fails), so T-10.1a cannot link a live GIO
backend yet.

## Decision

- **The crate is `services/files-core` (`dragonfruit-files-core`), a
  library.** No binary, no daemon; the UI and portal embed it. This matches
  the track's area note and keeps `apps/files/` for the Qt app that T-10.4
  builds on the core.
- **One platform seam: `DirectorySource` → `DirectoryReader`.** All directory
  I/O sits behind `DirectorySource::open`, which returns a reader whose
  `next_batch(max)` yields [`Node`](../../../services/files-core/src/node.rs)
  batches in arrival order. A GIO/GVfs reader and the fallback implement the
  same traits; the model never names a backend.
- **Streaming runs on a worker thread, keyed by a generation.** `begin`
  clears the model, bumps the generation, and spawns a named worker that
  forwards `ListingEvent`s over an `mpsc` channel. The consumer drains them
  from its event loop (`DirectoryModel::drain`); `DirectoryModel::apply`
  rejects events from a retired generation. No directory I/O ever runs on the
  consumer/UI thread, and navigation cancels the superseded worker.
- **The model assigns one stable `NodeId` per appended node, in arrival
  order.** Nodes carry raw name bytes (`OsString`); display is a lossy decode.
  Location URIs percent-encode bytes so non-UTF-8 and space-containing names
  round-trip.
- **The T-10.1a backend is `StdFsSource`, explicitly marked for
  replacement.** `SANCTIONED_FALLBACK_MARKER` names GIO/GVfs and T-10.1b; the
  fallback resolves only `file://` and returns `UnsupportedScheme` for
  anything else, so it cannot masquerade as the real backend. T-10.1b
  finalizes the GIO-vs-fallback decision and the sort order.

## Consequences

- T-10.1b sorts a `DirectoryModel` and adds the GIO reader (or hardens and
  keeps the fallback) behind the unchanged `DirectorySource` seam.
- T-10.2's operations and T-10.3b's watcher reconcile against the same
  `NodeId` and `Node` shape; the id is stable per session, and rename
  survival is the watcher's job, not a later model rewrite.
- The Qt bridge (T-10.4) exposes the model and a poll/drain call; it adds no
  business logic, per the design's hard rule.
- Until GIO lands, Trash/Recents/volumes are unavailable in the fallback
  (`UnsupportedScheme`), which is the sanctioned "degrade to local-only
  browsing" state from the design's risk note.