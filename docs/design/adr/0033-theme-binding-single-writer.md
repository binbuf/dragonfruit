# 0033 — One settings writer for the QML Theme

## Status

accepted

## Context

T-08.2a gave the shell one `SettingsClient` over `org.dragonfruit.Settings1`
(ADR 0032). The design-system `Theme` singleton (`design-system/Theme.qml`)
still resolved `dark` from `Application.styleHints.colorScheme` and left
`reducedMotion` `false`, and the shell wrote `Theme.reducedMotion` from its
Dock reconfigure path. `appearance.colorScheme` had no consumer, so a
settingsd change did not flip the theme. The track requires the design-system
`Theme.dark`/`Theme.reducedMotion` to be bound to the daemon.

## Decision

- **`shell/src/themebinding.{h,cpp}` is the single writer of `Theme.dark` and
  `Theme.reducedMotion`.** It consumes the same `SettingsClient` as the Dock
  (no second bus connection, no timer, no watcher) and reacts to `changed` and
  `refreshed`. The mapping is `appearance.colorScheme` →
  `Theme.dark` and `accessibility.reduceMotion` → `Theme.reducedMotion`.
- **`auto` follows the host and stays live.** `ThemeBinding` connects to
  `QStyleHints::colorSchemeChanged` and re-applies while the setting is
  neither `light` nor `dark`, so replacing the singleton's own
  `Application.styleHints` binding does not freeze the host preference. The
  resolver is the pure `ThemeBinding::darkForScheme(scheme, hostDark)`.
- **The shell owns the binding, not the design system.** The design system
  stays D-Bus-free and writable (the gallery and tests still assign `Theme`
  directly); the client seam and its lifecycle live in the shell (ADR 0032).
- **The compositor is a separate consumer.** T-08.2c mirrors the same key into
  `DfState::set_color_scheme`; this ADR does not touch it.

Rejected: binding the `Theme` singleton directly to `Application.styleHints`
and a settings value at once (two writers fight and the first settings change
breaks the host binding); a second `ThemeBinding` in every first-party app
(the shell is the session's process; apps inherit the running Theme).

## Consequences

- A settingsd `appearance.colorScheme` change flips the shell chrome within
  one event-loop turn; `tst_themebinding` asserts it headlessly, including a
  real gallery-variant pixel change.
- `appearance.accent` was unconsumed when this ADR was written:
  `Theme.color.accent` is a read-only scheme token, so an override needs a
  design-system change. T-09.2 added the generated `Theme.accentOverride` and
  the writer in `ThemeBinding` (ADR
  [0037](0037-accent-override-and-app-local-theme-sync.md)), which also gives
  apps a local mirror since this writer is shell-process-only.
- T-08.2c must mirror `appearance.colorScheme` into the compositor; until
  then a light shell can sit over the compositor's dark chrome/backdrop.