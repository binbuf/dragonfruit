# 0326 — Per-output DPMS is a connector property with a named fallback

## Status

Accepted (2026-11-23). Implements the compositor half of
[ADR 0313](0313-idle-dpms-ownership.md) and extends the private protocol to
`df_output` v5.

## Context

ADR 0313 puts the live idle timer and DPMS in the compositor, but the tree had
no panel-power path: `idle-blank` reached the shell and the display stayed on.
The DRM backend already owns one `SurfaceData` per CRTC and knows the
connector at `connector_connected`, but it did not keep the handle. The only
generic disable switch in Smithay is `DrmDevice::activate(false)`, which
affects every output on the device — the opposite of the per-monitor behavior
the Displays UI promises. A DPMS write can also fail (a connector without the
property, a driver rejecting the ioctl); that must be visible without ending
the session.

## Decision

1. **One connector write per output.** `SurfaceData` stores the
   `connector::Handle`. `df_output.set_dpms(on)` resolves the output by name,
   finds that connector's `DPMS` property, and writes it with `SETPROPERTY`.
   It never calls `DrmDevice::activate(false)` and never touches another
   output's connector.
2. **The compositor applies the idle stage, the shell may also request it.**
   `idle-blank` blanks every on output; `idle-awake` and `resume_session` wake
   every off output and request a repaint. The shell's `set_dpms` request is
   the same per-output path (T317 uses it for its own stage handlers), so both
   owners converge on one model.
3. **A blanked output is render-gated.** An atomic commit sets the CRTC active
   and would turn the panel back on. `render_surface` therefore skips a
   blanked output, and `frame_finish` reapplies DPMS off once for a frame that
   was queued just before the blank. The wake clears the state before the
   repaint.
4. **Nested and headless keep the state and name the fallback.** The pure
   `DpmsModel` records on/off and a `DpmsFallback`
   (`none|nested|headless|failed`, wire 0–3). The `df_output.dpms` event
   carries both values; the initial property replay already reports them. A
   failed DRM write leaves the previous state live and reports `failed` (3);
   it is logged and never ends the session.
5. **The state is per output.** `DpmsModel` is keyed by the stable output
   name, so one monitor can blank while another stays on, and an unplug
   forgets only that output.

## Consequences

- `df_output` bumps to v5 with the additive `set_dpms` request and `dpms`
  event. Older clients are unaffected; the shell stub remembers the last
  request so an output announced later receives it.
- The compositor owns the automatic blank even before T317 lands; T317 can
  still drive the same request from its idle handlers and owns dim/brightness,
  lock, and suspend.
- The actual panel-off is hardware-open until a DRM seat runs the T347
  hardware pass; headless tests prove the request/ack, the state machine, the
  render gate's decisions, and the named fallback only.
- If the connector has no `DPMS` property or the write is rejected, the panel
  stays in its previous state and the shell shows `failed`; a later request
  retries.
