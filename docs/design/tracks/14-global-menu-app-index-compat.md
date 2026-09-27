# T-14 — Global Menu, App Index, and Compatibility Bridges

> **Track, not a single slice.** This file is the design reference. It is executed as 11 one-session tasks: [T-14.1a](../../tasks/100-t-14.1a-app-index-identity-and-icons.md) · [T-14.1b](../../tasks/101-t-14.1b-app-index-events-and-recency.md) · [T-14.1c](../../tasks/102-t-14.1c-app-index-subscription.md) · [T-14.2a](../../tasks/103-t-14.2a-menu-broker-export-and-fixed-menu.md) · [T-14.2b](../../tasks/104-t-14.2b-menu-broker-accelerators-and-toggle.md) · [T-14.3](../../tasks/105-t-14.3-statusnotifier-appindicator-tray.md) · [T-14.4](../../tasks/106-t-14.4-dbusmenu-bridge.md) · [T-14.5](../../tasks/107-t-14.5-xdnd-bridge.md) · [T-14.6a](../../tasks/108-t-14.6a-strange-app-zoo-run.md) · [T-14.6b](../../tasks/109-t-14.6b-strange-app-zoo-fixes.md) · [T-14.7](../../tasks/110-t-14.7-retire-interim-paths.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 14 of 17 — third-party apps become first-class |
| **Area** | `services/menu-broker` · `services/app-index` · compat bridges · Xwayland XDnD |
| **Depends on** | T-09, T-13 |
| **Blocks** | T-15, T-17 |
| **Legacy detail** | [legacy/22-global-menu-broker.md](../../tasks/legacy/22-global-menu-broker.md) · [legacy/23-app-index.md](../../tasks/legacy/23-app-index.md) · [legacy/30-compatibility-bridges.md](../../tasks/legacy/30-compatibility-bridges.md) · [legacy/06-xwayland.md](../../tasks/legacy/06-xwayland.md) (XDnD) · [06-global-menu.md](../06-global-menu.md) |

## Demo

```
focus a Qt app that exports a menu → the menu bar shows its real File/Edit/…
  menus with live enable/disable; keyboard shortcuts route correctly
→ focus a non-exporting app → the fixed application menu only
→ a StatusNotifier tray app appears in the bar and its menu works
→ a DBusMenu app's menu works through the bridge
→ drag a file from a Wayland file manager to an X11 app and back (XDnD)
→ the strange-app zoo: Firefox (X11), Steam, an SDL game, xterm, a GTK4 app
→ the Dock resolves every one of them to the right identity and icon
```

Capture: `docs/captures/t14-compat.*`, including the zoo matrix.

## Why now

This is the "everything else on Linux works" slice. It depends on Settings
(menu-model publication and the global-menu toggle) and portals (Flatpak
apps), and it retires the last big interim hacks in the Dock and menu bar:
the `.desktop` resolver, the app-index stub, and the demo app menu. After
this slice, third-party identity, menus, tray, and drag-and-drop are real.

## Inherited and reused

- `GrabArbiter` and the shortcut engine's focused-app accelerator admission.
- The Dock's interim `.desktop` resolver and identity-miss logging (to be
  replaced, with the miss set as the heuristic input).
- The private protocols' `app_accelerator` event (emitted, never asserted).
- Xwayland window model integration and the `x11rb` conformance harness.
- Settings' published menu model.

## Scope

### In

1. **app-index** (`org.dragonfruit.AppIndex1`): identity resolution
   (`app_id`/`WM_CLASS`/heuristics), themed icons, install/uninstall/update
   events, a launch registry (`app_running`/`app_exited`), recency, and
   subscription; replaces the Dock's stub resolver.
2. **menu-broker**: global-menu model resolution for exporting apps, the
   fixed application menu with live state (Hide/Hide Others/Show All mapped
   to compositor window state), accelerator registration, and the
   global-menu on/off toggle from Settings.
3. **Compatibility bridges**: StatusNotifier/AppIndicator tray items in the
   menu bar; DBusMenu for apps that export it; the documented decoration-tier
   outcomes; **XDnD** across the X11/Wayland boundary (the known gap from the
   legacy T-06 work).
4. **Strange-app zoo**: run and document Firefox (X11), Steam, one SDL game,
   xterm, and a GTK4/Electron app; fix the identity/decoration/menu failures
   that surface.
5. **Retire interim paths**: delete the Dock's `.desktop` resolver and the
   `--placeholders` demo app menu.

### Out / explicitly deferred

- App Store/search integration (post-gate).
- Third-party decoration themes beyond the documented tier policy.
- Flatpak app management UI.

## Acceptance

- [ ] The demo runs and the zoo capture/matrix is committed.
- [ ] A Qt app's real menu appears with live state; Cmd+Q routes to the
      focused app.
- [ ] A tray item renders and its menu works; a DBusMenu app works.
- [ ] XDnD round-trips files in both directions.
- [ ] The interim resolver and demo menu are deleted.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- Unit: identity heuristics against the miss set; menu-model merge rules.
- Integration: D-Bus app-index queries; DBusMenu/tray bridges with mock apps;
  XDnD conformance with `x11rb`.
- Nested: zoo walkthrough capture.
- Regression: Dock identity/grouping tests.

## Risks

- **Zoo breadth** can absorb unlimited time; fix the top failures and record
  the rest as known outcomes rather than chasing every app.
- **XDnD** is upstream-adjacent (Smithay 0.7 has no bridge); budget for a
  custom bridge or document the gap explicitly at T-17.
- **Accelerator admission** must stay focus-scoped; do not let the broker
  install global grabs.

## Hand-off

- T-15's panes consume the app index (defaults, search later).
- T-17's gate uses the zoo matrix.

## Implementation note (T-14.6a)

The zoo run lives in `scripts/zoo/` and writes
`docs/captures/t14-zoo-matrix.{md,json}` plus `docs/captures/t14-zoo.png`
(`make zoo-run`). It records, per app: launch, raw identity, the app-index
desktop-id resolution, the SSD/CSD decoration tier, and the menu-broker tier.
Apps that cannot be installed are recorded as "not run"; two are stand-ins/
substitutes in this environment:

- **Steam** is not installable, so a raw X11 window is launched with Steam's
  real `WM_CLASS` (`Steam`/`steam`) against a faithful `steam.desktop`
  (`StartupWMClass=Steam`). It exercises the same tier-2 identity path.
- **The SDL game** is a small committed SDL2 sample (`scripts/zoo/sdl_zoo.c`).
  SDL2's Wayland backend connected but never mapped a window on the nested
  compositor, so the run uses SDL's X11 driver via Xwayland
  (`SDL_VIDEO_X11_WMCLASS=game.zoo.sdl`); the Wayland path is a known gap for
  T-14.6b.

All six rows pass identity resolution. Every zoo app falls back to the fixed
application menu (`tier: none`); none exports a native or DBusMenu menu, so the
global-menu tiers are exercised by the T-14.4 bridge tests with `--mock-menu`,
not here.

## Implementation note (T-14.6b)

The zoo's SDL row was the one fixable failure. The root cause was in the zoo
sample, not the compositor: `scripts/zoo/sdl_zoo.c` never presented a frame,
and SDL's Wayland backend does not attach its first buffer until the app draws,
so the toplevel never mapped. The sample now presents a frame each loop and the
zoo runs it on the nested Wayland socket (`SDL_VIDEODRIVER=wayland`,
`SDL_APP_ID=game.zoo.sdl`); the matrix records a Wayland SDL row with SSD
decoration.

The Wayland trace also showed SDL requesting a `wl_surface.frame` callback
before its first buffer. The compositor only answered frame callbacks for
mapped `Space` windows, so a client that paced its first paint on that callback
would stall. `DfState::send_pending_frame_callbacks` now answers a buffer-less
toplevel's callbacks from the commit handler; the headless regression test is
`sdl_style_pre_map_frame_callback_is_answered_and_then_maps` in
`compositor/tests/window_conformance.rs`. Remaining outcomes are unchanged
knowns: the Steam row is an X11 `WM_CLASS` stand-in, the global-menu tiers are
exercised by the T-14.4 mocks rather than the zoo, and XDnD is the documented
T-14.5 gap.

## Implementation note (T-14.7)

The last interim paths are gone. The Dock's local `.desktop` directory scan and
parser were deleted (`DesktopEntryIndex::scan`/`parse`/`defaultApplicationDirs`
and the absent-service fallback); the shell's entry cache is populated only
from `org.dragonfruit.AppIndex1::Enumerate` (T-14.1a), so identity and themed
icons have one owner. The shell's `demoAppMenu()` stand-in was deleted and
replaced by a real menu-broker client (`shell/src/menubrokerclient.{h,cpp}`):
`ShellController::applyFocusedApp` pushes `SetFocusedApp`/`SetWindowStates` and
reads `ResolveFocused`, rendering the broker's fixed application menu plus the
focused app's exported menus. With the service absent the shell keeps its own
`fixedApplicationMenu` (no exported menus) as before.

The dev/demo harness now starts `dragonfruit-app-index` and
`dragonfruit-menu-broker` alongside the shell (best-effort, session-bus only),
so the nested session exercises the real services; `make e2e` stays green.
Regression coverage: `tst_dockcore` (cache lookups, launch command,
`windowStatesJson`, `MenuBrokerClient::parseResolved`) and app-index's
`shipped_first_party_entries_are_launchable` (the migrated in-repo entry
check). The in-repo `org.dragonfruit.*` entries are still staged on a scratch
`XDG_DATA_DIRS` for the dev tree.
