# 0125 — The Keyboard/Mouse/Trackpad pane and tile: settingsd keys plus the read-only inventory

- **Status:** Accepted (T-15.4b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settingsd-live-settings.md](../tracks/08-settingsd-live-settings.md),
  ADR [0124](0124-input-device-adapter.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.4a shipped `dragonfruit-input`, a deliberately read-only libinput adapter:
libinput keeps no persisted configuration and has no setter, so the adapter is
the device *inventory*, and principle 3 keeps keyboard/pointer policy with the
compositor. The Keyboard/Mouse/Trackpad pane therefore has two halves: a device
list the adapter can answer, and preference rows that must live somewhere
durable and apply live. The sound task (ADR 0123) established the precedent:
settingsd keys for rows with no adapter owner, the adapter for device state.

## Decision

The pane and the Control Center tile are one functional unit.

1. **Inventory.** `services/system-status` grows an `InputHost` and a read-only
   `org.dragonfruit.SystemStatus1.Input` interface (`State()`/`Refresh()` over
   `dragonfruit-input`), mirroring the Storage/Bluetooth halves. The Settings
   panes and the shell tile decode that view. There is no write: libinput has
   none, and the preferences are not adapter state.
2. **Preferences.** The pane's rows are settingsd keys, additive in schema
   revision 12: `input.pointerSpeed`, `input.naturalScroll`,
   `input.tapToClick`, `input.leftHanded`, `input.scrollMethod` (the pointer
   half, mapped to `compositor/src/input/settings.rs` `PointerSettings`), and
   `input.keyboardBrightness`, `input.adjustBrightnessLowLight`,
   `input.backlightOffAfter`, `input.keyboardNavigation`, `input.emojiKeyAction`.
   `Key repeat rate`/`Delay until repeat` reuse the revision-1
   `input.repeatRate`/`input.repeatDelay`. Every control writes somewhere
   durable and applies live; the keyboard-backlight and emoji-panel hardware
   bridges are documented follow-ups, exactly as the alert-sound engine is.
3. **One body, three panes.** `apps/settings/InputPane.qml` is parameterised by
   `section`; `KeyboardPane`/`MousePane`/`TrackpadPane` set it, so the pointer
   rows cannot drift. The Mouse pane has no capture; its rows follow the
   Trackpad `Point & Click` controls.
4. **Absence.** The adapter's two hide rules apply (ADR 0124): libinput gone or
   no recognized device hides the device list and shows a one-line note. The
   settingsd-backed rows stay live on the schema defaults.

## Consequences

- Every shipped row applies live and persists; no dead controls.
- Pointer application in the compositor is the remaining wiring: the compositor
  already has `PointerSettings`, but the private protocol's `set_input_policy`
  carries only keyboard repeat and gesture gating today. A later task extends it
  (a new append-only request) to apply the pointer keys.
- The Control Center panel grew by one compact tile; its fixed height is 880
  from 780, covered by the fit test and the live capture.
- The pane drops the Apple-only `Dictation`, `Text Input`/`Input Sources`,
  `Force Click and haptic feedback`, and `Look up & data detectors` rows, and
  the `Keyboard Shortcuts…` editor, which has no first-party surface yet.