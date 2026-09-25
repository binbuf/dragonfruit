# 0063 — OSD keyboard and AT-SPI contract

## Status

accepted

## Context

T-11.4a shipped the transient volume/brightness OSD as a centered, unanchored
`overlay` surface with `keyboard = NONE` and an empty input region: it must
never steal focus from the active window
([0062](0062-osd-overlay.md)). The track still promises a keyboard/AT-SPI pass
for the OSD ([11-control-center-notifications.md](../tracks/11-control-center-notifications.md),
legacy FR-7), and T-11.4b has to commit the capture set. Because the surface is
non-interactive, the pass cannot add a focus ring or pointer targets; it must
make the alert announceable and keyboard-clearable without violating the
no-focus-steal rule.

## Decision

- **The OSD item is an AT-SPI alert with a name and a description.**
  `shell/osd/Osd.qml` keeps `Accessible.role: Alert` and the value-derived
  `Accessible.name` ("Volume 60%", "Brightness 75%", "Volume muted") and adds
  `Accessible.description` naming the state plus "Press Escape to dismiss."
- **The item exposes a PressAction.** `Accessible.focusable: true` plus
  `Accessible.onPressAction` lets an assistive client clear the alert; the
  surface still never holds keyboard focus in a real session, so this does not
  contradict the no-focus-steal rule (it is AT-SPI metadata, not Qt focus).
- **Escape clears the alert at the shell level.** `ShellController::onKeyEvent`
  hides a visible OSD before the Control Center/other chrome handling, so a
  keyboard user can dismiss it even though the OSD window is unfocused. The
  view also raises `dismissed()` from its own `Keys.onEscapePressed`, and
  `ShellController` connects `dismissed()` to the same `hideOsd()` slot as the
  auto-dismiss tick (`OsdModel::hide`).
- **The OSD volume glyph is a speaker.** The T-11.4a capture exposed
  `Icon.qml`'s `volume` glyph as a "drive slab" that read as a battery; T-11.4b
  redraws it as a speaker body + cone + waves, matching `StatusGlyph.qml`'s
  menu-bar volume glyph.
- **The capture set is committed under `docs/captures/t11-*`.**
  `scripts/capture-osd-dnd.sh` (`make osd-dnd-capture`) runs the status/
  notification fixtures with DND, crops the menu-bar crescent to `t11-dnd.png`,
  and drives a real Control Center Sound drag to present the OSD, writing
  `t11-osd.png` and `t11-osd-context.png`. The T-11.3a/b Control Center stills
  remain as committed.

## Consequences

- A screen reader can announce the OSD and invoke dismiss; a keyboard user can
  press Escape. The OSD still auto-dismisses and never takes focus.
- The live AT-SPI tree walkthrough (cross-process dump, every flow
  keyboard-only) remains T-16.6a; T-11.4b asserts the per-item roles and the
  dismissal path headlessly.
- `DF_OSD_FIXTURE` remains a capture-only seam; hardware media keys are still
  not wired (a later task routes them into `ShellController::showOsd`).
- The `Icon.qml` `volume` change affects the Control Center Sound tile too; the
  gallery icon snapshot only renders the titlebar/utility glyphs, so it is
  unaffected.