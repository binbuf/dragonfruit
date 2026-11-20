# 0079 — Screenshot capture delivery and save/copy

## Status

accepted

## Context

T-13.3a left the Screenshot portal as a presenter seam: the shell selects a
rectangle and emits `captureRequested`, but the pixels are not produced. The
compositor is the only process that owns the composed frame, and the shell is a
Wayland client whose chrome surfaces are painted *into* that frame. The
long-term capture path is a PipeWire stream (T-13.4b, [02-compositor.md](../02-compositor.md)),
but T-13.3b needs a single still frame now, save and copy working, and the
`check-no-capture-grab` invariant intact.

The options were: (a) a screenshot-only PipeWire stream, (b) a client-facing
grab protocol, or (c) a private request the compositor renders itself, reachable
only by the trusted portal presenter (the shell). (a) is the T-13.4b scope and
too large for one still frame; (b) is forbidden by policy and the gate.

## Decision

- **The compositor renders the still and writes a PNG to a caller-supplied
  path.** `df_toplevel_manager` grows to version 6 with the request
  `capture_screenshot(x, y, width, height, mode, path)` and the events
  `screenshot_saved(path)` / `screenshot_failed(reason)`. The request is only
  reachable after `df_core.authenticate` (the shell's launch token), so the
  presenter path is the only capture path; the gate's forbidden protocol names
  never appear.
- **`mode` disambiguates the selection.** `fullscreen` captures the whole
  output, `region` crops the rectangle, and `window` resolves the topmost window
  under the point in the compositor (the shell never receives window geometry).
  `width`/`height` <= 0 means the whole output.
- **The capture is a second, offscreen render pass of the same element list.**
  The nested and DRM backends render the frame's already-built elements into a
  `GlesTexture` (`create_buffer` + `bind`), read it back with
  `copy_framebuffer`/`map_texture`, crop, and write PNG. The shell unmaps its
  own screenshot overlay before requesting, so the scrim is not captured. On
  the nested rail the pass runs before the window render so the winit EGL
  surface is restored for the submit.
- **Save/copy is the shell's.** `ScreenshotWriter` (dockcore) writes the image
  into `Pictures/Screenshots` (`DF_SCREENSHOT_DIR` overrides it for headless
  tests and fixtures) and puts the image plus the saved `file://` URI on the
  clipboard. The shell loads the compositor's temporary PNG, delivers it, and
  answers any waiting portal request with the saved URI via
  `ScreenshotBridge::complete`.

## Update — ADR [0307](0307-screenshot-shortcut-immediate-desktop-save.md)

The local Cmd+Shift+3 fullscreen shortcut no longer opens the selection
overlay or writes to `Pictures/Screenshots`. It requests the same compositor
capture with a zero rectangle and saves the result to the XDG Desktop through
`ScreenshotWriter::deliverToDesktop()`. Portal requests and the region/window
flows keep the `Pictures/Screenshots` destination and the overlay described
above.

## Update — ADR [0308](0308-drm-screenshot-capture.md)

The capture pass is no longer nested-only. The same renderer-generic helper is
called by the DRM backend with its live element list, so the native rail can
satisfy the fullscreen and region requests. Headless remains the only backend
that answers `screenshot_failed`.

## Consequences

- The nested and DRM backends implement capture; the headless backend (no
  render target) fails the request explicitly with `screenshot_failed`, so the
  shell's presenter never hangs.
- Coordinates are output-local pixels at scale 1; a fractional-scale or
  transformed-output crop is a later refinement (the PipeWire path owns it).
- The clipboard copy uses `QGuiApplication`'s clipboard; the real Wayland
  data-device round trip is T-13.5's scope and can reuse `ScreenshotWriter`'s
  copy seam.
- T-13.4b adds the ScreenCast stream; this still-image request remains the
  screenshot capture source. T-13.7 owns the real `xdg-desktop-portal`
  frontend routing and the Flatpak walkthrough.