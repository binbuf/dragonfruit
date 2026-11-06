# 0218 — Dock motion: raised-cosine magnification, 135 ms smoothstep, and the controlled tooltip

## Status

accepted — amends [0162](0162-dock-hover-zoom-profile-and-label-tail.md) and
[0186](0186-dock-reference-retune-profile-fit-and-states.md) for the controlled
`macos-liquid-glass` motion capture.

## Context

The earlier Dock profile was fitted to screenshot stills and a partial hover
capture. The controlled reference campaign (`analysis/magnification_sweep.json`,
`analysis/temporal_response.json`, `docs/material-schema.json`) measures the
magnification spatially and temporally:

- the diagnostic `largesize=128` sweep peaks at **2.047x** and fits a raised
  cosine `S(d) = 1 + A * 0.5 * (1 + cos(pi * d / R))` with `A = 1.0676` and
  `R = 125.1 pt`;
- the default `largesize=73` case peaks at `73/60 = **1.217x**`;
- the step response is a **135 ms smoothstep** in both directions, with no
  overshoot;
- the tooltip is a **128 x 33 pt** capsule with a **16.5 pt** corner radius,
  **8 pt** above the Dock plate, appearing after **~1.4 s**;
- the genie minimize/restore is **~0.45-0.5 s** each way, and the one resolved
  launch lift is **15 pt** over ~0.3 s (the full multi-bounce is unresolved).

The shipped Dock used a pitch-normalized monotone cubic, a 160 ms CSS ease, a
250 ms tooltip dwell and a tailed capsule anchored to the icon box. It also
rendered genie as a 100/160 ms scale. Those are no longer the measured values.

## Decision

- **Spatial profile is an absolute-point raised cosine.** `Dock.qml` evaluates
  `0.5 * (1 + cos(pi * |d| / R))` from the resting tile centres, with
  `component.dock.magnifyInfluence = 125.1` pt. The profile is exactly zero at
  `R` and flat at both ends, so it is bounded for every live icon size. The
  continuous warp still pins the pointer through the resting-centre to
  magnified-centre map; the plate, hit region and popover anchors keep reading
  the same layout.
- **Two peak anchors.** `component.dock.magnifyPeak = 1.217` (default
  `magnification` 0.5) and `magnifyPeakMax = 2.047` (diagnostic 1.0). A
  smoothstep segment through `(0, 1)`, `(0.5, 1.217)` and `(1, 2.047)` keeps
  the mapping monotone and C1 at both anchors. The diagnostic peak is the
  observed 2.047x, not the 1 %-higher fitted `1 + A`.
- **The engagement envelope is the measured smoothstep.** `motion.dockHover`
  is 135 ms with cubic-Bezier control points `(1/3, 0)` and `(2/3, 1)`, which
  is exactly `g(u) = 3u^2 - 2u^3`. The existing `Behavior` retargets from the
  current value, so a quick reversal continues from the visible state instead
  of snapping or restarting. Reduced motion still collapses the duration to
  zero.
- **Tooltip follows the controlled shape and timing.** `component.tooltip.dwell`
  is 1400 ms; the capsule is a label-driven pill with `paddingV` raised to the
  8 pt spacing step, giving the measured ~33 pt height and half-height radius.
  The Dock anchors it 8 pt above the **plate** (not the icon box), keeps the
  horizontal centre on the magnified tile, and no longer opts into the tail.
  A stale projection (a reorder or pin change that moves a different tile
  under the pointer) dismisses the pending/open label rather than firing the
  old identity.
- **Genie timing is a mode-specific token.** `motion.windowGenie` is 450 ms
  with the same smoothstep control points. `WindowMotion` uses it only for
  minimize/restore while `dock.minimizedAnimation=genie`; scale keeps its
  existing 100/160 ms pacing and `none` keeps the collapse. The window target
  and origin mapping is unchanged.
- **Launch lift follows the measured 15 pt.** The existing three-hop launch
  bounce keeps its shape and total clock; only the amplitude becomes the
  measured `component.dock.launchLift` instead of a bar-relative quarter. The
  unresolved multi-bounce is deliberately not re-invented.

## Consequences

- `tst_dock` pins the raised-cosine samples, both peak anchors, the bounded
  influence radius, the 135 ms smoothstep and quick reversal, the tooltip
  shape/offset/dwell and stale-identity dismissal, and live size/magnification/
  position ranges. `tst_dockcore` clamps out-of-range live settings.
- The compositor pins the genie 450 ms envelope and the mode selection in
  `window/motion.rs`; scale and none are unchanged.
- `scripts/t199-dock-motion.py` reproduces the reference fit and checks the
  tokens (`make dock-motion-calibration`). The nested sweep capture remains the
  visual check on a host with a Wayland session.
- The tooltip tail stays available in the design-system component for other
  consumers, but the Dock no longer uses it.
