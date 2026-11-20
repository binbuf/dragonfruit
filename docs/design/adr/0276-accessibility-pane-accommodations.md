# 0276 — Accessibility is a five-section pane of live owners and honest delegates

- **Status:** Accepted (T275); the one-scroll form is superseded by the
  Accessibility big list and Vision sub-pane in
  [ADR 0297](0297-accessibility-big-list-and-vision.md) (T291a). The row
  shapes, owners, and absence rules below still hold.
- **Date:** 2026-11-15
- **Context:** [24-system-settings.md](../24-system-settings.md),
  [08-settings.md](../08-settings.md),
  [settings-business-logic-map.md](../settings-business-logic-map.md),
  ADR [0144](0144-accessibility-adapter.md),
  ADR [0145](0145-accessibility-pane-and-tile.md),
  ADR [0155](0155-compositor-magnifier.md)

## Context

The macOS Tahoe Accessibility pane is a five-section information architecture
(Vision, Hearing, Motor, Speech, General/Shortcut). T-15.14b shipped three
honest rows against it: the read-only AT-SPI status and one durable
`accessibility.reduceMotion` toggle. The T275 review found the map claiming
rows the pane did not render, a screen-reader row the map called a toggle, and
no compositor magnifier or shortcut consumer. The familiarity contract must
not become a wall of fake toggles.

## Decision

- **Five sections, five row shapes.** The pane renders Vision (status, Display,
  Zoom, Motion, Read & Speak), Hearing, Motor, Speech, and Accessibility
  Shortcut in that order. A row is a settingsd control, a read-only status, a
  delegate action, a stored preference (marked `storedOnly` in the map), or an
  explicit omission with a reason. No row is a fake toggle.
- **The screen reader is status + Open Orca.** `org.a11y.Status` is read-only
  (ADR 0144), so `VoiceOver` maps to a live status row and a real `Open Orca`
  action. The map records `delegated`, not `toggle`.
- **Live accommodations are one settingsd key each.** `accessibility.textSize`
  is one `Theme.textScale` input for the design-system type scale;
  `accessibility.differentiateWithoutColor` adds a non-colour check glyph to
  the Toggle; `accessibility.dimFlashingLights` reduces the Dock attention
  bounce amplitude; `increaseContrast`, `reduceTransparency`, and
  `reduceMotion` keep their existing Theme/Glass/compositor consumers.
- **The compositor owns the magnifier and the recorded shortcut.** A new
  append-only v12 `df_toplevel_manager.set_accessibility_policy` carries
  `accessibility.magnifier`, `magnifierZoom`, `magnifierMode`, and
  `accessibility.shortcut`. The magnifier is the whole-output transform from
  ADR 0155. The shortcut joins the one `ShortcutEngine` table as
  `accessibility-shortcut` and opens this pane; no client grabs keys.
- **Delegates are external programs, never reimplementations.** `Open Orca`
  launches `orca`; Read & Speak and Live Speech launch `spd-say` (falling back
  to `espeak-ng`); On-screen keyboard launches `onboard`/`wvkbd`. Each action
  disables with a reason when the program is absent. `DF_ACCESSIBILITY_DELEGATE_FIXTURE`
  makes the headless tests deterministic without spawning.
- **Absence keeps the structure.** With no AT-SPI host the pane still renders
  every section; the two status rows say `Unavailable`, the note explains why,
  and only the delegate actions that need a program disable. Durable keys and
  the magnifier/shortcut controls stay live.

Rejected: a second screen reader or speech engine; a Personal Voice/Vocal
Shortcuts equivalent; storing keys with no consumer (for example invert
colors, mono audio, sticky/slow keys) instead of disabling the row with a
reason; a magnifier or shortcut implemented in QML rather than the one
compositor owner.

## Consequences

- `services/settingsd` revision 32 adds seven `accessibility.*` keys and the
  `Accessibility` key group; `docs/settings-keys.md` is in lockstep.
- The shell's `ThemeBinding` and the app-local `SettingsShell` mirror the three
  new Theme keys; the Dock reads `Theme.dimFlashingLights`.
- `df_toplevel_manager` is version 12; the shell forwards the accessibility
  snapshot in `applyCompositorPolicy`, and the compositor installs the chord in
  its one binding table.
- Rows whose consumer is still deferred are visible, disabled, and named in
  `24-system-settings.md` §4.8 rather than presented as working.
- `make t275-accessibility-capture` writes every section and the absence
  structure in light/dark at 740×820.
