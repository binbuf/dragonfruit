# 0048 — The Files app shell owns a QML browsing model and a Qt location provider

## Status

accepted

## Context

T-10.4a builds the Files window, toolbar, and sidebar on top of the completed
`files-core` (T-10.1…T-10.3). The track
([tracks/10-files-mvp.md](../tracks/10-files-mvp.md)) and
[09-files.md](../09-files.md) describe the app embedding `files-core` through a
"thin Qt model bridge (cxx-qt)". That bridge does not exist yet, the project has
no cxx-qt dependency, and the views that consume the directory listing are
T-10.4b. The shell still needs real locations (the XDG Favorites and mounted
volumes), a browsing history, and per-location view state — none of which is
files-core's job.

The acceptance is "window, sidebar navigation, and toolbar work", tested
headless as "sidebar selection changes the model; toolbar history works".

## Decision

- **`apps/files` is a reusable QML module (`Dragonfruit.Files`) plus a thin
  executable**, mirroring the Settings app (ADR 0035/0036), so the app and its
  headless QML test render the same surfaces and iterate the same model.
- **`FilesBrowser` is the browsing model, in QML.** It owns the current
  `Location` URI, the per-window back/forward history (a new navigation
  truncates the redo tail), and the per-location view state (URI → `"icon"` /
  `"list"`), and exposes `navigate`/`back`/`forward`/`setView`. It is
  deliberately independent of the directory listing: T-10.4b attaches the
  `files-core` listing to `currentUri` without a UI rewrite. It is not the
  files-core model and does not reimplement it.
- **`Files` (a `FilesBridge` QML singleton) is the platform location
  provider.** It resolves the real Favorites through `QStandardPaths` (Home,
  Desktop, Documents, Downloads, Pictures, Music, Videos; only directories that
  exist ship), the mounted block volumes through `QStorageInfo` (root and
  system mounts excluded; pseudo/network filesystems deferred to the GVfs
  volume monitor), the Computer (`file:///`) and Trash (`trash://`) locations,
  and the path-bar breadcrumb. `DF_FILES_FIXTURE` selects a deterministic set
  for headless tests; `DF_FILES_START_URI` opens a specific location for
  captures.
- **The files-core bridge decision is deferred to T-10.4b**, when the listing
  is actually needed. T-10.4b records it (cxx-qt or a hand-written
  `QAbstractItemModel` facade) and feeds `FilesBrowser.currentUri`.

## Consequences

- The browsing model and location provider are headless-testable with no
  filesystem fixtures and no files-core linkage; the whole shell test is a
  QML `TestCase`.
- T-10.4b must introduce the files-core bridge and keep `FilesBrowser` as the
  navigation/history/view-state owner, not duplicate it. View state keyed by
  URI is the persistence seam named in `09-files.md#views`.
- `Files` is a Qt-level location provider, not the filesystem backend; it never
  lists or mutates files. The sidebar's mounted-volume list is a plain
  `QStorageInfo` read until the volume monitor ships, and is marked as such.
- The app shell does not read settingsd (unlike Settings, which owns
  appearance): the Files window theme is the `Theme` default until a later task
  binds it, matching the shell's other clients.