# 0043 — files-core keeps the `std::fs` fallback as its shipping backend; sorting is natural and stable

## Status

accepted

## Context

T-10.1a fixed the files-core model and the `DirectorySource` seam and deferred
two decisions to T-10.1b (ADR [0042](0042-files-core-streaming-listing-and-fallback.md)):
the GIO-vs-fallback backend choice and the sort order. The design
([09-files.md](../09-files.md)) intends GIO/GVfs (`trash://`, `recent://`,
`GFileMonitor`, the udisks2 volume monitor) and a single locale-aware,
numeric collation shared by every view and the portal FileChooser.

On the pinned toolchain here `pkg-config --exists gio-2.0` fails, so no GIO
backend can be linked. Adding a collation crate is also not possible: the
pinned dependency set is deliberate and the registry cache has no locale
collation crate, so a dependency would not build offline/CI.

## Decision

- **Keep `StdFsSource` as the shipping backend, still explicitly marked for
  replacement.** It stays behind the unchanged `DirectorySource` →
  `DirectoryReader` seam, resolves only `file://`, and returns
  `UnsupportedScheme` for `trash://`/`recent://`/remote schemes.
  `SANCTIONED_FALLBACK_MARKER` names GIO/GVfs and the seam; it is removed
  only when a GIO reader lands. The sanctioned degradation is local-only
  browsing, matching the track's "degrade to local-only" risk note.
- **Sorting is a first-class model concern, not a view concern.**
  `DirectoryModel` keeps arrival order in `nodes()` and maintains a separate
  sorted projection (`ordered()`, `ordered_indices()`). `set_sort` re-sorts
  what is loaded in place; each batch is merged into the projection with an
  O(n) stable merge, so a view can paint sorted from the first batch. Equal
  keys keep arrival order, and node ids are never reassigned.
- **Name collation is dependency-free natural/numeric order** (`file2` before
  `file10`), case-insensitive with a case-sensitive tie-break. It is
  deterministic but **not locale-aware**; locale-aware collation and the
  cross-locale matrix (legacy FR-5) remain a later task.
- **Folders-first defaults on and is not reversed by descending.** Keys are
  name, kind, size, and modified (`SortKey`), each with ascending/descending.

## Consequences

- The Files views (T-10.4b) and the portal FileChooser consume
  `ordered()`/`ordered_indices()` rather than sorting themselves — one
  collation, per the design's hard rule.
- Trash/Recents/volumes remain unavailable until a GIO reader replaces the
  fallback (`UnsupportedScheme`). T-10.3a's trash work must add the backend
  or explicitly degrade; it cannot assume `trash://` resolves.
- T-10.2b's optimistic operations reconcile against the same `NodeId`; the
  sorted projection rebuilds from the model, so an operation that mutates the
  node set must go through the model's apply/merge path.
- The `SortKey`/`SortDirection` `as_str`/`parse` pair is the persistence and
  bridge format; changing it is a migration, not a rename.