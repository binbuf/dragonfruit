# T-08 — Design System: Tokens, Components, Motion, Gallery

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `design-system/` |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) |
| **Blocks** | [T-09](09-menu-bar.md) · [T-10](10-dock.md) · [T-13](13-window-decorations-ssd.md) · [T-16](16-settings-app.md) · [T-18](18-files-app.md) · [T-21](21-control-center.md) |
| **Estimate** | XL |
| **Design docs** | [10-design-system.md](../design/10-design-system.md) · [05-window-decorations.md](../design/05-window-decorations.md) |

## Summary

Build the design system **before** building ten applications: the token
architecture (primitive → semantic → component), the first-party Qt Quick
component library, the motion language, the component gallery with visual
regression tests, and per-component accessibility guarantees. One source of
visual truth for the entire desktop.

## Background

The design system is the single place where the desktop's visual identity
lives — corner radii, translucency, shadows, animations, typography,
padding, focus rings, hover behavior, reduced-motion, dark/light
([10-design-system.md](../design/10-design-system.md)). Sharing token
**source** between QML and the compositor is what makes compositor-drawn SSD
titlebars and first-party `TitleBar`s unable to drift apart.

## Scope

### In scope

1. **Component library** (all of it, in
   [10-design-system.md](../design/10-design-system.md)):
   `Window`, `TitleBar`, `TrafficLights`, `Sidebar`, `Toolbar`, `SplitView`,
   `SettingsRow`, `SettingsGroup`, `Toggle`, `SegmentedControl`, `Popup`,
   `ContextMenu`, `MenuBarMenu`, `SearchField`, `SourceList`, `Icon`,
   `Dialog`, `Sheet`, `Popover`, `ScrollView`.
2. **Token architecture**:
   ```text
   primitive tokens    radius, color, elevation, spacing, type scale, duration
           │
   semantic tokens    color-scheme roles (surface, elevated, accent, "on"
           │           colors), material roles (translucency, blur amounts)
           │
   component tokens   per-component sizes, states, animation curves
   ```
   Tokens are defined **once** in `design-system/tokens` and consumed
   everywhere: QML singletons for shell/apps, and a **generated Rust module
   for the compositor** (SSD titlebars).
3. **Motion language**:
   - Named curves/durations live in the token layer, not component code:
     `motion.spaces-switch`, `motion.dock-magnify`, `motion.menu-open`, …
   - **Every animation maps to a reduced-motion variant** that removes
     translation/scale while keeping the state change legible.
   - **Gesture-driven transitions are progress-based and interruptible**;
     a discrete "instant" code path is a bug.
4. **Component gallery app** rendering every component in every state,
   scheme, and motion variant; visual regression tests run against it.
5. **Accessibility per component**: keyboard navigation and AT-SPI roles
   verified **per component**, not re-proven per app (`QAccessible`).
   Dark/light and reduced-motion variants are part of every component's
   definition of done.
6. **Materials**: translucency + blur levels expressed as tokens, matching
   what the compositor's blur pass renders behind chrome.

### Out of scope

- App-specific UIs (built *from* these components in T-16/T-18).
- The compositor's SSD titlebar renderer itself (T-13) — this ticket feeds
  it tokens and specs.
- Desktop wallpaper art (compositor-rendered, T-05/T-14).

## Requirements

- FR-1: Every listed component ships with: all states, dark + light,
  reduced-motion variant, keyboard navigation, AT-SPI role mapping, and
  gallery coverage.
- FR-2: Token singletons available in QML and generated Rust; a CI check
  that the two are generated from the same source (no hand-copied values).
- FR-3: `TitleBar`/`TrafficLights` in an app window and a compositor-drawn
  SSD titlebar render **identically** at the same tokens (screenshot diff —
  the [05-window-decorations.md](../design/05-window-decorations.md)
  "unable to drift apart" rule).
- FR-4: `MenuBarMenu` component exposes a declarative menu model suitable
  for publication to the menu-broker (T-22 consumer).
- FR-5: Reduced-motion variants pass the Phase-2 exit criterion (part of
  the 30-second loop with reduced motion on).
- FR-6: Visual regression suite runs in CI on the gallery (headless Qt
  scene graph rendering).

## Acceptance criteria

- [ ] Gallery app demonstrates every component × state × scheme × motion
      variant.
- [ ] Zero hand-rolled titlebars/menus/settings rows in first-party apps
      (lint rule in each app's CI).
- [ ] Token generation pipeline: edit once, regenerate QML + Rust.
- [ ] AT-SPI audit of the gallery passes (keyboard-only walkthrough +
      `atspi` role dump per component).

## Test plan

- Visual regression on the gallery across light/dark/reduced-motion.
- Keyboard-only scripted walkthrough of every interactive component.
- Token-generation round-trip test (source → QML singleton + Rust module).

## Risks / open questions

- Original visual direction only: interaction quality yes, Apple bitmaps
  no ([14-risks.md](../design/14-risks.md)); art direction review is part
  of this ticket.
- Blur/translucency tokens must match what the compositor render pass can
  actually produce — joint tuning session with T-02/T-13 owners.
- No libadwaita — ever ([10-design-system.md](../design/10-design-system.md)).
