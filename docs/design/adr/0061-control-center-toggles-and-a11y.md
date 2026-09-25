# 0061 — Control Center Focus/dark-mode toggles and tile accessibility

## Status

accepted

## Context

T-11.3b completes the Control Center panel that T-11.3a opened
([0060](0060-control-center-panel-and-brightness.md)). The design assigns
Focus/DND to the notification service ([0058](0058-focus-dnd-policy-semantics.md))
and the light/dark scheme to settingsd, which the shell's `ThemeBinding`
already applies ([0033](0033-theme-binding-single-writer.md)). The panel is a
pure view whose gestures are forwarded to the owning service; it must not grow
a second owner for either state.

## Decision

- **The Focus tile is a Do Not Disturb switch, not a three-way selector.**
  On writes `SetFocusMode("dnd")`, off writes `off`, through the same
  `NotificationClient` seam the menu bar uses. `focus` (the middle mode) is
  not reachable from the tile, but it still lights the switch and the menu-bar
  crescent when set elsewhere. The tile's mode and suppression count always
  come from `FocusPolicy()`, never local state.
- **The Dark Mode tile writes an absolute scheme.** On writes
  `appearance.colorScheme = "dark"`, off writes `"light"` through the shell's
  `SettingsClient`; the tile never writes `auto`. `ThemeBinding` consumes the
  same `changed` echo and flips the whole design-system Theme, so the panel and
  its own chrome recolor live. The tile's shown state is the *effective*
  scheme (`auto` resolved against the host), matching `ThemeBinding`.
- **The mapping is a pure function.** `shell/src/controlcenterpolicy.{h,cpp}`
  turns the two booleans into the service vocabulary so a headless unit test
  pins the contract without a bus or QML.
- **Every tile carries accessible roles.** Each tile is an
  `Accessible.Grouping` named for its title; switches and sliders keep their
  design-system roles and gain explicit names where the visible label lives in
  a separate `Text`; the trailing `… Settings…` links are focusable
  `Accessible.Button`s with a Space/Return activation. `Toggle` gained an
  optional `accessibleName` for this (its label otherwise comes from its
  visible text).
- **The trailing settings links only log.** Launching Settings on a matching
  pane is T-16, exactly as the T-11.3a Wi-Fi link.

Rejected: adding a Focus segmented control (the macOS sheet anatomy is a
switch, and `focus` has no first-class scheduling UI yet); writing `auto` from
the dark-mode switch (it would silently undo the Settings app's Auto choice);
a shell-local fallback when the notification service is absent (the tile then
shows `off` and the write is a no-op, matching the menu-bar degradation).

## Consequences

- The panel surface grew to five tiles (360×520); the layout constants and the
  capture driver track it.
- A later task can add Focus scheduling/allow-list editing by extending the
  same `SetFocusMode`/`SetFocusAllowList` seam without changing the tile
  contract.
- The `Toggle.accessibleName` property is shared; other callers may set it when
  their visible label is external.
- The Focus/Appearance settings links remain inert until T-16 wires
  Settings-on-a-pane.