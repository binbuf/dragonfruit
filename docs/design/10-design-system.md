# Design System

## Summary

We build the design system **before** building ten applications. It is the
single place where the desktop's visual identity lives — corner radii,
translucency, shadows, animations, typography, padding, focus rings, hover
behavior, reduced-motion behavior, and dark/light mode — and every
first-party application consumes it.

Qt Quick is particularly appropriate for this kind of custom animated
interface: Qt treats animation and transitions as first-class concepts and
provides its own scene graph and rendering engine.

## Component library

```text
Window
TitleBar
TrafficLights
Sidebar
Toolbar
SplitView
SettingsRow
SettingsGroup
Toggle
SegmentedControl
Slider
Select
Popup
ContextMenu
MenuBarMenu
SearchField
SourceList
Icon
Dialog
Sheet
Popover
ScrollView
```

## Token architecture

```text
primitive tokens    radius, color, elevation, spacing, type scale, duration
        │
semantic tokens    color-scheme roles (surface, elevated, accent, "on" colors),
        │           material roles (translucency, blur amounts)
        │
component tokens    per-component sizes, states, animation curves
```

Tokens are defined **once** in `design-system/tokens` and consumed
everywhere: QML singletons for the shell and apps, and a generated Rust
module for the compositor. Sharing the source is what makes
compositor-drawn SSD titlebars and first-party `TitleBar`s unable to drift
apart (see [05-window-decorations.md](05-window-decorations.md)).

The `component.elevation.{low,med,high,overlay}` group is the shared shadow
source (T-04.1a): each level resolves the primitive elevation scale into the
`blur`/`offsetY`/`layers` that both the `Shadow` QML component and the
compositor's window-shadow pass read, so surface shadow geometry is identical
across the process boundary by construction (see
[ADR 0011](adr/0011-elevation-shadow-tokens.md)).

The semantic `material` group (`chromeOpacity`/`chromeBlur`,
`popupOpacity`/`popupBlur`, `shadowOpacity`) is the shared source for surface
materials (T-04.2): the QML `Theme.material` group tints the shell chrome and
the compositor's backdrop pass reads the same values, per color scheme, so the
two sides cannot drift (see
[ADR 0013](adr/0013-backdrop-blur-pass.md)).

## Motion

Named curves and durations (`motion.spaces-switch`, `motion.dock-magnify`,
`motion.menu-open`) live in the token layer, not in component code. Two
rules:

- Every animation maps to a **reduced-motion variant** that removes
  translation/scale while keeping the state change legible.
- Gesture-driven transitions are **progress-based and interruptible**; a
  discrete "instant" code path is a bug (see
  [03-workspaces.md](03-workspaces.md)).

## Quality gates

- A component **gallery app** renders every component in every state,
  scheme, and motion variant; visual regression tests run against it.
- Keyboard navigation and AT-SPI roles are verified **per component**, not
  re-proven per application.
- Dark/light and reduced-motion variants are part of the definition of done
  for every component.

## Rules

- **Every first-party application consumes these components.** No app rolls
  its own titlebar, menu, or settings row.
- **One source of visual truth.** Tuning corner radii, translucency,
  shadows, spacing, or animation curves happens here, once, and propagates
  everywhere.
- **Reduced motion and accessibility are not afterthoughts.** The component
  library owns reduced-motion behavior, keyboard navigation, focus rings, and
  AT-SPI-compatible semantics (`QAccessible`).
- **Dark/light mode is a component-level concern,** not an app-level one.
- **No libadwaita.** First-party apps must have a strongly non-GNOME visual
  identity; libadwaita would put us in a recurring fight against another
  desktop's design language.

## Visual direction

Our visual baseline is **ours**, deliberately. "macOS-like" applies to the
interaction quality and mental model — animation timing, spatial organization,
control placement — not to Apple's bitmap output. Apple's own design keeps
moving (window shapes, menu-bar iconography, materials), so building against
"whatever macOS currently looks like" would make Apple an involuntary upstream
design dependency. See [14-risks.md](14-risks.md) for the IP constraints on
assets and branding.

## What the design system provides to siblings

- Compositor-drawn server-side decorations reuse the same visual tokens as
  first-party `TitleBar`s (see [05-window-decorations.md](05-window-decorations.md)).
- The `MenuBarMenu` component is the first-party path into the menu-broker
  (see [06-global-menu.md](06-global-menu.md)). Its normalized, JSON-serializable
  entry shape (`menuModel`) is the native publication contract: an app declares
  its menus once in that shape, and the shell's `MenuBar` consumes
  `applicationMenuItems` + `appMenuModel` unchanged (ADR
  [0041](adr/0041-native-menu-model-publication-shape.md)). Settings' single
  source is `apps/settings/SettingsMenu.qml`.
- Settings and Files are built entirely from these components (see
  [08-settings.md](08-settings.md), [09-files.md](09-files.md)).
