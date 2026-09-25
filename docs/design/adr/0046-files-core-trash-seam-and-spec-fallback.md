# 0046 — files-core trash: a `TrashOps` seam with the freedesktop-spec fallback

## Status

accepted

## Context

T-10.3a adds Trash to `files-core`. The design ([09-files.md](../09-files.md))
requires one trash source of truth: `trash://`, the freedesktop Trash spec's
`$XDG_DATA_HOME/Trash` plus per-volume `.Trash-$UID`, owned by GVfs and shared
with every other application. "We consume it, we do not re-implement it."
T-10.2a deliberately left `trash://` returning `UnsupportedScheme`, and
T-10.2b's `begin_delete` was built as the optimistic path trash would reuse
(ADR [0045](0045-files-core-optimistic-layer.md)).

GIO/GVfs headers are absent from the pinned toolchain (`pkg-config --exists
gio-2.0` fails), the same degradation ADR
[0043](0043-files-core-fallback-is-the-shipping-backend.md) records for
listing. Trash cannot simply be dropped, because the Dock and Files both read
it.

## Decision

- **One trash seam, `TrashOps`, a sibling of `FileOps`** (ADR
  [0044](0044-files-core-operations-seam.md)): `trash`, `restore`, `empty`,
  `entries`, and `home_trash`. `FileOps::delete` stays **permanent**; moving to
  trash is a distinct operation. A GIO backend implements the same trait later;
  no caller changes.
- **`FreedesktopTrash` is the sanctioned fallback**, marked by
  `SANCTIONED_TRASH_FALLBACK_MARKER`. It is not a behavioral degradation: it
  reads and writes the exact on-disk store GVfs uses (same directory layout,
  same `.trashinfo` format, same percent-encoded absolute `Path`, same
  per-volume rules), so trash remains one source of truth across
  applications. Replace it behind the seam when GIO is pinned.
- **The home trash is `$XDG_DATA_HOME/Trash`, else `$HOME/.local/share/Trash`.**
  An item on a different filesystem goes to that volume's `.Trash/$UID` when a
  sticky shared `.Trash` exists, otherwise `.Trash-$UID` created `0700`, so
  trash on removable media stays on the medium.
- **Names are de-duplicated** in the store with the shared `generated_name`
  helper; the original path lives only in the `.trashinfo`, so Put Back is
  exact.
- **Restore is spec-exact**: it reads `Path=` and moves the item back, failing
  with `AlreadyExists` if occupied and `NotFound` if the original parent is
  gone (the UI then offers a destination picker). `entries` skips malformed
  `.trashinfo` files rather than failing, because the store is shared.
- **The optimistic path reuses `begin_delete`** as `OptimisticModel::trash_via`,
  so Move to Trash paints within one frame and reverts on failure.
- **`DeletionDate` is UTC**, not the spec's preferred local time. It is a
  display detail; restore never reads it.

## Consequences

- T-10.3b's watcher reconciles trash like any other optimistic operation.
- T-10.6's Dock Trash badge/source reads `entries`/`home_trash`; listing
  `trash://` as a `DirectorySource` is still owed (the scheme remains
  `UnsupportedScheme` for listing until then).
- Undo/redo, the persistent journal across restarts, progress, and conflict
  policy are still later tasks; this layer is synchronous primitives plus the
  optimistic wrapper.
- A GIO backend must remove the marker and pass the same acceptance suite.