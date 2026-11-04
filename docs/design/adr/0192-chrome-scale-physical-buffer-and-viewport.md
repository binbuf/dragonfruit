# 0192 — Chrome scale: logical surfaces, physical buffers, and one viewport path

## Status

accepted (T-21.1b)

## Context

T-16.3b made the compositor advertise the output's fractional scale to chrome
surfaces through `wp_fractional_scale_v1`, but the shell did not bind
`wp_fractional_scale_manager_v1` or `wp_viewporter`. Every chrome surface
therefore committed a 1x buffer and the compositor upscaled it: menu-bar and
Dock text, icons, hairlines, and the glass rim were soft at 1.5x/2x even though
the material path was correct. The transport rewrite (ADR 0191) keeps the
logical layer-surface contract and does not change the scale problem.

The compositor's contract is fixed: layer configure, reserved zones,
`set_panel_rect`, input regions, and pointer coordinates are logical surface
coordinates. Only the attached buffer and its `wl_surface.set_buffer_scale` /
`wp_viewport` mapping are physical.

## Decision

- **One pure `ChromeScale` helper owns the mapping.** `shell/src/chromescale.*`
  maps a logical size to `ceil(logical * scale)` physical pixels, chooses the
  commit mode, and maps pointer coordinates both ways. It has no Wayland or QML
  dependency and is unit-tested at 1x/1.5x/2x, including the 1279.5 -> 1280
  rounding case.
- **Exact integers use `wl_surface.set_buffer_scale`.** 1x commits the same
  buffer as before (scale 1, no viewport). 2x commits a 2x buffer with
  `buffer_scale=2`, so the compositor does not run a viewport on the common
  integer path.
- **Fractional scales use a physical-size buffer plus `wp_viewport`.** 1.5x
  commits `ceil(logical * 1.5)` and sets the viewport source to the whole
  buffer and the destination to the logical surface. Switching back to an
  integer unsets both viewport properties.
- **The QML scene stays logical.** The root item is scaled by the preferred
  scale with a top-left transform origin; the offscreen window is resized to
  the physical buffer. Pointer events are converted logical -> physical at the
  injection seam, so QML handlers and hit-testing keep logical coordinates.
- **Preferred scale is tracked per chrome surface.** The shell binds the
  fractional-scale manager and creates one `wp_fractional_scale_v1` and one
  `wp_viewport` per chrome surface. A `preferred_scale` change rerenders every
  mapped scene at the new physical size; the `df_output.scale` event seeds the
  first render before the first preferred-scale event arrives. Lock surfaces
  (which are not layer surfaces) follow `df_output.scale` directly.
- **Unmapped surfaces reset.** Hide paths set buffer scale 1 and unset the
  viewport, so a later configure cannot inherit stale physical geometry.
- **The Control Center fits the logical output.** At 1.5x/2x the logical output
  is shorter than the 1x design; the panel now takes
  `min(1164, output height - bar - gap)` and its content scrolls instead of
  running past the screen edge.

## Consequences

- `chrome stats` now reports the logical size, physical buffer size, integer
  buffer scale, fractional scale, and whether a viewport is active. Nested
  measurements: 1x `1920x125 -> 1920x125`, scale 1, viewport 0; 1.5x
  `1280x125 -> 1920x188`, scale 1, viewport 1; 2x `960x125 -> 1920x250`,
  scale 2, viewport 0.
- Runtime scale changes (settingsd `display.scale` / `df_output.set_scale`)
  reconfigure the chrome to the new logical size and rerender at the new
  physical resolution. The nested stills at 1x/1.5x/2x are in
  `docs/captures/t21-hidpi-*.png`.
- Screenshot selection geometry is converted logical -> physical before the
  compositor capture request; without that, a fullscreen capture at 1.5x
  cropped the top-left 1280x800 of the 1920x1200 output.
- The transport remains the T-21.1a `grabWindow()` + shm path; physical
  resolution increases the bytes per commit (1.5x Dock is 1920x188 rather than
  1280x125). T185/T186 still own pooling, damage, frame callbacks, and the GPU
  path. Multi-output chrome surfaces with different scales still share one
  shell scale; per-output shell scenes remain a follow-up.
