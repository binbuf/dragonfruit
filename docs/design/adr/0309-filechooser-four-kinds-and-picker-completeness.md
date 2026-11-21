# 0309 — The FileChooser picker serves all four dialog kinds

## Status

Accepted (2026-11-20). Supersedes the picker UI and SaveFiles details in ADR
[0077](0077-filechooser-picker-seat.md); the presenter seam in ADR
[0076](0076-filechooser-portal-and-presenter-seam.md) is unchanged.

## Context

The portal backend (T-13.2a) registered OpenFile, SaveFile, and SaveFiles, and
the shell picker (T-13.2b) could browse and return one file. That was not a
usable file dialog:

- **SaveFiles was treated as SaveFile.** The picker asked for a filename and
  returned one path, but the portal contract is the opposite: the user chooses
  a **folder**, and the backend returns one URI per name in `options.files`.
  A caller saving several files got a single wrong path.
- **`multiple` was decoded but ignored.** Ctrl/Shift selection did not exist,
  so an OpenFile request for several files could only return one.
- **The SaveFile name was fragile.** `current_file` did not prefill the name,
  a typed absolute path was joined to the folder anyway, and an existing file
  was overwritten with no confirmation.
- **The listing blocked the shell.** The bridge polled `df_files_poll` with a
  500 ms timeout in a loop, on the shell's UI thread, and used the full-snapshot
  event instead of the incremental delta. A large folder froze every chrome
  surface until it finished.
- **The dialog opened keyboard-inert.** The surface was `on_demand`, but
  nothing asked the compositor for focus, so the first keystrokes went to the
  window underneath.
- **A second request was dropped.** `begin()` reset the live request, so a
  second application's request could never be answered.
- **The UI was a bare list.** No favorites, no clickable path, no column
  header, no hover/focus, no empty state, no New Folder, and the file rows used
  the colored place-folder artwork (invisible in some hosts) instead of the
  monochrome icon vocabulary.

## Decision

1. **One picker, four kinds.** `ChooserBridge`/`FileChooser.qml` serve:
   - `open` — files, or folders when `directory` is set, with `multiple`;
   - `save` — a folder plus a filename, with overwrite confirmation;
   - `save-files` — a destination folder. The portal appends `options.files`.
2. **SaveFiles is resolved in the portal, not the picker.** The presenter
   returns the chosen folder; `ChooserRequest::success` appends each requested
   name, takes only the final path component, and makes a name unique
   (`report (2).txt`) when the folder already contains it. The picker never
   invents a caller's filenames.
3. **SaveFile returns a canonical `file://` URI.** The typed name resolves
   against the selected folder (or the browsed folder when none is selected);
   an absolute path and `~` are accepted; `.`/`..`/trailing `/` are rejected.
   An existing file opens an inline Replace / Keep Editing confirmation.
4. **Browsing is asynchronous and incremental.** The bridge starts a
   files-core session and drains `df_files_poll_delta` from an 8 ms Qt timer,
   merging insertions by rank and preserving the selection by URI. The shell's
   event loop is never blocked, and the session stays alive after DONE so New
   Folder can paint its optimistic row.
5. **Requests queue.** A second portal request waits behind the live dialog and
   is shown when the first resolves; it is never silently dropped.
6. **The dialog is a small Files.** A 760×520 card with a favorites sidebar
   (Home, existing XDG user directories, Computer), Up plus clickable
   breadcrumbs, a New Folder action, a Name/Size column header, 28 pt rows with
   hover and a focus-ringed list, an empty/loading state, and a footer that
   names the Save destination or the folder the Choose button returns.
7. **Selection is explicit.** Click replaces, Ctrl/Super-click toggles,
   Shift-click extends from the anchor, Ctrl/Cmd+A selects all, and the arrow
   keys move the current row. Single-selection requests ignore the modifiers.
8. **The dialog takes the keyboard on open.** `ShellProtocol` gains a chooser
   keyboard grab: the surface is briefly `exclusive` so the compositor focuses
   it, then returns to `on_demand` once focus arrives, preserving click-away
   dismissal.

## Consequences

- All four portal entry points return the shape the frontend expects; the
  SaveFiles caller gets one URI per file, in order.
- A large folder streams into the dialog without stalling the menu bar, Dock,
  or any other chrome surface.
- The picker stays one QML view and one bridge; Files and the portal's
  `ListDirectory` keep using the same files-core implementation.
- Filters, `choices`, and `current_filter` remain unimplemented: the picker
  shows every visible entry. The options are still carried verbatim on the
  diagnostic signal, so a later slice can render them without a wire change.
- `portal/tests/filechooser.rs` still proves the backend round trip;
  `tst_chooser` now covers SaveFiles construction, the queue, New Folder,
  multi-selection, overwrite confirmation, and the four kinds; `tst_chooserui`
  covers the view's selection, keyboard, sidebar, breadcrumbs, New Folder,
  Replace banner, SaveFiles note, and folder-mode note.
