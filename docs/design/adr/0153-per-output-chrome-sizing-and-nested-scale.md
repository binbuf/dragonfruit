# 0153 — Per-output chrome sizing and the nested scale seam

## Status

Accepted (T-16.3b).

## Context

T-16.3a (`docs/xwayland-scaling.md`, ADR 0152) closed the integer half of the
fractional-scaling policy and explicitly deferred two things to T-16.3b:
per-surface viewport downscale, and "chrome sizing". Validating T-16.3a at an
output scale above 1 exposed the concrete chrome bug: the nested backend
created its `OutputDamageTracker` with a hardcoded `scale = 1.0`. The tracker's
static mode supplies the scale every render element's geometry is resolved
with, so at an output scale of 2 the compositor located elements in physical
pixels (`to_physical(2.0)`) but the tracker sized them in logical pixels
(`size * 1.0`). The wallpaper image therefore drew in the top-left fraction of
the output; the same mismatch affected every compositor-drawn element
(titlebars, shadows, menus).

A second gap: `df_output.set_scale` / `set_transform` / `set_mode` changed the
output's logical geometry but did not reconfigure the shell's chrome layer
surfaces. A scale change (the Displays pane, or a display-policy sync at
startup) left the menu bar and Dock sized for the old logical output, so they
were oversized or clipped.

## Decision

- **The nested backend has a scale seam.** `DRAGONFRUIT_NESTED_SCALE`
  (`backend::ENV_NESTED_SCALE`, parsed by `parse_output_scale`) sets the
  nested output's scale, default 1.0. `NestedData::ensure_tracker` rebuilds the
  damage tracker whenever the output's physical mode size or scale changes, so
  the tracker scale always matches the output. This is the chrome-sizing fix:
  every element (wallpaper, titlebars, shadows, menus, window surfaces) is
  now sized in the same physical space it is placed in.
- **Chrome reconfigures on output geometry changes.** A `df_output`
  `set_mode` / `set_scale` / `set_transform` that applies calls
  `reconfigure_layers()`: every chrome surface is sent a fresh `configure` for
  the new logical output geometry and the reserved zones are refreshed. The
  following `send_output_properties` ack carries the new geometry and scale.
- **Chrome is told the output's fractional scale.** `post_repaint` sends each
  output's visible chrome surfaces the fractional
  `wp_fractional_scale.set_preferred_scale`, so a scale-aware shell can render
  its buffers at the output resolution instead of a 1x raster the compositor
  stretches. This is the compositor half; the shell's hand-rolled Wayland
  client does not yet consume it (see Consequences).
- **X11 stays integer.** X11 surfaces keep T-16.3a's policy: they never
  receive a fractional preferred scale, and the compositor maps each surface's
  logical rect to physical pixels with the output's fractional scale. Xwayland
  derives its root window from the output's logical (xdg-output) size, so X11
  clients stay in integral X pixels and are not double-scaled.

## Consequences

- Headless gates: `shell_protocol_conformance::fractional_scale_reconfigures_chrome_to_the_logical_output`
  (a top bar re-configures from 1280x28 to 854x28 at scale 1.5),
  `xwayland_conformance::x11_fixture_at_fractional_scale_sizes_per_output`
  (the output is 854x480 logical and a 200x120 X11 window maps to 300x180
  physical), and the new `query scale` synthetic report they read.
- The nested session can now be run at a fractional scale for the live visual
  check (`DRAGONFRUIT_NESTED_SCALE`).
- **Follow-up (not in this slice):** the shell renders its QML chrome offscreen
  at a device pixel ratio of 1 and does not bind `wp_fractional_scale` /
  `wp_viewporter`. It therefore ignores the preferred scale and its chrome is
  upscaled at a fractional output scale. Making the shell render at the output
  scale (an integer buffer scale, or a fractional viewport destination) is a
  shell-side follow-up recorded in `docs/PROGRESS.md`.
- Multi-monitor outputs with different scales still choose the Xwayland
  integer scale once per session from the primary (ADR 0152); per-output
  Xwayland scale remains open.