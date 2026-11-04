# 0202 — VRR adaptation, chrome callbacks, and mixed-refresh evidence

## Status

accepted (T-21.2b, T188c)

## Context

ADR [0198](0198-per-output-refresh-model.md) carries `VrrState` but does not
interpret it; ADR [0200](0200-per-output-presentation-clock.md) records each
output's presentations; ADR [0201](0201-per-output-frame-requests-and-earliest-deadline.md)
schedules animation requests against those clocks. T188c has to decide what a
variable interval means, keep chrome frame callbacks on the same presentation
clock, and prove a 60 Hz + 144 Hz mixed run without forcing either output onto
the other's period.

## Decision

- **VRR adapts only with an explicit state and a range.** `PresentationClock`
  owns an optional backend-supplied `VrrRange` (shortest and longest period).
  VRR is active only when `VrrState::On` *and* a range is installed. A
  presentation interval inside the range becomes the next deadline; an
  interval outside it (an idle gap, a too-fast completion) falls back to the
  nominal mode period. One measured interval never turns VRR on.
- **The backend owns the state.** `DfState::output_vrr` is keyed by output.
  DRM queries `DrmCompositor::vrr_supported` / `vrr_enabled` at connector
  connect. `Supported` is enabled without a modeset; `RequiresModeset` and
  `NotSupported` keep the fixed period and report `off` / `unsupported`.
  Nested and headless report `unsupported`, so a measured interval can never
  claim VRR there. `df_output.vrr` reports the actual `On` state, not a
  hardcoded zero. This consumes a backend capability; it does not add a VRR
  setting, UI, or user policy.
- **The range is explicit.** Today it is the mode period (fastest end) to the
  refresh model's 24 Hz floor (slowest end). The clock validates against it; it
  never infers a panel range from measurements.
- **Chrome callbacks ride presentation.** `post_repaint` answers pending
  `wl_surface.frame` requests on every presented output frame, and
  `send_frame_callbacks` answers them on the no-damage callback path. Both use
  the same presentation timestamp as windows. There is no 1 s chrome throttle:
  a visible chrome surface is the shell's pacing source.
- **Mixed evidence has three layers.** The pure scheduler test runs 60 Hz and
  144 Hz clocks with an idle third output; the synthetic-output harness accepts
  an optional refresh (`add ... <refresh_mhz>`) and `animate-dummy-on <output>`
  scopes the calibration animation to one output; the
  `scripts/t21-vrr-mixed-refresh.sh` capture records a nested single-output
  trace, a headless 60/144 three-scenario trace, and an explicit native `OPEN`
  artifact when no free two-output DRM seat is available.

## Consequences

- VRR-capable connectors that can toggle without a modeset can present at a
  variable interval; HDMI `RequiresModeset` stays fixed. A 60 Hz output keeps
  its own deadline and is never throttled to 144 Hz (or vice versa).
- `presentation stats` and `frame request` append `vrr`, the accepted
  interval, and the range; the missed-deadline rule still compares against the
  nominal deadline, so a 60 Hz animation on a 144 Hz output is not reported
  healthy.
- Native mixed-refresh remains a hardware run; the committed artifact is
  `OPEN` with the exact rerun command rather than a synthetic claim.
