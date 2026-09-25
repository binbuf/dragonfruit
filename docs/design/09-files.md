# Files

## Summary

Files is the Finder-equivalent and the second flagship first-party
application. We **own the UI and semantics but do not build every filesystem
integration ourselves** — no SMB client, no SFTP client, no removable-disk
daemon, no file-watching daemon, no indexer, no thumbnailer service. Writing
those does not make the desktop more distinctive.

What we own completely is what users perceive as "how Finder works": the
browsing model, the view semantics, selection and keyboard muscle memory,
drag-and-drop behavior, and the operation semantics (copy/move/trash with
undo). **The backend is reused; the behavior is ours.**

The window chrome, sidebar, and view register is the Finder reference in
[macos-ui-inventory.md](../reference/macos-ui-inventory.md#finder-files)
(distilled from local-only macOS screenshots that never ship).

## Architecture

### One browsing implementation, three consumers

```text
dragonfruit-files (Qt Quick)        xdg-desktop-portal-dragonfruit
  windows / tabs / views               FileChooser (chooser mode)
  sidebar / chrome                              │
         │                                      │
         │        thin Qt model bridge (cxx-qt) │
         └────────────► files-core ◄────────────┘
                     (Rust library, not a process)
              model │ ops engine │ trash │ mimeapps
              recents │ search │ collation
                            │
           ┌────────────────┼─────────────────┐
        LocalFS           GIO/GVfs            UDisks2
   (GFileMonitor /    trash:// recent://    volume + mount
    inotify)          network:// admin://  lifecycle, eject,
                      automount (udisks2   busy reporting
                      volume monitor)
```

- **`files-core` is a Rust library, not a process.** It is async, event-driven,
  and headless-testable. Files embeds it through a thin C++/Rust bridge
  (cxx-qt) exposing models and signals to QML — no business logic crosses the
  seam. The portal backend (Rust) links it directly.
- **One implementation, three consumers:** browser windows, the desktop
  surface (below), and the portal's FileChooser. A second browsing
  implementation would drift on collation, sorting, trash, and bookmarks —
  and the file dialog must feel like a small Files, because on macOS the
  open/save panel *is* the file manager's.
- **GVfs is the backend, not a competitor.** `trash://`, `recent://`,
  `network://`, `admin://`, and the udisks2 volume monitor (including
  automount policy) come from GVfs's ordinary session processes. We observe
  and request; we never re-implement.

Rust for the core is not a whim: our services are already Rust, GIO/UDisks
bindings are mature (gtk-rs), and the Rust portal backend gets the core for
free. The bridge is the only Rust-in-Qt seam in the project and stays thin.

**Implementation status (T-10.1a).** The core lives in `services/files-core`
as `dragonfruit-files-core` (a library, no process). It ships the streaming
model — `Location`, `Node` with raw-byte names and a stable per-session id,
`DirectoryModel` fed by a worker thread — behind a `DirectorySource` seam.
Because GIO/GVfs headers are not part of every host's pinned toolchain, the
first slice exercises the sanctioned `StdFsSource` fallback, which is
**marked for replacement** (`SANCTIONED_FALLBACK_MARKER`, targeting T-10.1b)
rather than silently shipped; it resolves only `file://` and refuses other
schemes. See [adr/0042](adr/0042-files-core-streaming-listing-and-fallback.md).

### Model

- **`Location`** — URI-addressed, GFile-shaped: `file://`, `trash://`,
  `recent://`; later `smb://`/`sftp://` via GVfs.
- **`Node`** — one listed item: a stable per-session ID (survives renames so
  selection and drag state never glitch), display name, kind
  (`shared-mime-info`), themed icon, size, dates, permissions, symlink
  target, version/etag for change detection.
- **Filenames are raw bytes.** The model keeps the original byte string and
  displays a lossy decode (replacement glyph) when a name is not valid
  UTF-8, so odd names still list, sort, select, and trash. A rename that
  does not touch the odd bytes preserves them.
- **Listings are async and streaming.** A view paints from the first entries
  onward and fills in as the backend iterates. No I/O on the UI thread, ever.
- **One change monitor per visible directory** (GFileMonitor/inotify locally,
  GVfs change events remotely), fanned out to views. Idle windows do zero
  polling.
- **One collation implementation** — locale-aware, numeric mode (`file2`
  sorts before `file10`) — so icon view, list view, and the chooser always
  agree.

### State ownership

| State | Owner |
|---|---|
| Window/tab layout, per-location view state, favorites, tag registry | Files itself, under `$XDG_CONFIG_HOME/dragonfruit/files/` |
| Default applications | the freedesktop `mimeapps.list` files, read/written per spec — shared with every other Linux app |
| Mount state, trash contents, recents | GVfs/UDisks — we observe, never duplicate |
| Anything genuinely desktop-wide | `settingsd` (see [08-settings.md](08-settings.md)) |

Files is an **ordinary application**: no daemon of ours runs to support it,
and the session runs fine without it.

## Windows, tabs, and navigation

- **Browser model**, Finder-style: the window persists while the location
  changes within it; back/forward history is per tab.
- Tabs with independent histories; drag a tab out to a new window;
  Cmd-double-click opens a folder in a new window (or tab, per preference).
- **Proxy icon:** the title-bar folder icon *is* the current location —
  dragging it moves the folder. A Finder signature.
- **Path bar:** clickable ancestors; drag onto an ancestor to move there;
  right-click an ancestor for New Tab / New Window / Copy Path.
- **Go to Folder** (Shift+Cmd+G): a path-completion sheet.
- Single instance: activating Files with no windows opens a new one (Dock
  click); other components ask the running instance to reveal items over
  `org.dragonfruit.Files1` (`OpenPaths`, `RevealItems`), D-Bus-activated so
  "open folder" works from any app even with no Files process running.
- **Open in Terminal** (context menu / File menu) launches the
  `xdg-terminal-exec` default terminal at the current location, falling
  back to the first `TerminalEmulator`-category application — Files never
  picks or configures a terminal itself.
- **Double-click activates; single-click selects.** Deliberate Finder muscle
  memory, even though single-click-activate is common on Linux.
- xdg-user-dirs drives the real paths of Desktop / Documents / Downloads /
  Pictures / Music / Videos. The sidebar shows the **real directory names**
  (which the user may have localized); **we never rename the user's actual
  directories** — a Linux divergence from macOS we accept.
- Per-window state (geometry, tabs, scroll anchors, path) persists across
  quit and relaunch.
- `Computer` (Shift+Cmd+C) browses `/`.

Empty and error states are designed, not incidental:

| State | Treatment |
|---|---|
| Empty folder | zero-copy message with a New Folder affordance |
| Unreadable folder | inline error; Authenticate reopens via GVfs `admin://` behind polkit (prompt rendered by the shell — see [07-system-integration.md](07-system-integration.md)) |
| Unmounted volume in history | "The volume is not mounted" with reconnect hint |
| No search results | filter chip with scope; Escape restores the listing |

## Sidebar

Sections: **Favorites** (user-ordered; defaults Recents, Desktop, Documents,
Downloads, Pictures, Music, Videos), **Locations** (Home, Computer, Network
when GVfs reports it), **volumes** (internal and removable, with
eject/unmount badges and mount-on-demand), and — later — **Tags**.

- Sidebar items are drop targets: dropping on a folder moves (or copies
  across volumes); dropping on an unmounted volume mounts it first.
- Favorites reorder by drag, leave by drag-out or context menu, and any
  folder joins via **Add to Sidebar** in its context menu — added entries
  get the same drop-target and navigation semantics as the built-ins.
- Favorites and their order are Files state. Volumes are owned by the volume
  monitor and are not hand-reorderable.

## Views

Per-location view state persists: kind, sort, icon size, spacing, column
widths, "keep arranged" flag, selection where safe. **The schema covers all
four view kinds from day one**, so adding the later views never changes the
persistence format.

| View | Ships | Signature semantics |
|---|---|---|
| Icon | MVP | Grid with per-folder icon size; snap-to-grid or free placement per folder; rubber-band selection; inline rename |
| List | MVP | Sortable, resizable columns; disclosure triangles (a tree inside the flat list); inline rename |
| Column | later | Left-to-right drill-down, one folder per column; live preview column (Quick Look); per-column width persistence |
| Gallery | later | One item at full height with a filmstrip; keyboard scrubbing |

- Sort by name / kind / date modified / size (tags later), folders-first
  toggle. Grouping (kind / date / tags) is a later feature, but its
  group-header rendering is designed now.
- **Hidden files** (Cmd+Shift+.) toggle per window, not persisted. "Hidden"
  follows GIO semantics — dotfiles, `~`-suffixed backups, and `.hidden`
  entries — so we agree with every other Linux file manager, and search
  ignores hidden items while they are hidden.
- **Icon size is interactive:** Cmd+Plus / Cmd+Minus resize icons in icon
  view (a trackpad pinch does the same), snapping to the design-system
  token scale. Free-placement folders get **Clean Up** (align to grid),
  which hides itself while "keep arranged" is on.
- Icon labels wrap to at most two lines and then elide with a fade; list
  cells elide at the column edge, with the full name in the tooltip and the
  Info inspector.
- The later views are decided now because they constrain rendering and
  persistence:
  - **Column view** — one folder per column, drilling left→right; Right /
    Down opens, Left / Up ascends, type-ahead filters the focused column;
    Back / Forward replay column paths; the rightmost column hosts a live
    preview of the selection (Quick Look); spring-loading and
    drag-to-column behave exactly as in the other views.
  - **Gallery view** — one item at full height with a filmstrip below;
    Left / Right step through the folder; Space overlays Quick Look (zoom
    and pan for large media); Return still renames.
- **Status bar:** item count, selection count, volume free space.
- **Info inspector** (Get Info, Cmd+I): preview thumbnail, kind, size (and
  size on disk), where (a clickable path), created/modified, tags (later),
  permissions (display in the MVP; editing later). Multi-selection
  aggregates counts and totals.
- Icon view is an arrow-navigable grid; list view is an AT-SPI table.
  Keyboard-only and screen-reader use are component-level guarantees (see
  [10-design-system.md](10-design-system.md)).

## Selection and keyboard

The Cmd role maps to **Super/Mod4** and Option to **Alt**, system-wide (see
[02-compositor.md](02-compositor.md)). These shortcuts are the product:

| Action | Shortcut |
|---|---|
| Open | Cmd+O / Cmd+Down |
| Enclosing folder | Cmd+Up |
| Back / Forward | Cmd+[ / Cmd+] |
| **Rename** | **Return** |
| Quick Look (later) | Space (reserved from day one) |
| New folder | Shift+Cmd+N |
| Move to Trash | Cmd+Delete |
| Delete Immediately (in Trash) | Option+Cmd+Delete |
| Duplicate | Cmd+D |
| Make Alias | Cmd+L |
| Show Original (symlink) | Cmd+R |
| Get Info | Cmd+I |
| Select all / discontiguous / range | Cmd+A / Cmd-click / Shift-click |
| Go to Folder | Shift+Cmd+G |
| View kinds | Cmd+1 … Cmd+4 |
| Show / hide hidden files | Cmd+Shift+. |
| Icon size (icon view) | Cmd+Plus / Cmd+Minus; trackpad pinch |
| Type-ahead selection | type letters; repeat to cycle |

**Return renames; opening is Cmd+O or Cmd+Down.** Deliberate, documented
Finder muscle memory — if it ever feels like a bug, re-read this line.

## Context menus

The context menu depends on what is under the pointer — the Finder rule.
Everything in it routes through the same engine and components as the menu
bar; there is no context-menu-only code path.

| Target | Items |
|---|---|
| Item | Open · Open With ▸ · Quick Look (later) · Get Info · Rename · Duplicate · Make Alias · Show Original (symlinks) · Compress… · Copy · Move to Trash · Tags (later) |
| Multiple items | Open · Get Info · Compress… · Copy · Move to Trash |
| Folder | Open · Open in New Tab · Open in New Window · Open in Terminal · Get Info · Compress… · Make Alias · Copy · Move to Trash |
| Background | New Folder · Paste Item · Get Info · Sort By ▸ · Clean Up (free placement) · Show View Options · Open in Terminal |
| Volume | Open · Get Info · Eject / Unmount |
| Trash item | Open · Put Back · Get Info · Delete Immediately… |

Sensitive items (Trash, Delete Immediately, Eject on a busy volume) carry
their confirmation-sheet behavior wherever they appear.

## Drag and drop

- **Spring-loaded folders:** hover over a folder (or a list-view disclosure
  triangle) and it opens after a short delay (~0.5–1s, configurable), with a
  progress ring on the target; Escape backs out one level; drop or Escape
  ends the sequence.
- Drop targets include the sidebar, path bar, toolbar navigation buttons,
  and window edges (edge auto-scroll).
- **Within a volume, drag moves; across volumes, drag copies.** Option forces
  copy; Cmd forces move — the Finder rules.
- Drops arriving from other applications (e.g., a browser download) go
  through the same operations engine with the same conflict handling. There
  is no separate "download path" inside Files.

## Aliases and symlinks

- **Make Alias (Cmd+L) creates a symlink**, not a macOS-style alias record —
  the Linux-portable choice, documented as a deliberate divergence. The
  link is relative when link and target share a volume, absolute otherwise,
  and it is undoable like any other operation.
- **Show Original (Cmd+R)** reveals the target in its parent folder.
- A broken symlink lists normally with an inline "original missing" badge
  and sorts by its target's kind. Trashing a symlink trashes the link,
  never the target. The target path is already part of `Node`, so rendering
  costs no extra stat per cell.

## The operations engine

All mutations — copy, move, trash, restore, rename, duplicate, new-folder,
make-alias, compress, extract, empty-trash, eject/unmount — flow through one
async engine. **Views never call `rename()`/`unlink()` directly**; that is what
makes undo, progress, and conflict handling uniform.

- **Generated names come from one helper.** New Folder → `untitled folder`,
  `untitled folder 2`, …; Duplicate → `name copy`, `name copy 2`, …; the
  same next-available-name routine serves new-folder, duplicate, paste, and
  compress, so numbering is uniform everywhere.

- **Optimistic UI.** Rename, new-folder, and trash render within one frame
  and are reconciled by the change monitor; if reconciliation fails (target
  vanished), the item snaps back with an inline notice.
- **Batch model.** One operation, many items: per-item progress, per-item
  errors that do not abort the batch, and "Apply to all" for conflict
  decisions.
- **Conflicts.** Keep Both / Stop / Replace; folder-over-folder offers
  **Merge** (files inside apply the same per-file policy).
- **Undo/redo.** Move, rename, trash (Put Back), duplicate, new-folder, and
  paste are undoable; undoing a paste trashes the pasted copies,
  Finder-correct. The journal persists across restarts at least for trash
  restore.
- **Crash consistency.** Single-file writes are temp-then-rename. In-flight
  copies write hidden partial files and are journaled; the next launch
  cleans orphans. A SIGKILL never leaves a half-written file under its final
  name, and a killed cross-volume move never loses data (copy fully, then
  delete the source).
- **Progress UI.** One unified operations surface: per-operation and per-item
  progress, cancel, speed/ETA; conflicts pause the operation with a sheet
  rather than a stack of modal dialogs.
- **Busy volumes.** Unmount/eject failure reports the holder processes where
  UDisks names them; Force Unmount is explicit and confirmation-guarded.
- **Privileged paths.** No terminal dance: unreadable locations offer
  Authenticate through GVfs `admin://` behind polkit.

Compress produces a single zip via libarchive; extract handles the archive
family libarchive supports, with per-entry errors and no partial output.

## Trash

- The backend is GVfs `trash://` (freedesktop Trash spec: `$XDG_DATA_HOME/Trash`
  plus per-volume `.Trash-$UID`). **We consume it, we do not re-implement it**
  — so deletions made by other applications appear in our Trash too.
- **Put Back** restores from the `.trashinfo`'s original path; a missing
  origin falls back to a destination picker.
- Trash on removable media stays on that medium (spec behavior) and is
  labeled as such ("Trash on 'VolumeName'").
- Empty Trash is a confirmation sheet, Shift+Cmd+Delete.
- The shell's Dock Trash badge watches the **same** GVfs mount — one source
  of truth, zero coupling between Files and the shell (see
  [04-shell.md](04-shell.md)).

## Open With and default applications

- Association enumeration and defaults both come from GIO's `AppInfo` — one
  library owns all `mimeapps.list` semantics, shared with every other Linux
  application. Files never re-parses `.desktop` trees itself.
- "Always Open With" writes the default per spec; a plain Open With is a
  one-shot override. Our defaults and other apps' agree, in both directions.
- app-index is *not* involved here — it resolves window→application identity
  for the shell (see [01-architecture.md](01-architecture.md)), not file
  associations.
- Packaging registers Files as the default `inode/directory` handler (see
  [12-packaging.md](12-packaging.md)).

## Recents

- Backend is GVfs `recent://` (the freedesktop `recently-used.xbel` store) —
  other applications' recent files appear here too.
- Grouped Today / Yesterday / Previous 7 Days / earlier. Clearing Recents is
  a privacy affordance, not an afterthought.

## Search

- MVP: **search-as-you-type over names in the current location**, with a
  scope switch (This Location / Home). Live filtering inside the window;
  Escape restores the unfiltered listing.
- Later: saved searches ("smart folders") stored as predicate JSON in Files
  state, and content search delegated to the app-index search service — the
  Spotlight-equivalent (see [08-settings.md](08-settings.md)). **We never
  write an indexer**; the rule is the same as "never write an SMB client."

## Preferences

Finder-style in-app preferences (Cmd+,), persisted as Files state, not
settingsd keys — nothing here is desktop-wide: new windows open (Home /
Recents / last location), show all extensions, warn before emptying Trash,
spring-load delay, "keep arranged" defaults, default search scope.

## Later features — decided now

Recorded early because they constrain MVP rendering and persistence:

- **Column and Gallery views** — persistence schema and keyboard slots
  (Cmd+3 / Cmd+4) are reserved from day one.
- **Quick Look** — Space toggles a preview overlay inside the Files window;
  Left / Right step through the current selection while the overlay is up;
  preview providers per MIME (text, images, PDF via QtPdf, audio/video via
  QtMultimedia). System-wide Quick Look in other apps' windows is a later
  portal/service question, not a Files one.
- **Preview pane** (Cmd+Shift+P) — the same providers, pinned as an
  optional right-hand pane in the window; designed with Quick Look so the
  two never diverge.
- **Thumbnails** — implement the **freedesktop thumbnail cache spec**
  (`~/.cache/thumbnails`) so caches are shared with any spec-conforming file
  manager; async, cancellable, LRU-capped; never blocks listing; no
  thumbnails for Trash contents.
- **Tags** — `user.xdg.tags` xattrs (semicolon-separated, the convention
  other Linux file managers follow) for portability, plus our registry
  (name → color) in Files state. Tag dots are rendered in icon and list views
  from day one, so the tags UI later never re-touches the views.
- **Batch rename** — rules-based multi-rename sheet.
- **Connect to Server** (Cmd+K) — GVfs `smb://`/`sftp://` URI dialog; saved
  servers appear in the sidebar's Locations section. Credentials are
  offered to the host's Secret Service — Files never stores or caches them
  itself (see [07-system-integration.md](07-system-integration.md)).
- **Share extensions** — deferred until a desktop-level sharing story exists.

## Desktop icons (owned by Files)

The macOS model: **the file manager owns the desktop.**

- `~/Desktop` is an icon view with exactly the same core, semantics, and
  context-menu affordances as a Files window.
- It runs as **its own process** (`dragonfruit-files --desktop`), sharing
  `files-core` but not the browser process — a Files crash never takes the
  desktop, and vice versa.
- It renders on a **compositor desktop-layer surface**: the private layer
  protocol admits a fixed set of trusted session processes by launch token —
  the shell, and this surface (see [02-compositor.md](02-compositor.md)). The
  compositor provides the layer; Files owns everything on it.
- The Desktop Reveal hot corner moves windows aside to expose the background
  and its icons (see [04-shell.md](04-shell.md)). The MVP ships plain
  wallpaper.

## Menu model

Files publishes its menu via the design system's `MenuBarMenu` component (see
[06-global-menu.md](06-global-menu.md)), with live enable/disable state
driven by the selection:

```text
File    New Window Cmd+N · New Tab Cmd+T · New Folder Shift+Cmd+N ·
        Open Cmd+O · Open With ▸ · Open in Terminal · Get Info Cmd+I ·
        Rename · Compress… · Duplicate Cmd+D · Make Alias Cmd+L ·
        Show Original Cmd+R · Move to Trash Cmd+Delete · Eject · Close Cmd+W
Edit    Undo Cmd+Z · Redo Shift+Cmd+Z · Cut Cmd+X (marks; removes nothing
        until a paste) · Copy Cmd+C · Paste Cmd+V · Move to Here
        Option+Cmd+V · Select All Cmd+A
View    as Icons Cmd+1 · as List Cmd+2 · as Columns Cmd+3 · as Gallery
        Cmd+4 · Sort By ▸ · Clean Up (free placement) · Group By ▸ (later) ·
        Show Hidden Files Cmd+Shift+. · Show View Options Cmd+J ·
        Preview Pane Cmd+Shift+P (later) · Sidebar Option+Cmd+S ·
        Path Bar Option+Cmd+P · Status Bar Cmd+/
Go      Back Cmd+[ · Forward Cmd+] · Enclosing Folder Cmd+Up · Recents
        Shift+Cmd+F · Computer Shift+Cmd+C · Home Shift+Cmd+H · Documents
        Shift+Cmd+O · Downloads Option+Cmd+L · Desktop Shift+Cmd+D · Go to
        Folder Shift+Cmd+G · Connect to Server Cmd+K (later)
Window  Minimize Cmd+M · Zoom · Show Tab Bar · Bring All to Front · [tabs]
Help    Dragonfruit Files Help
```

## Performance budgets

Targets, enforced in the dev loop rather than discovered in the polish phase
(see [ROADMAP.md](../ROADMAP.md)):

| Path | Budget |
|---|---|
| Folder open, warm cache, 1k items | < 50 ms to first frame; listing streams in |
| List-view scroll, 100k items | 60 Hz via windowed rendering; flat memory |
| Rename / trash / new folder (local) | Visible within one frame (optimistic) |
| Thumbnail pipeline | Never blocks listing; cancels when items leave view |
| Idle Files window | Zero polling; wakes only on monitor events |

## Testing

- `files-core` is **headless**: model, ops engine, trash, mimeapps, and
  recents are unit- and property-tested with fake adapters, no display
  required.
- **Ops fuzzing:** scripted SIGKILL mid-copy/move/trash must recover to a
  consistent journal state on next launch; temp-file/rename ordering is
  asserted, not hoped.
- **Spec goldens:** trash (`.trashinfo` round-trips), `mimeapps.list`
  parsing/writing, `recently-used.xbel`, thumbnail cache naming.
- **Collation matrix** across locales (numeric mode, case handling).
- **Name edge cases:** invalid-UTF-8, overlong, and FAT/NTFS-illegal names
  round-trip through model, ops journal, and trash unchanged.
- UI tests run in the nested session; view-state persistence round-trips
  across restarts (see
  [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md)).

## MVP scope

```text
Files
├── Recents / Favorites (sidebar)
├── Home / Computer
├── Desktop, Documents / Downloads (xdg-user-dirs)
├── mounted disks, removable storage (UDisks mount-on-demand, eject/busy UX)
├── Trash (freedesktop, Put Back, empty)
├── icon view + list view
├── file operations (one engine: copy / move / trash / restore / rename /
│   duplicate / new-folder / compress, undo/redo, batch + conflicts,
│   unified progress)
├── drag/drop (spring loading, sidebar/path-bar targets, cross-volume rules)
├── Open With (mimeapps) + default-handler registration
└── basic search (live name filter in location, scope switch)
```

## Later

```text
Later
├── column view
├── gallery view
├── Quick Look
├── preview pane (Cmd+Shift+P)
├── tags (xattr + registry)
├── thumbnails (freedesktop cache)
├── SMB/SFTP shares (GVfs — never ours)
├── richer search/indexing (app-index)
├── saved searches (smart folders)
├── batch rename
├── connect to server
├── share extensions
├── desktop icons (own process, desktop-layer surface)
└── folder size calculation in list view
```

## Relationships

- The portal's **file chooser** is Files in chooser mode over the same
  `files-core` (see [07-system-integration.md](07-system-integration.md));
  the portal frontend handles document-store export for sandboxed callers.
- Files publishes a first-party **menu model** to the menu-broker for perfect
  global-menu behavior (see [06-global-menu.md](06-global-menu.md)).
- Traffic lights are exactly the design system's `TitleBar`/`TrafficLights`
  components (see [10-design-system.md](10-design-system.md)).
- The **Dock Trash badge** and Files watch the same GVfs trash mount — no IPC
  between them (see [04-shell.md](04-shell.md)).
- **Desktop icons** are Files, on a trusted desktop-layer surface (see
  [04-shell.md](04-shell.md) and [02-compositor.md](02-compositor.md)).
- Mounts are requested on demand through UDisks, with unmount/eject feedback
  and busy-state handling.

## Hard rules

- Every mutation flows through the operations engine — views never call
  rename/unlink directly.
- No I/O on the UI thread; optimistic rendering reconciled by monitors.
- Files is an ordinary application — no daemon of ours supports it, and the
  session runs fine without it.
- One browsing implementation: browser windows, chooser, and desktop share
  `files-core` and its semantics. Anything that would fork them does not
  ship.
- We never write: an SMB/SFTP client, an indexer, a thumbnail daemon, a
  file-watching daemon, or a credential store.
- **Return renames.** That is not a bug.
