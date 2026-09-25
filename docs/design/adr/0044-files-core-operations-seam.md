# 0044 — files-core operations: a `FileOps` seam, permanent delete, one generated-name helper

## Status

accepted

## Context

T-10.2a adds the first mutations to `files-core`: rename, new folder, move,
copy, and delete. The design ([09-files.md](../09-files.md)) requires that
**all mutations flow through one operations engine** (views never call
`rename()`/`unlink()`), that generated names come from **one helper**
(`untitled folder`, `untitled folder 2`, … shared by new-folder, duplicate,
paste, and compress), and that the platform backend is reused rather than
reimplemented. GIO/GVfs is the intended backend, but its headers are absent
from the pinned toolchain, as ADR [0043](0043-files-core-fallback-is-the-shipping-backend.md)
records for listing.

T-10.2a is a primitive layer only: optimistic semantics, reconciliation, undo,
progress, conflict policy, and `trash://` are explicitly later tasks.

## Decision

- **One operations seam, `FileOps`**, mirroring the listing's
  `DirectorySource`: `rename`, `create_dir`, `copy`, `move_to`, `delete`, and
  an `exists` probe. It is `Send + Sync + 'static`, synchronous, and driven
  off the UI thread. GIO implements the same trait later; no caller changes.
- **`StdFsOps` is the shipping backend**, the same sanctioned `std::fs`
  degradation ADR 0043 already records. Only `file://` resolves; other
  schemes return `UnsupportedScheme`. Copy is recursive and recreates
  symlinks (never follows them); a cross-device move copies fully then deletes
  the source; delete is permanent and recursive.
- **`generated_name` is the one next-available-name helper.** It preserves the
  base's raw bytes (`OsString`), so generated names keep the repo-wide
  raw-byte rule. `FileOps::new_folder` is a provided method built on it, so
  every backend numbers folders identically.
- **Permanent delete, not trash.** The delete operation is immediate and
  recursive; moving to Trash is T-10.3a.
- **Conflicts are an error, not a guess.** A destination that already exists
  fails with `OperationError::AlreadyExists`; Keep Both / Replace / Merge is a
  later policy layer, not a primitive.

## Consequences

- T-10.2b wraps these calls with optimistic rendering, reconciliation,
  conflict policy, and the journal; it must not bypass `FileOps`.
- T-10.3a adds trash behind the same seam (or a sibling one) — `delete` stays
  permanent, and `trash://` still returns `UnsupportedScheme` until then.
- T-10.4's bridge marshals `Location`/`OperationError` across the Qt seam; no
  operation logic lives in C++/QML.
- Crash consistency (temp-then-rename, partial-copy journal, orphan cleanup)
  and progress/cancel are **not** in this layer; the design's crash suite is
  still owed by a later task. Cross-device move is copy-then-delete, so a
  SIGKILL between the two leaves a duplicate rather than data loss — the safe
  direction, but not yet journaled.