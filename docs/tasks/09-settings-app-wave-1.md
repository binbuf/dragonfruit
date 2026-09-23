# T-09 — Settings Wave 1: The App and Its Core Panes

> **Track, not a single slice.** This file is the design reference. It is executed as 6 one-session units: [T-09.1](units/036-t-09.1-settings-app-shell.md) · [T-09.2](units/037-t-09.2-appearance-pane.md) · [T-09.3](units/038-t-09.3-wallpaper-pane.md) · [T-09.4](units/039-t-09.4-desktop-dock-pane.md) · [T-09.5](units/040-t-09.5-displays-basic-pane.md) · [T-09.6](units/041-t-09.6-menu-model-absence-matrix-wave-sign-off.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 9 of 17 — the first flagship app |
| **Area** | `apps/settings/` · `shell/` (app chrome) |
| **Depends on** | T-08 |
| **Blocks** | T-14 (menu model), T-17 |
| **Legacy detail** | [legacy/16-settings-app.md](legacy/16-settings-app.md) (Wave 1) · [legacy/15-settingsd-settings-model.md](legacy/15-settingsd-settings-model.md) · [08-settings.md](../design/08-settings.md) · [10-design-system.md](../design/10-design-system.md) |

## Demo

```
open Settings from the Dock / the system menu
→ the sidebar, search, and window with our traffic lights and titlebar
→ Appearance: flip dark/light and accent; the whole desktop updates live
→ Wallpaper: pick a per-Space image; the Space background changes live
→ Desktop & Dock: auto-hide, magnification, size, position; the Dock updates
  live
→ Displays-basic: resolution and scale; the output reconfigures
→ every control is live (no Apply, no restart), keyboard-navigable, and
  degrades cleanly when a provider is absent
```

Capture: `docs/captures/t09-settings-wave-1.*`, plus a dark/light and
reduced-motion still.

## Why now

Settings is the first flagship app and the highest-value consumer of
settingsd. Wave 1 is deliberately the four panes that touch the core
experience (appearance, wallpaper, Dock, displays) — the panes that make the
desktop *yours*. It also proves the no-half-panes rule and the design-system
"no hand-rolled chrome" contract before Files repeats the pattern.

## Inherited and reused

- Design-system `Sidebar`, `SettingsRow`, `SettingsGroup`, `SegmentedControl`,
  `Toggle`, `SearchField`, `Toolbar`, `AppWindow`, `TitleBar`, `TrafficLights`.
- T-01's SSD titlebar (Settings is a first-party Tier-1 app: its own titlebar
  must match the compositor SSD exactly, per the legacy T-08 FR-3 reference
  test).
- settingsd (T-08) for Appearance/Wallpaper/Dock; the output API from the
  private protocol for Displays.
- The `--placeholders` Settings window is replaced.

## Scope

### In

1. **App shell**: sidebar, pane search (local), window chrome with traffic
   lights, live-apply everywhere, keyboard navigation, AT-SPI roles.
2. **Appearance**: dark/light, accent, (reduced motion lives here or in
   Accessibility — pick and document).
3. **Wallpaper**: per-Space image selection and fit; binds the T-05 wallpaper
   model.
4. **Desktop & Dock**: auto-hide, magnification, size, position,
   minimize-into-icon, indicators, recents.
5. **Displays-basic**: resolution, scale, rotation. Advanced color/night
   light/VRR are T-15/T-16.
6. **Menu model**: publish the app's menu model (consumed by T-14; until then
   the fixed app menu is enough).
7. **Absent-provider states** and the no-half-panes rule: a pane ships only
   when every control on it is functional.

### Out / explicitly deferred

- Waves 2–3 panes (T-15).
- Global menu integration toggle (T-14).
- Displays advanced (color management, night light, VRR) (T-15/T-16).

## Acceptance

- [ ] The demo runs and the captures are committed.
- [ ] Every Wave-1 control applies live within one interaction beat.
- [ ] The Settings titlebar renders identically to the compositor SSD
      reference (the legacy FR-3 diff test stays green).
- [ ] Keyboard walkthrough + AT-SPI roles for the shell and each pane.
- [ ] Absent-provider matrix passes for the shipped panes.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- QML tests per pane against the settingsd mock; routing-table test.
- Nested: live-apply capture; restart persistence.
- Visual: gallery/golden comparison and the SSD/titlebar diff.
- Regression: the T-08 D-Bus flip demo.

## Risks

- **Scope creep** is the named project risk; Wave 1 only, no half-panes.
- **Titlebar drift** between the app QML and the compositor SSD; the shared
  token reference test is the guardrail.
- **Search** is local pane search only; the app-index Spotlight equivalent is
  post-gate.

## Hand-off

- T-14 consumes the published menu model.
- T-15 adds the remaining panes without reworking the shell.
- T-17's loop uses Settings as the first-party app.
