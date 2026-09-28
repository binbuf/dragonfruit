# T-17.1b nested navigation — reviewed state

This is the reviewed companion to the scripted capture in
`docs/captures/t17-navigation.*` (`make t17-navigation-capture`). The capture
harness and its evidence boundary are
[ADR 0160](../design/adr/0160-t17-navigation-capture.md); the per-behaviour
conformance is the headless `window_conformance` /
`shell_protocol_conformance` suites in `make e2e`.

## The navigation loop

Every path was driven with the compositor's synthetic-input harness against a
live nested session and observed through the compositor's read-only
introspection: `query spaces` (the active Space), `query grid` (the Mission
Control live-surface transform), `query wallpaper` (the in-flight Space
slide), and `query switcher` (the app-switcher entries, selection, and focus).
The raw report is `t17-navigation.txt`. The capture asserts every path, so a
run that does not reach the documented state fails.

| Path | Trigger | Observed | Still |
|---|---|---|---|
| Workspace switch | keyboard `Ctrl+Right` / `Ctrl+Left` | `active` Space `0 → 1` (then back `1 → 0`) | `t17-navigation-workspace-keyboard.png` |
| Workspace switch | trackpad three-finger swipe | caught mid-slide: `query wallpaper direction=1 progress=0.667 slots=2` (both Spaces' wallpapers on-screen); release commits `0 → 1`; the mirrored swipe returns to `0` | `t17-navigation-workspace-gesture.png` |
| Workspace switch | pointer: click a Mission Control Space card | click card 1 at `(960,86)` → `active` Space `0 → 1` | `t17-navigation-workspace-pointer.png` |
| Mission Control | keyboard `Ctrl+Up` | `grid progress=1.0`, `columns=2`, live surfaces `[w1 → (16,229,928,742)], [w0 → (1254,507,371,186)]` | `t17-navigation-mission-control-keyboard.png` |
| Mission Control | pointer: top-left hot-corner dwell | the same settled grid (`progress=1.0`, two live surfaces) | `t17-navigation-mission-control-pointer.png` |
| App switch | keyboard `Cmd+Tab`, release to commit | switcher active with both apps' live previews; `Cmd` release focused the next app (`focus 1 → 0`) | `t17-navigation-app-switch-keyboard.png` |
| App switch | pointer: click a live preview | click the non-focused app's preview → switcher closes and that app's window takes focus (`focus 0 → 1`) | `t17-navigation-app-switch-pointer.png` / `-committed.png` |

The initial still is `t17-navigation.png` (Space 0 with the first-party CSD
Settings client and the X11 `xmessage` client). The clip is
`t17-navigation.mp4`.

## Path coverage (pointer and keyboard)

| Feature | Keyboard | Pointer |
|---|---|---|
| Workspace switch | `Ctrl+Left` / `Ctrl+Right` (and `Ctrl+1..3` activate) | Mission Control workspace-strip card; three-finger swipe is the gesture arm |
| Mission Control | `Ctrl+Up` (toggles) | top-left hot-corner dwell opens the same overview |
| App switch | `Cmd+Tab` (open + commit on release) | click a live preview to commit |

Mission Control, workspace switch, and app switch each resolve to the one
compositor input vocabulary ([action.rs](../../compositor/src/input/action.rs),
[03-workspaces.md](../design/03-workspaces.md)): a keyboard shortcut, a
gesture, a hot corner, and a pointer act through the same
`process_input_event` router and the same overview/switcher state machines, so
the pointer and keyboard columns cannot diverge.

## Live visual check (run for this task)

The nested session was launched (`make demo`) and the whole navigation loop was
driven and captured. The desktop renders in every step: menu bar and Dock
present, the wallpaper and both clients composited, the Mission Control grid
showing the two live surfaces with the workspace strip, and the app-switcher
scrim + cards over the transformed previews. The vision model read the
app-switcher still conservatively (it also reads the committed T-06
`t06-app-switcher.png` the same way); the frame-level evidence is objective:
the switcher still's mean luminance drops (scrim) and the whole frame differs
from the resting desktop, and the `query switcher` transcript records live
preview rects for both apps. No blank, black, torn, or ghosted regions were
found in any still. The captures are `docs/captures/t17-navigation*.png`; the
transparent-host-shadow raw frames stayed in the scratch directory and were not
committed.

## Deviations and gotchas

- **Mission Control window selection by pointer is not synthesized in this
  capture.** The pointer path for Mission Control here is the hot-corner dwell
  that opens the overview; selecting a window from it is done by the shell's
  centered title-card row, not the compositor's live-surface rects, and the
  synthetic click did not reliably hit a card in a first probe. The
  compositor-level selection round-trip is already pinned headlessly by
  `overview_click_selects_and_focuses_the_live_representation` (T-05.2), and
  pointer workspace switching (the strip card) *is* exercised here.
- **Clicking a Mission Control Space card activates the Space but leaves the
  overview open.** The keyboard/gesture path dismisses it; `workspaceActivated`
  only sends `activateWorkspace` ([shellcontroller.cpp](../../shell/src/shellcontroller.cpp)).
  Recorded as observed, not changed by this verification unit.
- **The Mission Control keyboard and pointer stills are byte-identical.** Both
  triggers drive the one overview state machine to the same settled grid; that
  is the intended "one pipeline" behavior, not a capture bug.
- **A Space with no windows is a valid switch target.** The `Ctrl+Right` still
  shows the empty Space 1 (its own per-Space wallpaper palette); the switch is
  proven by the `query spaces` active flag, not by window presence.

## Reproduction

```bash
# host Wayland session, spectacle, python3+Pillow, ffmpeg (optional clip), built tree
make t17-navigation-capture          # or: bash scripts/capture-t17-navigation.sh
```

The headless conformance this capture sits over:

```bash
cargo test -p dragonfruit-compositor --test window_conformance \
    --test shell_protocol_conformance
```

## Follow-ups

- A stable synthetic locator for a Mission Control window card (the shell's
  centered title cards), so pointer selection can be captured as well; the
  compositor-level round-trip is already headless-pinned.