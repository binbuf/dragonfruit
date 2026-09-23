# T-17 — files-core: Rust Browsing/Ops Library

| | |
|---|---|
| **Phase** | 3 · Flagship apps |
| **Area** | `apps/files/` (Rust core + cxx-qt bridge) |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) |
| **Blocks** | [T-18](18-files-app.md) · [T-19](19-desktop-icons.md) · [T-27](27-portal-backend.md) (FileChooser) · [T-34](34-mvp-vertical-slice-gate.md) (MVP slice) |
| **Estimate** | XL |
| **Design docs** | [09-files.md](../../design/09-files.md) |

## Summary

`files-core`: the async, headless-testable Rust **library** (not a process)
that owns the browsing model, the single operations engine, trash, recents,
mimeapps, search, and collation. One implementation serves three consumers —
browser windows, the desktop surface, and the portal FileChooser. Embedded
in Qt through a thin cxx-qt bridge; linked directly by the Rust portal
backend.

## Background

We own what users perceive as "how Finder works" — the browsing model,
view semantics, selection, drag-and-drop, operation semantics — while GVfs
provides `trash://`, `recent://`, `network://`, `admin://`, and the UDisks2
volume monitor ([09-files.md](../../design/09-files.md)). **Backend reused;
behavior ours.**

## MVP slice (for T-34)

The MVP gate needs the browsing half only: `Location`/`Node`, async streaming
listings, the one change monitor, collation, and the operations engine for
**open/rename/new-folder/trash/restore** (with optimistic UI and the
journal). It does **not** need compress/extract, aliases, `mimeapps.list`
defaults, `recent://` grouping, search, or the cxx-qt bridge's later
features — those ship post-gate. Keep the FRS-1 budgets (warm 1k folder
< 50 ms, windowed 100k list) as the MVP performance bar; everything else can
follow.

## Scope

### In scope

1. **Model**:
   - **`Location`** — URI-addressed, GFile-shaped: `file://`, `trash://`,
     `recent://`; later `smb://`/`sftp://` via GVfs.
   - **`Node`** — one listed item: stable per-session ID (survives renames
     so selection/drag state never glitches), display name, kind
     (`shared-mime-info`), themed icon, size, dates, permissions, symlink
     target, version/etag for change detection.
   - **Filenames are raw bytes**: keep the original byte string, display a
     lossy decode (replacement glyph) for non-UTF-8 names — odd names still
     list, sort, select, and trash; a rename not touching odd bytes
     preserves them.
   - **Listings are async and streaming**: views paint from first entries
     onward; **no I/O on the UI thread, ever**.
   - **One change monitor per visible directory** (GFileMonitor/inotify
     locally, GVfs change events remotely), fanned out to views; idle
     windows do **zero polling**.
   - **One collation implementation** — locale-aware, numeric mode
     (`file2` < `file10`) — icon view, list view, and chooser always agree.
2. **Operations engine** — all mutations flow through it; views never call
   `rename()`/`unlink()` directly:
   - Operations: copy, move, trash, restore, rename, duplicate,
     new-folder, make-alias, compress, extract, empty-trash,
     eject/unmount.
   - **Generated names from one helper**: New Folder → `untitled folder`,
     `untitled folder 2`, …; Duplicate → `name copy`, `name copy 2`, … —
     the same next-available-name routine everywhere.
   - **Batch model**: one operation, many items; per-item progress;
     per-item errors that don't abort the batch; "Apply to all" for
     conflicts.
   - **Conflicts**: Keep Both / Stop / Replace; folder-over-folder offers
     **Merge** (files inside apply the same policy).
   - **Undo/redo**: move, rename, trash (Put Back), duplicate,
     new-folder, paste; undoing paste trashes the pasted copies
     (Finder-correct); journal persists across restarts at least for trash
     restore.
   - **Crash consistency**: single-file writes temp-then-rename; in-flight
     copies write hidden partial files and are journaled; next launch
     cleans orphans; a SIGKILL never leaves a half-written file under its
     final name; a killed cross-volume move never loses data (copy fully,
     then delete source).
   - **Busy volumes**: unmount/eject failure reports holder processes
     where UDisks names them.
   - **Privileged paths**: unreadable locations authenticate through GVfs
     `admin://` behind polkit — no terminal dance.
   - Compress via **libarchive** (single zip); extract handles libarchive's
     family with per-entry errors and no partial output.
3. **Trash**: GVfs `trash://` consumed, never reimplemented (deletions by
   other apps appear too); Put Back from `.trashinfo`; missing origin falls
   back to destination picker; per-volume trash labeled "Trash on
   'VolumeName'".
4. **Open With / defaults**: GIO `AppInfo` only — one library owns all
   `mimeapps.list` semantics; "Always Open With" writes the default per
   spec; one-shot Open With is an override.
5. **Recents**: GVfs `recent://` (freedesktop `recently-used.xbel`);
   grouped Today / Yesterday / Previous 7 Days / earlier.
6. **Search**: MVP search-as-you-type over names in current location with
   scope switch (This Location / Home); saved searches ("smart folders",
   predicate JSON) and app-index content search are later (T-23).
7. **cxx-qt bridge**: thin — models and signals to QML; **no business logic
   crosses the seam**; the only Rust-in-Qt seam in the project.
8. **Later-feature foundations decided now**: view-state persistence schema
   covering all four view kinds from day one; Quick Look provider hooks;
   thumbnail pipeline honoring the **freedesktop thumbnail cache spec**
   (`~/.cache/thumbnails`, shared, async, cancellable, LRU-capped, never
   blocks listing); tags via `user.xdg.tags` xattrs + Files-owned registry.

### Out of scope

- The Files **UI** (T-18); desktop surface (T-19); portal (T-27).
- Any SMB/SFTP client, indexer, thumbnail daemon, file-watching daemon, or
  credential store — hard rule: **we never write these**
  ([09-files.md](../../design/09-files.md)).

## Requirements

- FR-1: Performance budgets — folder open (warm, 1k items) **< 50 ms to
  first frame** with streaming; 100k-item list windows at 60 Hz via
  windowed rendering; idle window = zero polling.
- FR-2: Ops-engine property tests: every operation journaled, undoable per
  the matrix, conflict policies per spec.
- FR-3: Crash suite: scripted SIGKILL mid-copy/move/trash recovers to a
  consistent journal state on next launch; temp-then-rename ordering is
  **asserted**.
- FR-4: Spec goldens: `.trashinfo` round-trips, `mimeapps.list`
  parse/write, `recently-used.xbel`, thumbnail cache naming.
- FR-5: Collation matrix across locales (numeric mode, case handling).
- FR-6: Name edge cases: invalid-UTF-8, overlong, FAT/NTFS-illegal names
  round-trip through model, ops journal, and trash **unchanged**.
- FR-7: Headless: model, ops, trash, mimeapps, recents unit/property
  tested with fake adapters, no display required.

## Acceptance criteria

- [ ] Full test suite green headless in CI (Phase gate for T-18/T-27).
- [ ] Performance budgets measured in the dev loop, not deferred.
- [ ] Bridge review: zero business logic in C++/QML beyond view wiring.

## Test plan

- Extensive: fuzzing (ops), goldens (specs), property tests (journal,
  collation, IDs), fake-GVfs adapters, crash scripts (SIGKILL matrix).

## Risks / open questions

- cxx-qt bridge maintenance is a standing cost — keep the seam minimal by
  review gate.
- GVfs daemon availability inside minimal VMs — degrade to local-only
  browsing with explicit "unavailable" locations, consistent with the
  graceful-degradation principle.
