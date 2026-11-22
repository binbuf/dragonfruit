# 0316 — Settings consumer conformance

## Status

Accepted (2026-11-22). Extends the “no dead affordance” gate in
[ADR 0262](0262-system-settings-completion-contract.md) and
[`24-system-settings.md`](../24-system-settings.md).

## Context

The Settings map and the settingsd schema distinguish `stored_only: false`
(live) from `stored_only: true` (durable preference with no consumer). The
2026-11-22 review found live-marked keys with no observable effect:

| Key | Current consumer | Actual effect |
|---|---|---|
| `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend` | “session/idle engine” | No production service starts the engine |
| `power.preventSleep` | “session/idle engine” | Same |
| `display.brightness` | “shell/display forwarder, compositor/output” | Stored and acked; no panel write |
| `input.pointerSpeed`, `input.naturalScroll`, `input.tapToClick`, `input.leftHanded`, `input.scrollMethod`, `input.secondaryClick`, `input.scrollSpeed`, `input.doubleClickSpeed`, `input.haptics` | “compositor/input …” | Stored in `PointerSettings`; never applied to libinput |
| `input.keyboardBrightness`, `input.adjustBrightnessLowLight`, `input.backlightOffAfter` | “compositor/keyboard backlight” | No hardware bridge |

The existing gates check that a row has an owner/consumer string in the map,
and that the schema key types match. They do not check that the named consumer
exists. A string is not a consumer. The user sees a live slider and the
hardware does nothing; this is the same class of defect as a dead button.

## Decision

1. **A live key has a test.** Every key with `stored_only: false` must have a
   named consumer and an integration test that observes the effect (a mock
   device write, a protocol request, a logind call, a compositor state change).
   The test name and the consumer are recorded in one table.
2. **No test means stored-only.** A key without such a test is
   `stored_only: true`; the Settings pane shows the existing stored-only note
   with what it will control. A live row with no consumer is a task failure.
3. **The stability set owns the initial list.** T304 adds
   `scripts/check-settings-consumers.py` and `scripts/settings-consumers.json`,
   which start with the known dead keys as an explicit “known gaps” list.
   T315–T320, T336, and the brightness/idle tasks remove entries as they land.
   The check fails if a new `stored_only: false` key appears without a
   consumer entry.
4. **The map and the schema agree.** A key is live in both or stored-only in
   both. The existing schema-mirror test stays; the consumer test is the
   behavioral half.
5. **The UI tells the truth.** A stored-only row is not disabled; it keeps its
   control and says what it will control. A live row must not say stored-only.
   The existing `SettingsMap` validation is extended to the consumer table.

Rejected alternatives:

- **Trust the `consumer` string.** It is already wrong for the keys above.
- **Mark everything stored-only.** That would hide real regressions (brightness,
  pointer settings) instead of fixing them.
- **A manual checklist only.** It would drift; the test is the gate.

## Consequences

- The stability set removes the known gaps; the test becomes an empty list and
  stays as a guard.
- New Settings rows must add a consumer test before they can be marked live.
- The schema `consumer` and `stored_only` fields become behavioral claims, not
  documentation.
- The Settings panes for keyboard backlight, auto-brightness, and the remaining
  stored-only preferences keep their note; they are not silently promoted.
