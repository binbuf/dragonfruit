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
