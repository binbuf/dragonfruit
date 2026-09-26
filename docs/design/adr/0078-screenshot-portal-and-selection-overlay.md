# 0078 — The Screenshot portal and the shell selection overlay

## Status

accepted

## Context

T-13.3a adds the fourth concrete portal backend interface,
`org.freedesktop.impl.portal.Screenshot`, and the selection UI the track's demo
names: Cmd+Shift+3/4 and region/window/fullscreen selection. Like the
FileChooser (T-13.2a/ADR [0076](0076-filechooser-portal-and-presenter-seam.md)
and T-13.2b/ADR [0077](0077-filechooser-picker-seat.md)) the standard method is
*interactive*: it does not return until a presenter has produced a capture. The
capture path is portal-only by policy
([02-compositor.md](../02-compositor.md)); no arbitrary-grab protocol exists.
Save/copy of a captured image is explicitly T-13.3b.

Two questions had to be settled: how a request selects region/window/fullscreen,
and where the selection UI and its presenter client live.

## Decision

- **The Screenshot portal reuses the FileChooser's presenter registry shape.**
  `portal::screenshot` owns a pure `ScreenshotRegistry` of live requests and a
  one-shot completion; the synchronous `Screenshot` method registers a request
  and awaits. A presenter resolves it with `CompleteScreenshot(handle, uri)` or
  `CancelScreenshot(handle)` on the diagnostic `org.dragonfruit.Portal1`
  surface; a `ScreenshotOpened(handle, mode, app_id, parent_window, options)`
  signal announces a waiting request. The result URI is normalized to a
  canonical `file://` URI through files-core, exactly as the chooser does.
- **The mode is a Dragonfruit option extension.** The standard options
  (`modal`, `interactive`, `handle_token`) do not name the selection; that is
  the presenter's choice. A `mode` string option
  (`fullscreen` / `region` / `window`) lets the desktop shortcut path and the
  headless tests request one explicitly. Absent it, a request is `fullscreen`,
  or `region` when `interactive` — so a standard frontend that never sends
  `mode` still behaves sensibly.
- **The overlay is a shell surface, not a second process.** `ShellProtocol`
  grows a full-output `screenshot` overlay layer surface (all four anchors,
  `exclusive_zone = -1`, `ON_DEMAND` keyboard);
  `ShellController` renders `Dragonfruit.Screenshot`/`SelectionOverlay.qml`
  into it and routes pointer/keyboard input. No compositor change: layer
  namespaces are generic.
- **`ScreenshotBridge` is the presenter client.** It watches
  `ScreenshotOpened`, opens the overlay in the request's mode, and hands the
  chosen rectangle to the capture seam (`captureRequested`) and the URI back to
  the portal (`complete`). The desktop's own shortcut calls `beginLocal` —
  the same view, no portal request. A missing portal is a normal state.
- **The shortcut is split at the compositor.** Cmd+Shift+3 is
  `InputAction::Screenshot` (fullscreen) and Cmd+Shift+4 is
  `InputAction::ScreenshotRegion`; both already flowed through the one shortcut
  engine and are routed to the shell as `screenshot` / `screenshot-region`.

## Consequences

- `model::BACKEND_INTERFACES` and `dragonfruit.portal` now list
  `org.freedesktop.impl.portal.Screenshot`; ScreenCast appends its own in
  T-13.4a. The backend name, path, and data-file identity are unchanged.
- The actual framebuffer grab and file write are not in this task: the shell
  emits `captureRequested` and T-13.3b turns it into a saved image and calls
  `ScreenshotBridge::complete(uri)`. Until then a *portal* request stays with
  the presenter seam; the desktop shortcut closes the overlay. This is the
  "save/copy is T-13.3b" boundary the task states.
- `portal/tests/screenshot.rs` proves each mode's blocking round trip with a
  test client as the presenter; `tst_screenshot` proves the shell bridge and
  `tst_screenshotui` the view. `make check-no-capture-grab` stays green (no new
  grab path).
- T-13.7 still owns the real `xdg-desktop-portal` frontend routing and the
  Flatpak walkthrough.