# T-17.1a nested window loop — reviewed state

This is the reviewed companion to the scripted capture in
`docs/captures/t17-window-loop.*` (`make t17-window-loop-capture`). The
capture harness and its evidence boundary are
[ADR 0159](../design/adr/0159-t17-nested-window-loop-capture.md); the
per-behaviour conformance is the headless `window_conformance` /
`xwayland_conformance` / `milestone_e2e` suites in `make e2e`.

## The loop

Every step was driven with the compositor's synthetic-input harness against a
live nested session and observed through `query decorations` / `query identity`
(the raw report is `t17-window-loop.txt`).

| Step | Action | Observed | Still |
|---|---|---|---|
| Launch / appear | the demo maps the Qt SSD (`kcalc`), CSD (`Settings`), and X11 (`xmessage`) clients | all three tracked; `kcalc`/`xmessage` carry a compositor titlebar, `Settings` does not | `t17-window-loop.png` |
| Focus | left-click the Qt SSD titlebar | window activated | `t17-window-loop-focused.png` |
| Move | drag the titlebar by (90,70) | content `(664,355) → (754,425)`, exact | `t17-window-loop-move.png` |
| Zoom | double-click the titlebar | `floating → zoomed (0,68,1920,1046)` (fills the usable area below the 28 px menu bar and 40 px titlebar) | `t17-window-loop-zoom.png` |
| Minimize | click the yellow traffic light | `floating → minimized`; the Dock shows the running/minimized entry | `t17-window-loop-minimized.png` |
| Restore | compositor `restore` primitive (the Dock-tile path; see deviations) | `minimized → floating (754,425)` | `t17-window-loop-restored.png` |
| Close | click the red traffic light | window removed from `query decorations` | `t17-window-loop-closed.png` |

## App tiers

| Client | Tier | Decoration report | Still |
|---|---|---|---|
| `kcalc` (third-party Qt/KDE) | SSD | `ssd=True`, titlebar `(664,315,640,40)` | `t17-window-loop.png`, `-move.png` |
| Dragonfruit `Settings` (first-party Qt) | CSD | `ssd=False`, titlebar `(0,0,0,0)`, content stays full size | `t17-window-loop-csd.png` |
| `xmessage` (Xwayland) | SSD | `ssd=True`, titlebar `(1558,40,320,40)` | `t17-window-loop-x11.png` |

The third-party Qt client is `kcalc` because the first-party apps are CSD by
design (`apps/settings/SettingsWindow.qml`). The X11 client is the demo's
`xmessage`. A CSD client is "unaffected" in the strict sense: it never
receives a compositor titlebar and keeps its own full content size.

## Live visual check (run for this task)

The nested session was launched (`make demo`), kcalc was started against the
private socket, and the whole loop was driven and captured. The vision model
found the desktop fully composited in every step — menu bar and Dock present,
the calculator window with red/yellow/green traffic lights, calculator content
rendered, and no blank, black, torn, or ghosted regions; the CSD and X11 crops
showed the expected decoration on each. The captured stills are
`docs/captures/t17-window-loop*.png`; the transparent-host-shadow raw frames
stayed in the scratch directory and were not committed.

## Deviations

- **Restore uses the compositor primitive, not a synthesized Dock-tile click.**
  kcalc is an unpinned running app, so minimizing it makes its tile appear and
  the pinned tiles re-center; a Dock-band pixel diff then points at several
  regions and clicking one can launch a pinned app (observed: an extra Files
  window) instead of restoring. The restore therefore goes through the same
  lifecycle primitive the tile invokes. The Dock-tile click is exercised by the
  human walkthrough and the headless T-01/T-02 geometry suites; making the
  agent capture click the tile needs a stable tile locator, which is a separate
  tooling task.
- **No new headless test.** The loop's behaviours are already pinned headlessly
  in `make e2e`; this unit is a verification capture over those suites.

## Reproduction

```bash
# host Wayland session, spectacle, python3+Pillow, built tree
make t17-window-loop-capture          # or: bash scripts/capture-t17-window-loop.sh
```

The headless conformance this capture sits over:

```bash
cargo test -p dragonfruit-compositor --test window_conformance \
    --test xwayland_conformance --test milestone_e2e
```

## Follow-ups

- A stable Dock-tile locator so a future verification unit can synthesize the
  restore-from-Dock click (T-17.5a/T-17.2 may want it).