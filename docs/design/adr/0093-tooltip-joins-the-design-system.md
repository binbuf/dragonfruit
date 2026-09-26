# 0093 — Tooltip joins the design-system component library

## Status

accepted

## Context

The Dock needs a macOS-style hover name label (T-14.7i), the design language
now follows the macOS Tahoe reference
([ADR 0091](0091-dock-tahoe-floating-glass-language.md)), and
[09-files.md](../09-files.md) already promises a tooltip for elided file names
that has no component behind it. There was no `Tooltip` in the design system;
`Popover` is the closest surface but takes focus and carries an anchor arrow,
which is wrong for a passive hover label. Adding a component to the library has
an established precedent (ADR
[0039](0039-slider-and-select-design-system-components.md)).

## Decision

- **A first-class `Tooltip` component** joins `design-system/components/`:
  pointer-anchored, non-focusable, single elided text, above/below placement,
  a dwell delay, and token-driven open/close motion. It never accepts keys or
  takes active focus.
- **Tokens own its values**: a `controls.tooltip` group (dwell, offset, radius,
  padding, max width, font size) in `tokens.json`, generated into `Theme.qml`.
- **The gallery and tests hold it to the library gate**: a gallery page plus a
  golden snapshot pair (light/dark, reduced motion) and `tst_design_system`
  cases, exactly like the other components.
- **Consumers opt in.** The Dock is first (T-14.7i); the Files list and
  menu-bar status items may adopt it later rather than each rolling their own.

## Consequences

- The component list in [10-design-system.md](../10-design-system.md) gains
  `Tooltip`, and the "every first-party UI consumes these components" rule now
  covers hover labels.
- Tooltips stay presentational: accessibility state remains on the owning
  control (the Dock entry's accessible name already carries it), so a tooltip
  never changes what a screen reader announces.
- Reduced motion opens/closes instantly; the tooltip never blocks pointer
  events to the entry underneath.
- A rich/multi-line preview is not part of this component; if needed it is a
  separate surface.