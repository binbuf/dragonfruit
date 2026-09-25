# 0060 — Control Center panel surface and compositor-owned brightness

## Status

accepted

## Context

T-11.3a is the first Control Center slice. It needs a transient top-right
panel surface distinct from the notification banner and the Dock popover, and
it needs the Display tile's brightness control, which the design assigns to
the compositor ([04-shell.md](../04-shell.md), [07-system-integration.md](../07-system-integration.md)).
Before this task there was no per-output brightness state or request anywhere
in the compositor, and the Control Center entry point in the shell only
logged. T-11.3b extends the same panel and T-15 fills more tiles, so the
surface and ownership decisions are frozen here.

## Decision

- **The panel is one more generic layer surface, not a new protocol.** The
  shell creates a top-right `overlay` layer surface, namespace
  `"control-center"` (no compositor special-casing), anchored `top|right`
  below the menu bar, `exclusive_zone = -1`, keyboard interaction
  `ON_DEMAND`. It is mapped only while open; the whole panel is its input
  region. Escape and click-away dismissal use the existing chrome focus
  rules (Escape by key event, click-away by losing chrome keyboard focus),
  with a one-turn deferral so moving focus from the bar to the panel does
  not read as a dismissal.
- **Brightness is compositor-owned and settingsd-persisted.** `display.brightness`
  (double, 0.0–1.0, default 1.0, owner `shell/control-center`) is added to the
  settings schema at revision 4; the shell's existing display forwarder sends
  it as the new `df_output.set_brightness` request (interface v2, additive,
  appended last). The compositor stores the value per output name, clamps it,
  and reports it back in the `brightness` event. Applying it to hardware is
  best-effort and deferred: headless/nested store the value only, DRM is a
  later task.
- **Volume stays on the T-07 bridge host** (`services/system-status`); the
  panel writes through the existing `volumeSetRequested` path, not settingsd.
- **Wi-Fi is read-only for now.** The T-07 NetworkManager adapter exposes
  join but no radio write, so the tile reflects `radioEnabled` and its toggle
  is inert until T-15 adds the write; this is the tile-level degradation rule
  rather than a fake control.

Rejected: a new `df_control_center` interface (the layer surface already
carries geometry, input, and keyboard), a shell-local brightness store
(settingsd is the single owner, ADR [0034](0034-compositor-policy-via-shell-bridge.md)),
and an exclusive-keyboard panel (it would steal input from windows).

## Consequences

- T-11.3b adds Focus/DND and dark-mode tiles to the same QML module and
  surface; it must keep the `ON_DEMAND` + deferred-dismissal rules.
- T-15 can make the Wi-Fi toggle writable by adding the adapter method and
  flipping the panel's `wifiWritable` property; the tile model already
  carries `enabled`.
- A real DRM backlight apply slots into the `SetBrightness` handler without
  changing the protocol or the shell.
- The `brightness` event is `fixed` (24.8), so consumers compare with
  fixed-point tolerance.