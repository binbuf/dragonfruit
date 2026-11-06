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

- [x] Apply `menu.showBackground` to both the foreground strip's fill and compositor veil/backdrop. Preserve active items, hit targets, keyboard entry and reservation while the bar is visible. Verify the old rail's background toggle cannot leave a sampled strip behind.
- [x] Implement the existing `menu.autoHide` values on the owning output. `never` shows the bar in the desktop and fullscreen, `always` starts hidden with an edge/keyboard reveal, and `full-screen` shows it normally and reveals on demand in fullscreen. Keep a revealed bar available through active menu/focus interactions. On conceal, release its temporary regions and any overlay focus.
- [x] Route policy changes through the existing Settings refresh path. Keep fullscreen visibility changes specific to the menu bar and use the existing chrome policy model. Document the resulting reservation and direct-scanout implications.

## Out of scope

- Changes to Dock auto-hide mechanics, unrelated menu settings, or an unmeasured macOS auto-hide animation curve.

## Design notes

A transient reveal does not resize a fullscreen client or leave stale reserved/interactive space. Respect lock and fullscreen on each output. A fullscreen bar the user explicitly keeps visible is a real composition requirement. Hidden fullscreen chrome must leave the output eligible for scanout. Keep keyboard and pointer entry consistent and settle to zero unnecessary frames. Use existing motion/accessibility policy for reveal transitions.

## Done when

- [x] Add automated visibility tests for all modes across normal/fullscreen, hide/reveal, live toggles, second output, active menus and lock/unlock. Test the background toggle with a moving window and verify no residual veil or blur.
- [x] `tst_menubar`, `tst_settings_menu_bar` and `make gates` pass. Update the key schema and the fullscreen policy docs. Fill in Hand-off.

## Hand-off

**Status: done.**

### Changes

- `menu.showBackground` is now covered end-to-end: the QML fill and the
  compositor panel/veil share `compositorPanelActive`; turning it off clears
  the explicit region set on both the replacement veil and the legacy sampled
  rail, while items/hit targets/keyboard/reservation stay intact.
- Added the additive v4 `df_layer_surface.set_auto_hide`
  (`never`/`always`/`full_screen`). `applyMenuBarPolicy` forwards the settings
  value on the existing `changed`/`refreshed` path.
- `compositor/src/shell/menubar.rs` owns the policy and per-output reveal.
  `never` is visible on the desktop and fullscreen; `always` starts concealed
  and reveals from the top edge or Control-F2; `full-screen` is normal on the
  desktop and reveals on demand over fullscreen. Active menu/focus keeps it
  up; conceal drops the reservation/input/material and clears overlay focus.
- A concealed bar is filtered from `visible_chrome_surfaces` and
  `refresh_reserved_zones`, so it contributes no reserved zone, input hit
  area, veil, or blur panel. A visible `never` bar over fullscreen keeps the
  material pass (a real composition requirement, not scanout-eligible).
- Startup for `always` ignores the initial (0,0) seat position; only a real
  motion or keyboard focus reveals. Lock clears the reveal; unlock rests at
  the policy's concealed/visible state.
- Added `query menubar` diagnostics (mode + per-output
  fullscreen/revealed/active/visible/reserved-top).
- Docs: `docs/settings-keys.md` and `services/settingsd/src/schema.rs`
  consumer strings; `docs/design/02-compositor.md`, `04-shell.md`,
  `07-system-integration.md`, `docs/private-protocols.md`; ADR 0210 note and
  new ADR 0220.
- Moving-window residual veil: the T200 damage-commit test remains the
  moving-window half, and the T201 toggle tests assert an explicit empty panel
  set (no region/veil to leave behind) for both rails.

### Deviations

- Reveal is an immediate visibility flip rather than an animated curve. The
  task explicitly excludes an unmeasured macOS auto-hide curve; this also
  honors reduced motion trivially.
- The menu bar remains one all-output layer surface. Per-output reveal is a
  compositor filter over that surface rather than a second surface per output.

### Checks run

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 431
  passed.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance` —
  51 passed, including the new T201 edge/live-toggle/active/fullscreen test.
- `ctest --test-dir build -R '^tst_menubar$|^tst_settings_menu_bar$'` — pass
  (`tst_menubar` 52 cases).
- `cargo test -p dragonfruit-settingsd --test schema_doc` — pass.
- `make gates` — PASS (glass self-test, presets 165/0, menubar veil, e2e,
  QML 83/83).

### Remaining work

None for T201. Follow-ups: T202 menu-family glass below the veil; T208 native
DRM evidence for visible-`never` vs concealed scanout; a measured reveal
animation can replace the state flip later.
