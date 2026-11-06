---
verify: make gates
---
# T201 — Connect menu-bar visibility settings

## Goal

Make the existing Menu Bar pane's background and auto-hide settings govern both the foreground bar and the compositor veil.

## Context (read first)

- Depends on T196 and T200. Read `docs/settings-keys.md`, `apps/settings/MenuBarPane.qml`, and `shell/src/shellcontroller.cpp::applyMenuBarPolicy`.
- `shell/menubar/MenuBar.qml` currently exposes `showBackground`. The shell forwards an auto-hide mode that currently lacks a working visibility path.
- `compositor/src/shell/layer.rs::visible_in_fullscreen` and `docs/design/adr/0210-fullscreen-chrome-suppression-and-scanout-counters.md` define the current fullscreen contract.

## Scope

- [ ] Apply `menu.showBackground` to both the foreground strip's fill and compositor veil/backdrop. Preserve active items, hit targets, keyboard entry and reservation while the bar is visible. Verify the old rail's background toggle cannot leave a sampled strip behind.
- [ ] Implement the existing `menu.autoHide` values on the owning output. `never` shows the bar in the desktop and fullscreen, `always` starts hidden with an edge/keyboard reveal, and `full-screen` shows it normally and reveals on demand in fullscreen. Keep a revealed bar available through active menu/focus interactions. On conceal, release its temporary regions and any overlay focus.
- [ ] Route policy changes through the existing Settings refresh path. Keep fullscreen visibility changes specific to the menu bar and use the existing chrome policy model. Document the resulting reservation and direct-scanout implications.

## Out of scope

- Changes to Dock auto-hide mechanics, unrelated menu settings, or an unmeasured macOS auto-hide animation curve.

## Design notes

A transient reveal does not resize a fullscreen client or leave stale reserved/interactive space. Respect lock and fullscreen on each output. A fullscreen bar the user explicitly keeps visible is a real composition requirement. Hidden fullscreen chrome must leave the output eligible for scanout. Keep keyboard and pointer entry consistent and settle to zero unnecessary frames. Use existing motion/accessibility policy for reveal transitions.

## Done when

- [ ] Add automated visibility tests for all modes across normal/fullscreen, hide/reveal, live toggles, second output, active menus and lock/unlock. Test the background toggle with a moving window and verify no residual veil or blur.
- [ ] `tst_menubar`, `tst_settings_menu_bar` and `make gates` pass. Update the key schema and the fullscreen policy docs. Fill in Hand-off.

## Hand-off

_(Implementation session records mode semantics and fullscreen/scanout observations.)_
