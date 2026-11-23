# 0333 — Publish `_NET_FRAME_EXTENTS` from a private X11 property connection

## Status

Accepted (T327).

## Context

X11 clients need `_NET_FRAME_EXTENTS` to place dialogs, menus, and tooltips
against their own content area. The compositor already draws the SSD titlebar
and configures the client one strip shorter, but the EWMH property was never
published (STATUS G-16), so clients assumed a zero-size frame.

Smithay 0.7's `X11Wm` keeps its X11 connection private and exposes no
property-write API; `X11Surface` only exposes typed operations such as
`configure`. The compositor therefore has no handle for arbitrary WM-owned
properties through Smithay.

## Decision

- `XwaylandState` owns a **private X11 connection** opened lazily on the first
  property write (`RustConnection::connect` against the exported `DISPLAY`). It
  interns `_NET_FRAME_EXTENTS` once and writes the four CARDINAL values as
  `left, right, top, bottom`. The connection is dropped on disconnect and
  restart, and is re-opened on the next write.
- The values come from `DfState::insets_for`: an SSD window's `top` is the
  resolved `WindowInsets::for_tier_with_chrome` height; `left`/`right`/
  `bottom` are zero because no border is drawn. A CSD X11 window (an explicit
  `_MOTIF_WM_HINTS` opt-out) reserves no inset, so it publishes zeroes.
- **The property is in X11 client pixels, and the output scale is not applied
  a second time.** Under ADR 0152 Xwayland's root window is the output's
  *logical* size and Smithay sends X11 geometry unscaled (`client_scale` is
  1), so one X11 pixel is one compositor logical point at every integer scale.
  A 32 pt titlebar is therefore 32 X pixels at scale 1 and at scale 2;
  multiplying by `integer_scale` would make a client reserve twice the strip
  the compositor actually draws.
- The property is seeded at **`CreateNotify`** and written again **before the
  frame is mapped**, each followed by a server round-trip (`sync`), so a
  client that reads it from its first `MapNotify` sees the final value even
  though Smithay maps the client window during reparenting. The map path
  republishes from the model, and `set_decoration_tier`, `set_window_style`,
  and every window-state transition republish as well. It is not a map-only
  property.
- Override-redirect windows are outside `WindowModel` and are not managed, so
  they are not written.

## Consequences

- X11 dialogs can position against the SSD titlebar; CSD X11 windows correctly
  report no compositor frame.
- The compositor holds one extra X11 connection for the session. A failed
  connect or write is logged and the connection is dropped; it never blocks a
  window map beyond one local round-trip.
- A future per-surface X11 scale (for example a mixed-DPI Xwayland policy)
  changes only `frame_extents` in `compositor/src/xwayland.rs`; the tier/state
  publishing paths stay the same.
- Evidence: `xwayland_conformance::x11_frame_extents_publish_the_ssd_titlebar`
  (SSD, CSD flip, republish) and the scale-2 assertion in
  `x11_fixture_at_integer_scale_two`.
