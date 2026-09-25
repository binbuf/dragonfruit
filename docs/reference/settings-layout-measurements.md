# System Settings layout — reference measurements

Deterministic measurements of the local-only macOS captures under
`docs/reference/macos/` (git-ignored, never shipped), used to align the
Settings app's window, sidebar, and row layout with the reference. The
information architecture and wording live in
[System_Preferences.md](System_Preferences.md); this file covers *geometry and
color*.

Reproduce with:

```sh
python3 scripts/measure-settings-reference.py 9.21.21
python3 scripts/measure-settings-reference.py 9.21.21 --json
```

## Method and units

A capture does not record its display scale, so the script reports distances in
**traffic-light units**: pixels divided by the red close button's diameter.
Our `component.trafficLights.diameter` token is 12 (px), which is the same
physical size macOS uses (12 pt), so one unit is one logical pixel in our app.

The captures prove this scale independently: the macOS close button, the
System Settings sidebar row, and the toolbar all measure at the same ~2.3 px
per unit, and that yields 12 / 28 / 52 in our units — matching our existing
`trafficLights.diameter`, `sidebar.rowHeight`, and `toolbar.height` tokens.
That is the evidence the rest of the numbers below are read at the same scale.

## Reference values (median across the System Settings captures)

| Metric | Reference (units) | Our token | Status |
|---|---|---|---|
| Window | ~620 × 627 | `900 × 620` | **fixed** → `640 × 640` |
| Sidebar width | ~191 (31% of window) | `220` token, `240` hardcoded | **fixed** → `192` |
| Toolbar height | ~52 | `52` | already matches |
| Sidebar row height | ~27.4 | `28` | already matches |
| Content side margin | ~24 | `spacing.xl` = `24` | already matches |
| Settings row height | ~31–33 | `44` | **recommend** ~36 |
| Sidebar bg | `#f9f9f9` | `surfaceMuted` `#f8f6fa` | close |
| Content bg | `#ffffff` | `surface` `#ffffff` | matches |
| Group card fill | `#f7f7f7` | `surfaceElevated` `#ffffff` | **differs** (see below) |
| Selected row / accent | `#3474ee` | `accent` `#b32a66` | deliberate brand color |

## Applied changes

- `apps/settings/SettingsWindow.qml`: default `640 × 640`, minimum
  `560 × 480`. The reference window is ~620 wide and macOS only resizes System
  Settings vertically, so the width is effectively the fixed default.
- `design-system/Theme.qml` (via `tokens.json`): `controls.sidebar.width`
  `220 → 192`; `controls.select.minWidth` `140 → 80` so popup controls size to
  their value like macOS instead of hogging the row.
- `apps/settings/SettingsShell.qml`: the sidebar pane uses
  `Theme.controls.sidebar.width` instead of a hardcoded `240`, so the sidebar
  is ~30% of the default window width as in the reference.
- `apps/settings/{Appearance,DesktopDock,Displays,Wallpaper}Pane.qml`: the pane
  root fills the detail-pane slot (`width: parent ? parent.width :
  implicitWidth`), and `AppearancePane`'s rows set `width: parent.width`. The
  detail pane is ~400 px at the default window, so the old 480 implicit width
  pushed the row controls past the card and clipped them.
- `apps/settings/AppearancePane.qml`: accent swatches `24 → 20` so the swatch
  row plus `Custom…` fits beside its label.

## Recommendations not applied (need a coordinated design pass)

- **Settings row height.** The reference toggle rows are ~31–33 units apart;
  ours are `settingsRow.height = 44`. Reducing to ~36 (with matching control
  heights) would match, but it touches every pane and the component tokens, so
  it should be one deliberate change with regenerated goldens.
- **Light-mode group card fill.** macOS group cards are `#f7f7f7` on a
  `#ffffff` content background (a *fill* difference). Our light
  `surfaceElevated` is `#ffffff`, so the card only reads via its border. Either
  use `surfaceMuted` (`#f8f6fa`, close to the reference) for the group fill, or
  give light `surfaceElevated` a light-gray value.
- **Accent color.** The reference selection/accent is macOS blue `#3474ee`;
  ours is the Dragonfruit berry `#b32a66`. That is a deliberate brand choice
  (and the accent is user-configurable via the Appearance pane), so it was left
  alone. Switch the default if a macOS-faithful default is wanted.
- **Control sizes** (toggle, segmented control, slider). The capture suggests
  these are slightly smaller than our tokens, but the color-based measurements
  are less reliable than the edge-based ones above; measure again with a
  cleaner capture before changing them.

## Pane-switch latency (fixed)

The reference-matched narrow window also exposed a latency bug: after the app
had been idle, switching panes could take seconds. The app's GUI thread handled
the click immediately (`selectPane`/`paneLoaded` in a few ms) but its render
thread waited ~10 s for a compositor frame.

Cause: the compositor is damage-driven and only sends `wl_callback.frame` from
[`post_repaint`](../../compositor/src/render.rs) when it renders. A client that
requests a callback after that render (Qt Wayland does, for every update) and
then waits cannot commit, so the compositor has no damage and never renders
again — a stall until some unrelated event wakes it.

Fix: after input, the compositor holds a short (~500 ms) ~60 Hz frame cadence
while a mapped window has an outstanding frame callback
(`DfState::frame_cadence_active` + `has_pending_frame_callbacks`, and the
nested no-damage branch flushes callbacks via
`render::send_frame_callbacks`). Outside that window the loop is a pure block,
so the idle budget is unchanged (`idle_steady_state_renders_zero_frames` and
`idle_menu_bar_contributes_zero_wakeups` still pass). Measured after 8 s idle:
`appframe=0 ms`, `present=2 ms`; the 10 s stall is gone.

The instrumentation used to find it (`DRAGONFRUIT_FRAME_TRACE=1
DF_SETTINGS_TRACE=1`) is documented in
[11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md).