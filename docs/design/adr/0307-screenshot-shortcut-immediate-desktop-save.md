# ADR 0307 — Fullscreen screenshot shortcut captures immediately to the Desktop

- **Status:** accepted
- **Date:** 2026-11-20

## Context

The existing Screenshot portal presenter and the shell's selection overlay
(ADR [0078](0078-screenshot-portal-and-selection-overlay.md)) were built around
one interactive surface for `fullscreen`, `region`, and `window` requests. The
desktop's fullscreen shortcut (`Cmd+Shift+3`, i.e. Super+Shift+3) opened that
overlay in `fullscreen` mode and waited for a click or Return before asking the
compositor for the frame. Portal captures and the region shortcut both land in
`Pictures/Screenshots` and copy the image to the clipboard
(ADR [0079](0079-screenshot-capture-delivery-and-save-copy.md)).

That is more ceremony than a whole-screen capture needs. The expected
behaviour for this shortcut is one key press, one saved file, no selection
surface. The reference desktop drops the file on the Desktop, where the
desktop-icon view already reflects new files.

## Decision

1. **Cmd+Shift+3 captures immediately.** `ShellController` routes the
   compositor's `screenshot` action to `captureFullscreenScreenshot()` instead
   of opening `ScreenshotBridge` in `fullscreen` mode. It calls the existing
   private `capture_screenshot` request with a zero rectangle, which the
   compositor already defines as "the whole active output". No overlay is
   mapped, so no scrim or badge is part of the frame.
2. **The local fullscreen capture saves to the Desktop.** `ScreenshotWriter`
   gains `desktopDirectory()` (XDG Desktop) and `deliverToDesktop()`. Portal
   requests and the region/window overlay keep `saveDirectory()`
   (`Pictures/Screenshots`), so an application asking for a screenshot still
   gets the documented portal location.
3. **`DF_SCREENSHOT_DIR` overrides both destinations.** The headless tests and
   capture fixtures stay deterministic without a real Desktop or Pictures
   directory. If the Desktop location is unavailable, `deliverToDesktop()`
   falls back to `saveDirectory()`.
4. **The image is still copied to the clipboard and named uniquely.** The
   timestamped `Screenshot_<date>_<time>.png` naming and the `-N` collision
   suffix are unchanged; a failed save does not copy.

## Consequences

- The fullscreen shortcut no longer has a cancel step: pressing it always
  produces a file when the backend can render, or logs the compositor's
  failure reason when it cannot. There is no user-facing failure surface yet;
  that remains open and is not silently treated as success.
- Region and window captures are unchanged: they still use the overlay and
  save to `Pictures/Screenshots`.
- The Desktop path is the XDG user directory, not a hard-coded `~/Desktop`, so
  localized or relocated desktops behave correctly.
- `tst_screenshotwriter` covers the Desktop destination alongside the existing
  save, copy, and uniqueness tests. The compositor and private-protocol
  capture path are untouched.
