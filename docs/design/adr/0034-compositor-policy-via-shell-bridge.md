# 0034 — The compositor consumes settingsd policy through the shell bridge

## Status

accepted

## Context

T-08 makes `settingsd` the single owner of desktop settings over
`org.dragonfruit.Settings1` (ADR 0030). The shell consumes that surface
through one `SettingsClient` (ADR 0032); the design-system Theme followed in
T-08.2b (ADR 0033). The compositor still held defaults for the motion and
input keys: `accessibility.reduceMotion` reached it over the private protocol,
but `appearance.colorScheme`, `dock.titlebarDoubleClick`,
`dock.minimizedAnimation`, `input.repeatDelay`/`input.repeatRate`, and the
`gestures.*` switches did not. The design doc's diagram shows settingsd over
the compositor, but the compositor is a Wayland server with no D-Bus client
stack, and the existing reduced-motion precedent is a private-protocol request
from the shell.

## Decision

- **The compositor does not open a second D-Bus connection.** It consumes the
  motion/input policy over the private `df_toplevel_manager` protocol: the
  additive v5 `set_motion_policy` (resolved color scheme, titlebar
  double-click, minimized animation) and `set_input_policy` (repeat delay/rate
  and the gesture family switches). `set_reduced_motion` (v3) stays.
- **The shell is the sole forwarder.** `ShellController::applyCompositorPolicy`
  re-reads the same `SettingsClient` the Dock and Theme use (no watcher, no
  timer, no new bus connection), maps it through the pure
  `shell/src/compositorpolicy.{h,cpp}`, and forwards it. It runs on
  `changed`/`refreshed`, on host `colorSchemeChanged` (for `auto`), and once
  after the protocol authenticates.
- **The compositor is the sole applier.** `DfState::set_motion_policy`/
  `set_input_policy` apply to the live seat, gesture recognizer, and render
  state. Unknown spellings leave the current value untouched. There is no
  local settings file on either side (the shell's interim file was deleted in
  T-08.2a; the compositor never had one).

Rejected: a D-Bus client (zbus) inside the compositor (a second event loop and
a duplicate owner, against the "one consumer path" precedent); passing raw
`auto` to the compositor (it has no host style hints, so the shell resolves
it); a single monolithic `set_policy` with a variant map (the protocol is
typed and additive-only within a release).

## Consequences

- A settingsd key change alters compositor motion and input repeat within one
  event-loop turn. `compositor/tests/shell_protocol_conformance.rs` drives the
  v5 requests and reads the synthetic `query policy` instrument; the shell's
  pure mapping is covered by `tst_compositorpolicy`.
- `dock.minimizedAnimation=genie` is accepted and currently renders as
  `scale` (the genie warp is a material-pass follow-up); `none` collapses the
  minimize/restore tween exactly like reduced motion.
- `workspaces.count` is **not** in this bridge: it is a workspace-model key,
  not motion/input, and still needs an owner in a later task. The gesture
  `spaceSwitch`/`missionControl` switches are gated in the recognizer.
- Any future compositor-consumed settings key must extend the v5 request (or
  append a new `since` request) and the `compositorpolicy` mapper, never add a
  second source.