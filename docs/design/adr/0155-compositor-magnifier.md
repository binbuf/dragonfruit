# 0155 — The compositor magnifier and the reduced-motion sweep

## Status

accepted

## Context

The T-16 accessibility sweep ([16-platform-polish-packaging.md](../tracks/16-platform-polish-packaging.md))
asks for "compositor magnifier if specified, reduced-motion across every
animation". The compositor's responsibilities already name screen
magnification as compositor-owned because it transforms the whole scene, not
one window ([02-compositor.md](../02-compositor.md)); the legacy
`31-polish-hardening` ticket scheduled it with follow-focus/follow-caret
"shipped to Settings". T-15.14b deferred it when it built the Accessibility
pane, because no Linux host owner or Settings row existed, "rather than shipped
as dead controls" ([07-system-integration.md](../07-system-integration.md)).

Two questions were open: what the magnifier contract actually is, and how to
prove that every animation has a reduced-motion variant.

## Decision

- **The magnifier is a whole-output uniform zoom, not a lens.** At zoom `z`
  the output shows the scene rectangle `size/z` centred on a scene point; the
  render layer rescales every element about the output centre and translates
  so that point lands on the output centre. The model is pure
  (`compositor/src/magnifier.rs`): zoom (clamped `1.0..=8.0`, `1.0` disables),
  the centre, the follow mode, and the two coordinate mappings. The nested
  backend applies it as one `Rescale(+Relocate)` wrapper per element and forces
  a full-damage frame while it is active. DRM is not wired yet (it is the
  untested hardware rail; its cursor must not scale with the scene).
- **Input is mapped, not ignored.** Every pointer/touch position is mapped from
  the view back to the scene point under it, and relative pointer deltas are
  divided by the zoom, so hit-testing and the client's cursor position stay on
  the content the user sees. A magnifier that renders but misroutes clicks
  would be worse than none.
- **Follow modes.** `follow-focus` re-centres on the focused window.
  `follow-caret` is modelled and accepted, but this compositor has no
  caret-position source, so it falls back to the focused window (the honest
  Linux adaptation). Both clamp the centre so the visible window never leaves
  the output.
- **The durable trigger is a follow-up.** This task's area is `compositor/` and
  `shell/`; the Settings row, its `settingsd` key, and the private-protocol
  request that carries the value are outside it. Until they land the magnifier
  is reachable through the synthetic-input seam (`set magnifier`,
  `set magnifier-zoom`, `set magnifier-mode`, `set magnifier-center`,
  `query magnifier`), which is what the conformance test and the capture use.
- **Reduced motion is swept, not spot-checked.** Every named motion is
  generated from one source (`design-system/tokens/tokens.json`) and carries a
  reduced variant (`reducedDuration: 0`). A headless sweep
  (`compositor/tests/reduced_motion_sweep.rs`) enumerates the Rust and QML
  motion catalogs (cross-checked), every QML animation site in `shell/`,
  `design-system/`, and `apps/` (each must resolve through `Theme.motion` or
  gate on `Theme.reducedMotion`), and every compositor lifecycle kind (each
  must route through `Tween::from_motion`; the overview machine keeps its
  single-step branch). A new literal-duration animation fails the sweep.

## Consequences

- The magnifier is a real compositor feature with a pure, headless-tested
  model and a live A/B capture (`docs/captures/t16-magnifier.png`); it is
  applied on the nested (development/demo) backend. Wiring it to Settings and
  the private protocol is a bounded follow-up, and DRM magnification is a
  second.
- `follow-caret` is normative but currently equivalent to `follow-focus`; a
  later text-input caret source makes it real without changing the contract.
- The sweep is the standing guard for "every animation has a reduced-motion
  variant": adding an animation without one fails CI (it is part of
  `make e2e`).
- The magnifier reuses the existing `Rescale`/`Relocate` element wrappers and
  the scene-transform discipline, so no new render effect path was added.