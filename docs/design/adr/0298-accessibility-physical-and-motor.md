# 0298 — Accessibility Physical and Motor: two shipped groups, honest omissions

- **Status:** Accepted (T291b)
- **Date:** 2026-11-19
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.5, [24-system-settings.md](../24-system-settings.md) §4.8,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.7,
  ADR [0276](0276-accessibility-pane-accommodations.md),
  ADR [0281](0281-device-input-pane-completion.md),
  ADR [0297](0297-accessibility-big-list-and-vision.md)

## Context

The mockup's Physical and Motor section has four groups: Voice Control,
Keyboard, Pointer Control, and Switch Control. The build has no voice-control
engine, no sticky/slow-key machine, no keyboard pointer mode, no per-device
pointer ignore, and no switch-control or scanning source. It does have a
durable keyboard-navigation preference, an on-screen-keyboard delegate
(`onboard`/`wvkbd`), and a compositor-read double-click interval.

## Decision

- **Two groups ship.** Physical and Motor is the second big-list row, after
  Vision, and its body has exactly **Keyboard** and **Pointer Control**.
- **Keyboard keeps two rows.** Full keyboard access writes
  `input.keyboardNavigation` and is marked `storedOnly` because no compositor
  consumer reads it yet; the row says so. On-screen keyboard is a delegate row:
  `Open` launches `onboard`/`wvkbd`, and an absent program keeps the row with a
  disabled action and the missing-program reason.
- **Pointer Control keeps the live key.** Double-click speed writes
  `input.doubleClickSpeed`, which the compositor pointer policy reads. The
  Mouse and Trackpad panes keep their per-device `input.mouse.*` /
  `input.trackpad.*` values.
- **Ownerless rows are omissions.** Voice Control, Sticky Keys, Slow Keys,
  Mouse Keys, Ignore built-in trackpad, and Switch Control are `state:
  "omitted"` / `absence: "omitted"` map rows with a reason. They are not
  rendered, not landable, and not search-indexed; no fake toggle or disabled
  placeholder is added.
- **Row identity.** The big-list row uses the `hand-pointing` Phosphor mark in
  the same `SettingsCategoryIcon` container as every other category.

Rejected: rendering Voice Control as a speech-to-text toggle (dictation is a
text-input preference, not pointer control); a disabled placeholder for every
missing Apple row; a second row list or loader for this section.

## Consequences

- `SettingsMap.accessibilityRowSubPanes` maps the `motor.` prefix to the
  `motor` sub-pane, and the Keyboard/Pointer Control section names resolve to
  it for section deep links.
- T291c–T291e follow the same one-row/one-body pattern; the omitted rows stay
  documented in the map.
- The T291b capture writes six stills: list and sub-pane in light/dark, the
  reduce-motion/increase-contrast state, and the absent-delegate state.
