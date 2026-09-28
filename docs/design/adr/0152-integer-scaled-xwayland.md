# 0152 — Integer-scaled Xwayland via the advertised `wl_output.scale`

## Status

Accepted (T-16.3a).

## Context

`docs/xwayland-scaling.md` fixes the fractional-scaling policy: Xwayland is
integer-scaled at `ceil(primary_output_scale)`, and the compositor applies a
per-surface `wp_viewport` downscale (T-16.3b). The policy assumed Xwayland's
integer `-scale` option. That option does not exist upstream, and `-hidpi` is
both integer-only and **rootful-only** (`man Xwayland`), so neither can drive a
rootless session.

The policy also says a fractional scale must never reach X11 clients. In the
implementation the compositor advertised `wp_fractional_scale_v1` to every
client and sent every window a fractional `preferred_scale` in
`render::post_repaint`, X11 windows included. Xwayland is a Wayland client, so
an X11 surface could receive fractional metrics — exactly what the policy
forbids.

## Decision

- The compositor chooses the Xwayland integer scale once per session as
  `ceil(primary_output_scale).max(1)` and records it in
  `XwaylandState::integer_scale`; it is logged with the `DISPLAY` hand-off.
  This is the same value Smithay already serializes as the integer
  `wl_output.scale`, so the output global is the single source of truth — no
  extra Xwayland option is needed.
- `render::post_repaint` skips the fractional `preferred_scale` for windows
  with an X11 surface (`accepts_fractional_scale`). Wayland windows keep it.
  The global is still bindable by Xwayland (Smithay 0.7 exposes no per-client
  global filter); not sending the event is the effective and safe enforcement.
- The headless backend takes its output scale from
  `DRAGONFRUIT_HEADLESS_SCALE` (default 1.0), so a conformance test can boot
  the eager Xwayland against a scale-2 primary. The synthetic-output harness
  only runs after Xwayland starts, so it cannot do this.

## Consequences

- An X11 client at scale 2 is reported with its logical geometry (200x120 stays
  200x120) and the X server's screen is the logical size; the headless gate
  `xwayland_conformance::x11_fixture_at_integer_scale_two` fails if the
  compositor double-scales X11 or inflates the configure.
- No viewport is applied yet, so at a fractional (non-integer) scale X11
  windows show at the integer `ceil` size — the documented, accepted fallback
  until T-16.3b.
- Multi-monitor outputs with different scales still use the primary's integer
  scale for the whole session; per-output decisions are T-16.3b.
- Known chrome bug exposed while validating: the wallpaper image element is
  sized in logical pixels but placed in physical pixels, so outputs above scale
  1 show the image only in the top-left fraction. It is chrome sizing and
  belongs to T-16.3b.