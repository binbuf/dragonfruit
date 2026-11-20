# 0299 — Accessibility Hearing: one shipped Audio group, honest omissions

- **Status:** Accepted (T291c)
- **Date:** 2026-11-19
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.5, [24-system-settings.md](../24-system-settings.md) §4.8,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.7,
  ADR [0276](0276-accessibility-pane-accommodations.md),
  ADR [0297](0297-accessibility-big-list-and-vision.md),
  ADR [0298](0298-accessibility-physical-and-motor.md)

## Context

The mockup's Hearing section lists five groups: Hearing Devices, Audio,
Subtitles and Captioning, Live Captions, and Spoken Name. The build has no
hearing-aid or LE Audio device service, no caption or live-caption engine, no
real-time text service, and no on-device name model. It does have one durable
audio preference, `sound.balance`, already written by the Sound pane.

## Decision

- **Hearing is the third big-list row**, after Vision and Physical and Motor.
  Its body has one group, **Audio**, with the single shipped row Balance.
- **Balance is the shared sound key.** The row writes `sound.balance` and is
  marked `storedOnly` because the audio engine has no balance consumer yet;
  the row says so. It is not a second audio model.
- **Ownerless groups are omissions.** Hearing Devices, Subtitles and
  Captioning, Live Captions, and Spoken Name are represented only by
  `state: "omitted"` / `absence: "omitted"` map rows with a reason. They are
  not rendered, not landable, and not search-indexed. Mono audio, Flash the
  screen on alert, and Background sounds are Audio-group omissions for the
  same reason; no fake toggle or disabled placeholder is added.
- **Row identity.** The big-list row uses the `speaker-high` Phosphor mark in
  the same category-tile container as every other row.

Rejected: a disabled placeholder for every missing Apple row; a caption
delegate backed by a program that does not exist; a second balance model that
diverges from the Sound pane.

## Consequences

- `SettingsMap.accessibilityRowSubPanes` maps the `hearing.` prefix to the
  `hearing` sub-pane, and the five group names resolve to it for section deep
  links.
- T291d–T291e follow the same one-row/one-body pattern; the omitted rows stay
  documented in the map.
- The T291c capture writes six stills: list and sub-pane in light/dark, the
  reduce-motion/increase-contrast state, and the absent-bridge state.
