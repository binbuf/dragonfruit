# T-18 — Files Application (Browser UI)

| | |
|---|---|
| **Phase** | 3 · Flagship apps |
| **Area** | `apps/files/` |
| **Depends on** | [T-08](08-design-system.md) · [T-17](17-files-core.md) · [T-22](22-global-menu-broker.md) (menu model publication) |
| **Blocks** | Phase-3 exit (Files MVP scope) · [T-19](19-desktop-icons.md) |
| **Estimate** | XL |
| **Design docs** | [09-files.md](../design/09-files.md) · [10-design-system.md](../design/10-design-system.md) |

## Summary

The Finder-equivalent browser: windows/tabs/navigation, sidebar, icon and
list views with Finder keyboard semantics (**Return renames**), context
menus per the Finder rule, drag-and-drop with spring-loaded folders, the
unified operations/progress UI, in-app preferences, and the full menu model
published to the menu-broker.

## Background

Files is an **ordinary application** — no daemon supports it, the session
runs fine without it ([09-files.md](../design/09-files.md)). Everything
users perceive as "how Finder works" is ours; the backend is GVfs/UDisks.

## MVP slice (for T-34)

The T-34 MVP gate depends only on **FR-1** (the MVP scope above plus icon and
list views, navigation, and the single ops engine for
open/rename/new-folder/trash). The richer Finder behaviors in the scope list
— tab drag-out, proxy-icon and path-bar drag, Info inspector depth, aliases,
advanced progress UI — are post-gate. Files still launches, renders Tier-1
traffic lights, and publishes its menu model at the gate.

## Scope

### In scope

1. **Windows, tabs, navigation**:
   - Browser model: window persists while location changes; back/forward
     per tab.
   - Tabs with independent histories; drag a tab out to a new window;
     Cmd-double-click opens folder in new window/tab per preference.
   - **Proxy icon**: title-bar folder icon *is* the current location —
     dragging it moves the folder.
   - **Path bar**: clickable ancestors; drag onto ancestor to move;
     right-click ancestor → New Tab / New Window / Copy Path.
   - **Go to Folder** (Shift+Cmd+G) path-completion sheet.
   - Single instance: Dock click with no windows opens one; others ask the
     running instance over `org.dragonfruit.Files1` (`OpenPaths`,
     `RevealItems`), **D-Bus-activated** so "open folder" works from any
     app even with no Files process running.
   - **Open in Terminal** via `xdg-terminal-exec` default, fallback to
     first `TerminalEmulator`-category app — Files never picks or
     configures a terminal itself.
   - **Double-click activates; single-click selects** — deliberate Finder
     muscle memory, even though single-click-activate is common on Linux.
   - xdg-user-dirs drives real paths; sidebar shows **real directory
     names** (possibly localized); **we never rename the user's actual
     directories** — accepted Linux divergence from macOS.
   - Per-window state (geometry, tabs, scroll anchors, path) persists
     across quit/relaunch.
   - `Computer` (Shift+Cmd+C) browses `/`.
   - Designed empty/error states: empty folder (New Folder affordance),
     unreadable folder (inline error + Authenticate via `admin://` behind
     polkit), unmounted volume in history (reconnect hint), no search
     results (filter chip; Escape restores).
2. **Sidebar**: Favorites (user-ordered; defaults Recents, Desktop,
   Documents, Downloads, Pictures, Music, Videos), Locations (Home,
   Computer, Network when GVfs reports it), volumes (internal/removable
   with eject/unmount badges and mount-on-demand), later Tags.
   - Items are drop targets; dropping on a folder moves (copies across
     volumes); dropping on an unmounted volume mounts first.
   - Favorites reorder by drag, leave by drag-out/context menu, join via
     **Add to Sidebar** — same semantics as built-ins. Volumes are
     volume-monitor-owned, not hand-reorderable.
3. **Views**: MVP ships **Icon + List**; schema covers all four kinds
   (Column, Gallery) from day one.
   - Per-location view state persists: kind, sort, icon size, spacing,
     column widths, keep-arranged, selection where safe.
   - Icon: grid, per-folder size, snap or free placement, rubber-band
     selection, inline rename, **Cmd+Plus/Minus** and pinch resize
     (snapping to token scale), **Clean Up** (hides when keep-arranged on).
   - List: sortable resizable columns, disclosure triangles, inline
     rename, elision at column edge with full name in tooltip/inspector.
   - Sort: name/kind/date modified/size, folders-first; grouping later but
     group-header rendering designed now.
   - Hidden files (Cmd+Shift+.) per window, not persisted; GIO semantics
     (dotfiles, `~` backups, `.hidden`); search ignores hidden while
     hidden.
   - Labels wrap to two lines then fade-elide.
   - Status bar: item count, selection count, volume free space.
   - **Info inspector** (Cmd+I): preview, kind, size + size on disk, where
     (clickable path), created/modified, tags (later), permissions
     (display MVP, edit later); multi-selection aggregates.
   - Icon view = arrow-navigable grid; list view = AT-SPI table — keyboard
     and screen-reader use are component-level guarantees.
4. **Selection and keyboard** (Cmd=Super/Mod4, Option=Alt): the full table
   from [09-files.md](../design/09-files.md) — Open Cmd+O/Cmd+Down,
   Enclosing Cmd+Up, Back/Forward Cmd+[/], **Rename Return**, Quick Look
   Space (reserved day one), New Folder Shift+Cmd+N, Move to Trash
   Cmd+Delete, Delete Immediately Option+Cmd+Delete, Duplicate Cmd+D, Make
   Alias Cmd+L, Show Original Cmd+R, Get Info Cmd+I, selection modes,
   Go to Folder Shift+Cmd+G, View kinds Cmd+1..4, hidden toggle, icon size,
   type-ahead.
   **Return renames; opening is Cmd+O or Cmd+Down — if it ever feels like
   a bug, re-read the design doc.**
5. **Context menus** per the Finder rule (everything under the pointer):
   item / multiple items / folder / background / volume / trash-item rows
   from [09-files.md](../design/09-files.md); sensitive items (Trash,
   Delete Immediately, Eject on busy volume) carry confirmation sheets.
   Same menu engine/components as the menu bar — **no context-menu-only
   code path**.
6. **Drag and drop**: spring-loaded folders (~0.5–1 s configurable, progress
   ring, Escape backs out one level); drop targets include sidebar, path
   bar, toolbar nav buttons, window edges (auto-scroll); within-volume
   drag moves, cross-volume copies, Option forces copy, Cmd forces move;
   external drops (browser downloads) go through the same ops engine with
   the same conflict handling — no separate "download path."
7. **Aliases/symlinks**: Make Alias (Cmd+L) creates a **symlink**
   (relative when same volume, absolute otherwise, undoable) — documented
   deliberate divergence; Show Original reveals target; broken symlink
   lists with "original missing" badge, sorts by target kind; trashing a
   symlink trashes the link, never the target.
8. **Progress UI**: unified operations surface — per-operation/per-item
   progress, cancel, speed/ETA; conflicts pause with a sheet, not stacked
   modals.
9. **Preferences** (Cmd+,): Finder-style, Files state not settingsd keys —
   new windows open, show all extensions, warn before emptying Trash,
   spring-load delay, keep-arranged defaults, default search scope.
10. **Menu model**: the full File/Edit/View/Go/Window/Help menu from
    [09-files.md](../design/09-files.md) via `MenuBarMenu`, with live
    enable/disable driven by selection.

### Out of scope (Later list — recorded, not built here)

Column view, Gallery view, Quick Look overlay, preview pane, tags UI,
thumbnails (foundation in T-17), SMB/SFTP browsing, richer search,
saved searches, batch rename, Connect to Server, share extensions,
desktop icons (T-19), folder size in list view.

## Requirements

- FR-1: MVP scope (from [09-files.md](../design/09-files.md)) fully
  works: Recents/Favorites sidebar; Home/Computer; xdg-user-dirs locations;
  mounted disks + removable storage with mount-on-demand and eject/busy UX;
  Trash (Put Back, empty); icon+list views; the single ops engine (copy/
  move/trash/restore/rename/duplicate/new-folder/compress, undo/redo,
  batch+conflicts, unified progress); drag/drop rules; Open With +
  default-handler registration; basic search.
- FR-2: Perf budgets (T-17's table) verified in the nested session.
- FR-3: Phase-3 exit — Files covers MVP scope; publishes a menu model and
  renders Tier-1 traffic lights identical to Settings.
- FR-4: Keyboard walkthrough of the full shortcut table passes.
- FR-5: Optimistic rename/new-folder/trash visible within one frame,
  reconciled by monitors; failure snaps back with inline notice.

## Acceptance criteria

- [ ] MVP tree above item-by-item checked in a scripted walkthrough.
- [ ] D-Bus activation test: "open folder" from another app with Files not
      running works.
- [ ] View-state persistence round-trips across restarts.
- [ ] Phase-3 exit review with Settings (shared traffic lights, menu
      models).

## Test plan

- UI tests in the nested session (design-system gallery + scripted flows).
- Drag-and-drop matrix (spring load, sidebar/path-bar targets, cross-volume
  rules, external drops).
- Restart persistence round-trip tests.

## Risks / open questions

- Rubber-band + inline rename + optimistic-rename interactions are fiddly —
  property-test selection stability with the stable per-session Node IDs.
- Performance on giant folders depends on T-17 windowed rendering — verify
  with the 100k-item fixture.
