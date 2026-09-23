# T-10 — Files MVP: Browse, Open, Rename, Trash

> **Track, not a single slice.** This file is the design reference. It is executed as 14 one-session tasks: [T-10.1a](../../tasks/056-t-10.1a-files-core-streaming-listing.md) · [T-10.1b](../../tasks/057-t-10.1b-files-core-sorting-and-platform.md) · [T-10.2a](../../tasks/058-t-10.2a-files-core-operations.md) · [T-10.2b](../../tasks/059-t-10.2b-optimistic-semantics.md) · [T-10.3a](../../tasks/060-t-10.3a-files-core-trash.md) · [T-10.3b](../../tasks/061-t-10.3b-files-core-folder-watcher.md) · [T-10.4a](../../tasks/062-t-10.4a-files-window-toolbar-sidebar.md) · [T-10.4b](../../tasks/063-t-10.4b-files-list-and-icon-views.md) · [T-10.4c](../../tasks/064-t-10.4c-files-context-menus-multiselect.md) · [T-10.5](../../tasks/065-t-10.5-files-performance-budgets.md) · [T-10.6a](../../tasks/066-t-10.6a-dock-trash-source.md) · [T-10.6b](../../tasks/067-t-10.6b-dock-drop-to-trash-and-empty.md) · [T-10.6c](../../tasks/068-t-10.6c-files-navigation-and-desktop-identity.md) · [T-10.7](../../tasks/069-t-10.7-files-capture-and-acceptance-walkthrough.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 10 of 17 — the second flagship app |
| **Area** | `apps/files/` · `services/files-core` (new) · Dock Trash integration |
| **Depends on** | T-08 |
| **Blocks** | T-13 (FileChooser), T-17 |
| **Legacy detail** | [legacy/17-files-core.md](../../tasks/legacy/17-files-core.md) · [legacy/18-files-app.md](../../tasks/legacy/18-files-app.md) · [09-files.md](../09-files.md) |

## Demo

```
open Files from the Dock → a real file manager: sidebar, toolbar, list/grid
→ open a folder, scroll a large directory smoothly
→ open a file with its registered app
→ rename, new folder, move to Trash (optimistic, visible within one frame)
→ the Dock's Trash badge updates immediately; "Empty Trash" from the Dock
  empties what Files shows
→ Dock "Show in Files" reveals an app's executable; the Dock's Trash click
  opens Files at trash://
→ open a file from the Downloads stack
```

Capture: `docs/captures/t10-files.*`, plus a large-directory scroll trace.

## Why now

Files is the second flagship app and the missing half of several existing
paths: the Dock's Trash, "Show in Files", and Downloads-stack file opens all
currently degrade to a blank placeholder or `xdg-open`. Files also owns
`trash://`, which lets the Dock drop its interim home-trash watcher for the
shared source of truth. It precedes portals because the portal FileChooser
needs files-core.

## Inherited and reused

- Design-system components (`Sidebar`, `Toolbar`, `SourceList`, `SplitView`,
  `SearchField`, `ScrollView`, `Icon`, `ContextMenu`, `Dialog`).
- The Dock's Trash entry, Trash context menu/confirmation, Downloads stack,
  external-drop plumbing, and the `org.dragonfruit.Files.desktop` activation
  target.
- T-01's titlebar pattern for first-party windows.
- The `files-core` scope in `legacy/17-files-core.md` (streaming listing,
  optimistic operations, trash via the freedesktop spec/GVfs, no indexer).

## Scope

### In

1. **files-core**: streaming directory listing, sorting, file operations
   (rename, new folder, move, copy, delete), trash (freedesktop spec via
   GIO/GVfs when available; the sanctioned fallback otherwise), and a folder
   watcher. Reuse, never reimplement, the platform plumbing.
2. **Files UI (MVP scope)**: browser with sidebar (Home, Documents,
   Downloads, Trash, mounted volumes when present), toolbar (back/forward,
   view toggle, search-local), list and icon views, context menus, multi-select.
3. **Performance**: warm 1k-item folder < 50 ms to first frame with streaming
   listing; 100k-item list scrolls at 60 Hz with flat memory (windowed
   rendering).
4. **Optimistic operations**: rename/trash/new folder visible within one frame.
5. **Dock integration**: the Dock reads Trash state from the same source as
   Files; drop-to-trash and Empty Trash go through files-core/GVfs; click
   opens `trash://`; "Show in Files" reveals the path; Downloads-stack rows
   open through Files.
6. **Identity**: install `org.dragonfruit.Files.desktop` (and the Settings
   one) so the default Dock pins resolve; register
   `org.dragonfruit.Files1` for activation.

### Out / explicitly deferred

- FileChooser portal (T-13).
- Desktop icons (post-gate backlog).
- Advanced features (tabs, dual pane, batch rename, previews, network mounts)
  beyond MVP scope.
- Files preferences UI (owned by Files, not settingsd).

## Acceptance

- [ ] The demo runs and both captures are committed.
- [ ] The performance budgets are measured and recorded with raw numbers.
- [ ] The Dock and Files agree on Trash state, including a deletion made by a
      third-party app while both are open.
- [ ] Empty Trash from the Dock and from Files produce the same result.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- Unit: files-core model and operations against a temp tree; trash spec
  round-trips; watcher events.
- Headless: listing/streaming benchmarks.
- Nested: capture review; the Dock↔Files Trash walkthrough.
- Regression: T-09 Settings still opens from the Dock.

## Risks

- **GVfs/GIO headers** may be absent on the dev host; the fallback is
  sanctioned but must be marked for replacement, not silently shipped.
- **Large-directory performance** is the reason this is a slice and not a
  footnote; measure it here, not in T-16.
- **Trash ownership**: one source of truth; the Dock's interim watcher must be
  deleted in the same change.

## Hand-off

- T-13's FileChooser reuses files-core.
- T-17's loop uses Files as the second first-party app.
