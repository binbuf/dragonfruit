# Xwayland Fractional Scaling Policy

Decision record for the T-06 open question ("scale-viewport per X11 surface
vs integer scale"), from
[../docs/tasks/legacy/06-xwayland.md](../docs/tasks/legacy/06-xwayland.md). The design
direction is `viewporter` + `fractional-scale`
([../docs/design/02-compositor.md](../docs/design/02-compositor.md)),
which is a Wayland-client mechanism; X11 clients cannot opt in.

## Decision

**Integer-scaled Xwayland + per-surface viewport downscale.**

1. Xwayland runs at an integer scale equal to
   `ceil(primary_output_scale)`, clamped to at least 1. X11 clients then
   render into an integer coordinate space they understand, and the X
   server's DPI/scale is never fractional. The compositor drives this
   through the integer `wl_output.scale` it advertises (Xwayland's own
   `-hidpi` is integer-only **and** rootful-only, so it is not the
   mechanism for a rootless session).
2. The compositor maps each X11 toplevel to the output's **logical** size
   when it composites: every surface is resolved against the output's
   fractional scale (`logical * scale` physical pixels). The integer scale
   keeps Xwayland's coordinate space integral; the fractional output scale
   is applied once, by the compositor, on the way to the framebuffer.
3. `wp_fractional_scale_v1` is never advertised to Xwayland; it is a
   Wayland-client protocol, and T-16.3a's policy keeps X11 metrics integral.
   The compositor maps the surface itself, so no fractional metric reaches
   an X11 client.

## Why not the alternatives

- **Nearest integer scale (round to 1x or 2x).** At 1.5x, rounding to 1x
  makes X11 apps and their text too small; rounding to 2x makes them too
  large. Both are visibly wrong next to Wayland apps on the same output.
- **Fractional `-scale` for Xwayland.** Not supported by Xwayland (integer
  `-scale` only), and even where patched it makes X11 metrics fractional,
  which breaks applications that assume integer pixels.
- **Per-app scale heuristics.** Rejected: the compositor is the sole owner
  of output scale; per-app special cases are the T-30 zoo's problem, not
  the foundation's.

## Trade-offs and follow-ups

- X11 popups, menus, and override-redirect windows are composited with the
  same output-scale mapping as their toplevel, so their offsets stay
  consistent.
- The integer scale is chosen once per Xwayland session from the primary
  output; multi-monitor outputs with different scales need a per-output
  decision (run Xwayland at the largest, downscale on the smaller). That
  per-output decision is still open; the render path already resolves each
  surface against its own output's fractional scale.

## Current state

Both halves are built. The compositor chooses the Xwayland integer scale once
per session as `ceil(primary_output_scale)`, clamped to at least 1, and records
it in `XwaylandState::integer_scale` (`compositor/src/xwayland.rs`). Smithay
advertises that same value as the output's integer `wl_output.scale`
(`Scale::integer_scale`), and the compositor never sends an X11 window a
fractional `preferred_scale` (`compositor/src/render.rs`:
`accepts_fractional_scale` skips X11 surfaces in `post_repaint`), so X11
clients only ever see the integer coordinate space and consistent integer
metrics.

Xwayland derives its root window from the output's **logical** size, which
Smithay computes from the physical mode and the *fractional* output scale
(`Space::output_geometry`). At output scale 1.5 the X screen is therefore
853x480 (`1280/1.5`, `720/1.5`), and X11 clients keep integral X pixels: they
are never double-scaled by Xwayland. The compositor maps each surface's
logical rectangle to physical pixels with the output's fractional scale, so a
200x120 X11 window is composited into 300x180 physical pixels (`logical *
1.5`), not 400x240 (an integer 2x footprint). This is the per-surface
downscale, and it is verified by
`xwayland_conformance::x11_fixture_at_fractional_scale_sizes_per_output` and
the synthetic `query scale` report. The integer-scale fixture
(`xwayland_conformance::x11_fixture_at_integer_scale_two`) still pins the
integer coordinate space.

The nested session can boot at a fractional scale with
`DRAGONFRUIT_NESTED_SCALE` (T-16.3b); its render path sizes chrome elements in
physical pixels at every output scale. The wallpaper top-left bug found during
T-16.3a is fixed (the nested damage tracker now carries the output scale, ADR
0153).

**Deferred, shell-side:** the shell's hand-rolled Wayland client renders its
QML chrome offscreen at a device pixel ratio of 1 and does not bind
`wp_fractional_scale` / `wp_viewporter`; at a fractional output scale the shell
ignores the compositor's preferred-scale event and its chrome is upscaled. The
compositor advertises the preferred scale (`render::update_chrome_preferred_scale`),
so the remaining work is entirely in `shell/`. Recorded in `docs/PROGRESS.md`
under Follow-ups.
