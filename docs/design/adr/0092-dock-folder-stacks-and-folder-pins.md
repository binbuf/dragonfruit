# 0092 — Dock folder entries are stacks, and any folder can be pinned as one

## Status

accepted

## Context

Clicking a folder entry on the Dock did nothing useful (the Downloads stack
rendered a cramped name inside its artwork and the click path was unclear), and
dragging a folder onto the Dock was a silent no-op. macOS treats a folder in the
Dock as a **stack**: it shows the folder's contents and opens the folder in the
file manager; any folder can be dragged into the Dock to become a stack. Our
`files-core` library already owns directory listing and folder watching (ADR
[0042](0042-files-core-streaming-listing-and-fallback.md), ADR
[0046](0046-files-core-trash-seam-and-spec-fallback.md)), and the existing
Downloads stack proves the popover pattern.

## Decision

- **A folder entry is a stack, never an app launch.** Clicking it opens its
  popover (the macOS list/fan equivalent); the popover header and the context
  menu open the folder in Files; double-click opens it in Files directly.
- **The folder name is not crammed into the artwork.** The entry renders a
  designed folder silhouette; the name appears in the hover name label
  (T-14.7i/ADR [0091](0091-dock-tahoe-floating-glass-language.md)) and is
  elided cleanly in the popover.
- **Any folder can be pinned.** A folder dropped on the Dock is pinned as a
  stack, persisted as a path list in a new settingsd-owned key
  (`dock.pinnedFolders`), listed read-only through `files-core`, and rendered
  in the right region beside the Downloads stack. A pinned folder whose path
  disappears degrades to a dimmed entry with a notice, never a crash.
- **File drops are preserved.** Files dropped on a folder stack move/copy into
  that folder (the Downloads rule generalized); a folder dropped on it moves
  the folder itself. Spring-loading opens the stack during a drag.
- **Accessibility carries state.** Names include the folder name and item
  count, and the stack is keyboard navigable like the Downloads stack.

## Consequences

- `settingsd` gains a schema key and `docs/settings-keys.md` is updated; the
  Dock never scans or watches directories itself, it consumes `files-core`.
- The Downloads stack becomes a special case of the general folder stack
  (same widget, same behavior), reducing duplicate code.
- Arbitrary folder pins are a new persisted state; the external-drop path
  gains an "application | folder | files" payload classification alongside the
  existing app-alias rule.
- Real macOS "fan" layouts are not required; the list popover is the shipping
  presentation and any fan/grid variant is future polish.