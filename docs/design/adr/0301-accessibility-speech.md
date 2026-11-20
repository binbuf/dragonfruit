# 0301 — Accessibility Speech: one delegated Live Speech group, honest omissions

- **Status:** Accepted (T291d)
- **Date:** 2026-11-19
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.5, [24-system-settings.md](../24-system-settings.md) §4.8,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.7,
  ADR [0276](0276-accessibility-pane-accommodations.md),
  ADR [0297](0297-accessibility-big-list-and-vision.md),
  ADR [0299](0299-accessibility-hearing.md)

## Context

The mockup's Speech section lists Live Speech, Custom Voice, and Vocal
Shortcuts. The build has no first-party speech synthesis service, no on-device
voice model or cloning service, and no vocal-shortcut trigger engine. It does
have one existing speech owner: the `speech-dispatcher` / `espeak-ng` delegate
(`Settings.speakAccessibilityText`) that the Vision Read & Speak row launches.

## Decision

- **Speech is the fourth big-list row**, after Hearing, in mockup order, with
  the speaker mark in the same category tile as every other row.
- **Live Speech is the one group.** Its single row, **Speak sample**, calls the
  shared speech delegate and names speech-dispatcher/espeak-ng as the owner and
  `spd-say` as the hand-off. There is no new speech integration.
- **A present-but-unavailable delegate disables, it never hides.** With
  neither `spd-say` nor `espeak-ng` installed the row stays visible, its action
  reads **Unavailable**, and the description names the missing program. The
  same `delegated` map row is searchable and landable in both states.
- **Custom Voice and Vocal Shortcuts are omissions.** They are represented
  only by `state: "omitted"` / `absence: "omitted"` map rows with a reason.
  They are not rendered, not landable, and not search-indexed. No fake toggle
  or disabled placeholder is added.
- **The section is not removed.** Because Live Speech has a real owner, the
  Speech row ships, so the Accessibility list is the mockup order minus only
  the sections that end up ownerless.

Rejected: removing the Speech row entirely (Live Speech has a real delegate);
inventing a voice model or vocal-shortcut engine; duplicating the Vision
Read & Speak row's behaviour under a second owner.

## Consequences

- `SettingsMap.accessibilityRowSubPanes` maps the `speech.` prefix to the
  `speech` sub-pane; `accessibilitySectionSubPanes` accepts Live Speech,
  Custom Voice, and Vocal Shortcuts.
- `tst_settings_accessibility` covers the list row, the sub-pane, the delegate
  round-trip, both omissions, search/deep links, keyboard, and the measured
  window; `tst_settings_accessibility_speech_absence` covers the absent engine.
- `make t291d-settings-accessibility-speech-capture` writes the list and
  sub-pane in light/dark plus the absent-engine still.
- If a speech service or voice model ever lands, flip the corresponding
  `speech.*` omission to a shipped row and add its synonyms.
