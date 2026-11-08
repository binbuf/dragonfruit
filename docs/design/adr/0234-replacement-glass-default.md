# 0234 — Ship the replacement glass look by default

**Status:** accepted (2026-11-07)

## Context

T200–T207 built and calibrated the replacement glass vocabulary: the no-blur
menu-bar veil, role presets, attached/detached/status menu families, Control
Center and notification materials, first-party Settings/Files chrome, and the
nested legacy/replacement acceptance matrix. T208 kept the shipped rollout at
`legacy` because the native replacement rows could not be produced without a
free DRM seat; ADR [0227](0227-t208-native-rollout-gate.md) records that gate.

The replacement is now the intended everyday appearance. Keeping the default
at `legacy` means a normal native session still paints the old QML chrome
plate (Light 0.82 / Dark 0.72 alpha) and never shows the veil or the
replacement presets. The rollout switch must remain for A/B and rollback, but
it should not keep the improved look behind an environment variable.

The durable settings that control the glass are already the replacement
defaults: `appearance.transparency = 0.5255` (the measured midpoint),
`accessibility.reduceTransparency = false`,
`accessibility.increaseContrast = false`, and `menu.showBackground = true`.
They stay unchanged; this ADR only changes the look selected by default.

## Decision

- `design-system/tokens/tokens.json` sets
  `glass.rollout.defaultMode = "replacement"`. The generated `Theme.qml` and
  `design_tokens.rs` follow from that single source.
- `Rollout::default()` reads the generated token instead of hard-coding a
  variant. `Rollout::from_env()` still honours an explicit
  `DRAGONFRUIT_GLASS_ROLLOUT=legacy|replacement`; an unset or unknown value
  takes the shipped default.
- The shell controller seeds `Glass.rollout` from the token default and only
  overrides it for an explicit environment value. The Settings and Files apps
  return an empty override when the variable is unset, so their QML shells read
  `Theme.glass.rollout.defaultMode` rather than a duplicated literal.
- The legacy rail stays fully selectable:
  `DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild` (or
  `make native-reboot`) persists the rollback through `native.env` and the
  compositor/shell unit drop-ins.
- The settings schema defaults above stay pinned by the settingsd schema tests
  and the settings-client default table; no user preference is migrated or
  rewritten.

## Consequences

- Sessions started after this change use the replacement glass by default: the
  compositor veil, role presets, menu families, Control Center and
  notification materials, and the first-party app chrome.
- A plain `reboot` no longer needs the environment variable to see the
  replacement; the variable is now only an explicit A/B or rollback switch.
- T208's native replacement performance, accessibility, scanout and lock-exit
  rows remain OPEN. Enabling the default does not convert those rows into
  evidence; they must still be measured on a free DRM seat.
- The legacy rail remains the recovery path. A failed replacement session can
  return to the old appearance with one explicit `native-rebuild`.

## References

- `design-system/tokens/tokens.json` (`glass.rollout`)
- `compositor/src/material_preset.rs` (`Rollout`)
- `design-system/Glass.qml`, `shell/src/shellcontroller.cpp`
- `apps/settings/SettingsBridge.cpp`, `apps/files/FilesSettings.cpp`
- `docs/settings-keys.md`, `docs/rendering.md`, `docs/runbook-drm-session.md`
- ADR [0226](0226-t207-nested-acceptance-matrix.md), ADR
  [0227](0227-t208-native-rollout-gate.md)
