# 0039 — Slider and Select join the design-system component library

## Status

accepted

## Context

T-09.4 is the Settings app's Desktop & Dock pane. Its reference UI is the
macOS-style slider rows (`Size`, `Magnification`) and value-popup rows
(`Dock position`, `Minimized window animation`, `Titlebar double-click
action`). The design system has neither control: the settings panes so far
used only `Toggle` and `SegmentedControl`, and the shell's volume menu
hand-rolls a slider. T-09.5 (Displays) needs a `Brightness` slider on the very
next task, and the T-15 panes repeat the popup pattern, so a pane-local
composition would be copied several times.

The design-system rules (`10-design-system.md`) say every first-party app
consumes the library, one source of visual truth, and keyboard/AT-SPI/reduced
motion are per-component definition-of-done — so the fix belongs in the
library, not in `apps/settings`.

## Decision

- **Add `Slider` and `Select` to `design-system/components/`** (the module and
  its `df_qml_lint` list), with tokens `component.slider` and
  `component.select` generated into `Theme.qml` / `design_tokens.rs` as usual.
- **`Slider`** owns a real `[from, to]` value, drag/tap/arrow/Home/End input,
  optional `minLabel`/`midLabel`/`maxLabel` captions, and the one `FocusRing`;
  it reports `moved(value)` continuously and `committed(value)` at gesture
  end and never writes its own bound state, so the T-09.1b
  write-on-interaction / bind-to-`Settings.values` pattern holds. AT-SPI role
  `Slider`.
- **`Select`** is the popup row: the current label plus a trailing chevron
  that opens a `ContextMenu` of the choices. It reports
  `activated(index)`/`selected(value)` and re-reads `currentIndex` from the
  owner. AT-SPI role `ComboBox`. Reusing `ContextMenu` keeps one menu chrome
  and one keyboard model.
- **Both ship gallery pages and per-component tests** (rendered states,
  keyboard, roles, token colors), satisfying the library's quality gates.

## Consequences

- Later settings panes (`Displays`, `Menu Bar`, `Sound`, …) use `Slider` and
  `Select` instead of composing their own; changing slider/select geometry or
  colors is a token edit in one place.
- Like the existing design-system `Popup`, `Select`'s menu is a plain `Item`
  and is clipped by a `ScrollView`. A pane keeps popup rows inside the initial
  viewport, or a future overlay layer must host them; the Desktop & Dock
  pane's three popups are all above the fold.
- `Slider` writes on every `moved`, so a settings pane bound to a Dock key
  emits a settingsd write per drag step and the shell re-lays-out live. That
  is the intended "one interaction beat" behavior; if it needs damping, the
  shell's divider-preview pattern is the precedent.