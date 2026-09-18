# T-30 — Compatibility Phase: DBusMenu Bridge, StatusNotifier, Decoration Themes, Xwayland Zoo

| | |
|---|---|
| **Phase** | 6 · Compatibility |
| **Area** | `services/menu-broker/` (bridge) · `shell/menubar/` (tray) · `compositor/` (SSD theming) · app-zoo harness |
| **Depends on** | [T-22](22-global-menu-broker.md) · [T-09](09-menu-bar.md) · [T-13](13-window-decorations-ssd.md) · [T-06](06-xwayland.md) |
| **Blocks** | Phase-6 exit |
| **Estimate** | L |
| **Design docs** | [13-roadmap.md](../design/13-roadmap.md) · [06-global-menu.md](../design/06-global-menu.md) · [04-shell.md](../design/04-shell.md) · [05-window-decorations.md](../design/05-window-decorations.md) · [14-risks.md](../design/14-risks.md) |

## Summary

Third-party compatibility work: the DBusMenu/AppMenu global-menu bridge,
StatusNotifierItem/AppIndicator tray support in the menu bar, third-party
decoration themes, GTK left-side-button steering, and the tracked
"strange Xwayland applications" zoo with its test harness.

## Background

The roadmap's compatibility phase: "third-party decoration themes,
DBusMenu global-menu bridge, StatusNotifier/AppIndicator support, strange
Xwayland applications" ([13-roadmap.md](../design/13-roadmap.md)). Phase-6
exit: "a DBusMenu-exporting Qt app shows a global menu; a StatusNotifier
tray item renders in the menu bar."

## Scope

### 1. DBusMenu / AppMenu global-menu bridge

- Bridge third-party DBusMenu exports into the menu-broker representation
  (Tier 2 of the broker's priority order,
  [06-global-menu.md](../design/06-global-menu.md)).
- Accelerator strings parsed by the broker and registered with the
  compositor **while the owning window is focused** (T-22 FR rules).
- Hard rules carry over: never remove the app's internal menu; show
  globally only on successful export; GTK-under-Wayland apps that don't
  export stay Tier 3 (app name only) — never fight it
  ([06-global-menu.md](../design/06-global-menu.md)).
- Bridge live updates: about-to-show, item activation, dynamic
  enable/checked state, icons.

### 2. StatusNotifierItem / AppIndicator tray

- Menu bar's system area hosts SNI exports "through a bridge, shipped in
  the compatibility phase" ([04-shell.md](../design/04-shell.md)) — the
  de-facto Linux tray standard (KDE/Ayatana lineages).
- **Third-party items get the same sizing, hover, and dark/light treatment
  as first-party items** — they live inside the menu bar's slot system
  (T-09's unified item styling), never a separate tray strip.
- Support: icons (incl. attention states), tooltips, menus (reusing the
  menu engine), activate/secondary-click events.

### 3. Third-party decoration themes + GTK steering

- Ship the mechanism for third-party SSD **decoration themes** over the
  design-system token model ([05-window-decorations.md](../design/05-window-decorations.md)
  lists them as a compatibility work item).
- Document + configure **GTK settings that steer CSD buttons left** so
  cooperative GTK apps place controls on the left (Tier 3 improvement —
  documented, configured by our packages, never forced).
- Xwayland windows are already Tier 2 (T-06/T-13) — verify the zoo cases
  stay correct under theming.

### 4. Xwayland application zoo

- A **tracked app zoo** ([14-risks.md](../design/14-risks.md)): fixture
  list of known-difficult X11 apps (older SDL games, Java/Swing apps,
  multi-window tools with odd transient hints, apps with override-redirect
  menus) plus a scripted exercise harness.
- Robustness-first handling: never crash on protocol-bending behavior;
  file per-app bugs with traces; fix categories, not symptoms.
- DRM fractional-scaling policy for Xwayland (decided in T-06) validated
  against the zoo.

### Out of scope

- Electron/Chromium CSD redesign (we don't inject — the accepted
  compromise stands, [14-risks.md](../design/14-risks.md)).
- Flatpak portal compatibility (that's T-27, already Phase 5).

## Requirements

- FR-1: **Phase-6 exit:** a DBusMenu-exporting Qt app (e.g., a configured
  Qt/KDE app) shows a working global menu with live state and
  accelerators.
- FR-2: **Phase-6 exit:** a StatusNotifier tray item renders in the menu
  bar with menu + attention support, styled identically to first-party
  items.
- FR-3: Bridge robustness: exporter crash mid-menu never crashes broker or
  menu bar (degrades to app name).
- FR-4: Decoration theme validation: a sample third-party theme applies
  to SSD windows without drift between SSD and first-party titlebars
  (token contract holds).
- FR-5: GTK steering document + packaged defaults; a GTK CSD app shows
  left-side buttons on a fresh install (where the toolkit cooperates).
- FR-6: Zoo harness runs in CI (nested/headless where possible); new
  regressions file as zoo entries.

## Acceptance criteria

- [ ] Both Phase-6 exit demonstrations recorded and reproducible.
- [ ] Zoo harness integrated; current zoo list green (or explicitly
      waived per-app with rationale).
- [ ] Robustness drills (crash mid-export, malformed DBusMenu trees)
      green.

## Test plan

- Bridge conformance vs a reference Qt exporter + the Ayatana/SNI test
  items.
- Zoo matrix in nested and DRM VM; theming round-trip.

## Risks / open questions

- DBusMenu dialect variance (icons-in-menu, shortcuts syntax) — tolerant
  parser with logs (shared with T-22 risk).
- Keep scope disciplined: this phase exists to make the ecosystem work,
  not to chase every app; the two accepted compromises are the boundary.
