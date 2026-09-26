# 0077 — The FileChooser picker: a shell overlay and a files-core browsing bridge

## Status

accepted

## Context

T-13.2a made the portal backend's presenter seam real: a request waits in
`portal::chooser::ChooserRegistry` and any process that watches the diagnostic
`org.dragonfruit.Portal1.FileChooserOpened` signal can answer it with
`CompleteFileChooser`/`CancelFileChooser` (ADR
[0076](0076-filechooser-portal-and-presenter-seam.md)). T-13.2b must supply the
actual picker: the design-system dialog that browses and returns a selection.
The task's area is `portal/ + shell/screenshot`, and every other shell surface
(the menu bar, Control Center, OSD, lock, app switcher) is a QML view rendered
by `ShellController` into a private-protocol layer surface through
`ShellProtocol`. The design also says the chooser is "Files in chooser mode over
the one `files-core` implementation" ([09-files.md](../09-files.md),
[07-system-integration.md](../07-system-integration.md)).

Two questions had to be settled: where the picker process lives, and how it
browses.

## Decision

- **The picker is a shell overlay surface, not a second process.** `ShellProtocol`
  grows a `file-chooser` layer surface (overlay layer, unanchored so the
  compositor centres it, `ON_DEMAND` keyboard, the card-sized input region) and
  `ShellController` renders `Dragonfruit.Screenshot`/`FileChooser.qml` into it —
  the same pattern as the OSD and Control Center. No compositor change: layer
  namespaces are generic, and the surface is created only while a request
  waits.
- **`ChooserBridge` is the presenter client and the browsing seam.**
  `shell/src/chooserbridge.{h,cpp}` (dockcore) watches
  `FileChooserOpened`, exposes the request and its listing to the view, and
  calls `CompleteFileChooser`/`CancelFileChooser` with the chosen `file://`
  URIs. A missing portal is a normal state (the picker never opens); `begin()`
  is the one entry point for the live signal and the `DF_CHOOSER_FIXTURE`
  capture seam.
- **Browsing is files-core, linked in-process.** The bridge lists folders
  through the C ABI (`df_files_begin`/`df_files_poll`; the listing slice is
  mirrored in `shell/src/files_core_list.h`, like the Dock's
  `files_core_trash.h`). This is the one browsing implementation Files and the
  portal's diagnostic `ListDirectory` already use, and it works without the
  portal running — so the capture fixture renders a real listing. The bridge
  never re-implements collation, folders-first, or URI normalization; the
  portal still normalizes the final selection.
- **The view is pure QML.** `FileChooser.qml` owns no D-Bus and no listing:
  `ShellController` pushes request/rows/selection into its properties and
  routes its `selectionChanged`/`entryActivated`/`accepted`/`cancelled`
  signals back through the bridge.

## Consequences

- The picker is part of the shell binary and reaches `files-core` through the
  same static library as the Dock and Files; no new process, service, or bus
  round trip is introduced for browsing.
- `DF_CHOOSER_FIXTURE` (a folder to browse; empty/`1` = home) presents the real
  picker for the live visual check with no portal or hardware, exactly like the
  OSD/lock fixtures.
- A request the portal never hands over costs nothing: the surface is created
  at startup but stays unmapped until `FileChooserOpened`.
- T-13.3a (Screenshot) and T-13.4a (ScreenCast picker) reuse the same
  `shell/screenshot` module and overlay pattern; T-13.7 still owns the real
  `xdg-desktop-portal` frontend routing and the Flatpak walkthrough.