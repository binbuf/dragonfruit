# Xwayland Fractional Scaling Policy

Decision record for the T-06 open question ("scale-viewport per X11 surface
vs integer scale"), from
[../.docs/tasks/06-xwayland.md](../.docs/tasks/06-xwayland.md). The design
direction is `viewporter` + `fractional-scale`
([../.docs/design/02-compositor.md](../.docs/design/02-compositor.md)),
which is a Wayland-client mechanism; X11 clients cannot opt in.

## Decision

**Integer-scaled Xwayland + per-surface viewport downscale.**

1. Xwayland is spawned at an integer scale equal to
   `ceil(primary_output_scale)`, clamped to at least 1. X11 clients then
   render into an integer coordinate space they understand, and the X
   server's DPI/scale is never fractional (Xwayland's `-scale` is
   integer-only).
2. The compositor maps each X11 toplevel buffer to the output's logical
   size with a `wp_viewport` (`set_destination`), i.e. it downscales the
   integer-scaled buffer to the fractional scale. This is the same
   `viewporter` path the Wayland surface already uses.
3. `wp_fractional_scale_v1` is never advertised to Xwayland; it is a
   Wayland-client protocol. X11 clients see the integer scale and a
   consistent, if slightly soft, rendering at 1.5x/1.75x.

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

- Downscaling a 2x buffer to 1.5x can soften subpixel-hinted text; the
  compositor's viewport filter choice is the mitigation.
- X11 popups, menus, and override-redirect windows must inherit the same
  viewport mapping so they scale consistently with their toplevel.
- The integer scale is chosen once per Xwayland session from the primary
  output; multi-monitor outputs with different scales need a per-output
  decision (run Xwayland at the largest, downscale on the smaller). This
  is deferred to T-31 (fractional-scale edge cases), together with the
  actual viewport plumbing.

## Current state (Foundation)

The policy is recorded now; the plumbing is not built yet. Xwayland runs
at scale 1 and the compositor does not yet apply a viewport to X11
surfaces, so X11 currently renders at 1x on every output. The viewport
downscale lands with T-13/T-31, when decorations and the render path are
fractional-scale aware.
