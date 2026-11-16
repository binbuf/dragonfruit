# 0281 — Device/input panes: separate pointer families, honest stored-only rows

- Status: accepted
- Date: 2026-11-16
- Task: T280

## Context

The reviewed Sound, Keyboard, Mouse, Trackpad, Printers & Scanners, and Game
Controllers panes had four structural problems: Mouse and Trackpad shared a
single `input.pointerSpeed`/`input.naturalScroll` pair; several preferences were
presented as live although no process read them; Sound exposed only the sink
level and zeroed its slider while muted; and the printer/controller dialogs
lost state or had no explicit way to close.

The completion contract (ADR [0262](0262-system-settings-completion-contract.md))
forbids a fake control and requires a hardware-dependent row to be disabled with
a reason, not hidden. It also permits a durable preference with no consumer to
ship as `stored-only`.

## Decision

1. **Pointer families are per device.** Mouse rows write `input.mouse.*`;
   trackpad rows write `input.trackpad.*`. The revision-1 shared
   `input.*` pointer keys remain read fallbacks for an upgraded store, never a
   write target. A pane row never writes the other device's key.
2. **The new families are stored-only until the compositor reads them.** The
   five remaining gesture families, mouse smart zoom/acceleration, and the
   per-device pointer keys ship with a real key and a visible "stored-only"
   description. They are not described as live.
3. **Sound has two independent levels.** `SetSourceVolume`/`SetSourceMute`
   write the default source; `SetVolume`/`SetMute` remain the sink's. The
   output slider always shows the stored level, and committing while muted
   unmutes first. Balance is stored-only (no per-channel CLI write); Per-app
   volume is disabled with the `pw-dump`/`wpctl` limitation.
4. **Hardware probes disable, never hide.** The input view carries
   `backlightAvailable` from `/sys/class/leds/*kbd_backlight/brightness`; the
   three brightness rows stay visible and disabled with a reason when it is
   false. Haptics already follows the touchpad inventory.
5. **Printer actions are explicit writes over the one seam.** Remove
   (confirmed), test page, and share are `PrintSource` methods with a mock
   counter; Open Scanner is a delegated hand-off (`simple-scan`/`xsane`) and is
   absent when no scanner application exists. Discovery and options have a
   loading state; the Add sheet resets every field on open.
6. **Controller dialogs are modal with a fresh report.** Opening Test clears
   the previous report through `clearControllerTest`; both sheets have a Close
   action. Rumble is disabled with its reason.

## Consequences

- The panes are honest before the compositor bridge exists; T282 can wire the
  stored families without changing the pane contract.
- A future per-device compositor policy consumes `input.mouse.*` and
  `input.trackpad.*`; the legacy keys then become migration-only.
- The capture (`make t280-devices-capture`) records every pane, the Sound input
  tab, the shortcut/dictation sheets, the three trackpad tabs, the add-printer
  sheet, the controller test, and absence stills in light and dark.
