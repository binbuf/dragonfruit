# ADR 0308 — DRM screenshots use the same offscreen capture pass

- **Status:** accepted
- **Date:** 2026-11-20

## Context

ADR [0079](0079-screenshot-capture-delivery-and-save-copy.md) implemented the
trusted screenshot capture as a second offscreen render pass on the nested
backend. The DRM backend deliberately answered every `capture_screenshot`
request with `screenshot_failed("the DRM backend does not implement capture
yet")`, so the native session's Cmd+Shift+3 produced no file. The region
shortcut opened the overlay, but the subsequent capture failed on the same
path.

A native screenshot is not a separate scene. The compositor already builds the
exact element list it is about to composite for the output. Reusing that list
for an offscreen pass keeps the capture aligned with what the user sees and
avoids a client-facing grab protocol.

## Decision

1. **`capture_frame` is renderer-generic.** The nested helper now requires only
   `Renderer + Bind<GlesTexture> + Offscreen<GlesTexture> + ExportMem`, plus the
   element type's `RenderElement<R>` bound. It still renders into a
   `GlesTexture`, reads it back with `copy_framebuffer`/`map_texture`, crops,
   and produces the PNG payload.
2. **The DRM backend calls it with its live element list.** After building
   `custom_elements` for the frame and before
   `DrmCompositor::render_frame`, `render_surface` runs the same capture pass
   with the output's mode size, scale, and capture rectangle. The scanout
   render then proceeds on its own buffers.
3. **One resolve path.** The shared `resolve_capture` writes the PNG to the
   shell-supplied path and broadcasts `screenshot_saved` or
   `screenshot_failed`. Headless still fails explicitly because it has no render
   target.
4. **A hidden screenshot overlay cannot be remapped by a late GPU commit.**
   `ShellController::hideScreenshot` discards the screenshot producer's pending
   view, and the overlay's scene target reports `isCurrent` from
   `m_screenshotActive`. A pointer-move render that completes after the
   selection is accepted is therefore dropped instead of mapping the overlay
   again.

## Consequences

- Native DRM sessions can now satisfy the Cmd+Shift+3 shortcut and the
  region/window overlay through the same private capture request. The headless
  rail remains the only backend that answers `screenshot_failed`.
- The DRM capture uses the output mode's physical size and the frame's scale,
  matching the existing nested semantics. A transformed or rotated output
  still shares the pre-existing coordinate limitation documented in ADR 0079.
- This is a shared offscreen pass, not a new protocol or privilege: the
  request remains reachable only by the authenticated shell.
- Hardware evidence on the native rail still needs to be recorded; compiling
  and unit tests do not prove a particular GPU/driver path.
