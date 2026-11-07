# 0220 — Menu-bar auto-hide is per-output compositor visibility

- **Status:** Accepted (T201)
- **Date:** 2026-11-06
- **Context:** [02-compositor.md](../02-compositor.md),
  [04-shell.md](../04-shell.md), [07-system-integration.md](../07-system-integration.md),
  ADR [0135](0135-menu-bar-pane-and-tile.md),
  ADR [0210](0210-fullscreen-chrome-suppression-and-scanout-counters.md),
  ADR [0219](0219-menubar-no-blur-veil.md)

## Context

`menu.autoHide` (`never`/`always`/`full-screen`) was stored policy: the Menu
Bar pane wrote it and the Control Center tile projected it, but the shell
chrome surface stayed pinned. ADR 0210 then hid every persistent `top` surface
over a fullscreen client, so a `never` preference could not be honored and a
`full-screen` reveal had no path. The menu bar is one all-output layer surface,
but the reveal must be per output: revealing on one display must not float the
bar onto a second.

## Decision

- **The compositor owns the visibility state.** `df_layer_surface` gains the
  additive v4 `set_auto_hide` request (`never`/`always`/`full_screen`). The
  shell forwards the settings value from `applyMenuBarPolicy` through the
  existing Settings `changed`/`refreshed` path. Only the `menubar` namespace
  consumes it; the Dock keeps its own auto-hide.
- **One pure rule, one reveal state.** `shell/menubar.rs` holds
  `MenuBarVisibility` (mode plus a per-output revealed set) and the pure
  `menu_bar_visible(mode, fullscreen, revealed, active)`. `never` is always
  visible; `always` is `revealed || active`; `full-screen` is
  `!fullscreen || revealed || active`. `active` is an open menu popup or
  menu-bar keyboard focus, so a revealed bar stays available through the
  interaction even after the pointer leaves the edge.
- **Edge and keyboard reveal.** Pointer motion at the top `menuBar.height`
  band reveals on the output under the pointer before hit-testing, so the same
  motion reaches the bar. Control-F2 (`InputAction::FocusMenuBar`) reveals the
  focused output and transfers keyboard focus; Escape/release returns focus to
  the window. Moving below the band conceals unless `active`.
- **Concealment releases everything.** A concealed bar is filtered out of
  `visible_chrome_surfaces` per output, so it contributes no reserved zone,
  input hit area, material region, veil, or blur panel. Its reveal is cleared
  on lock; unlock starts from the policy's resting state. A revealed bar keeps
  the normal 28 pt reservation and input strip; a transient reveal therefore
  never resizes a fullscreen client, because fullscreen geometry ignores the
  usable area.
- **`menu.showBackground` stays one switch.** The shell already clears the QML
  fill and declares an explicit empty panel set when it is off; T201 keeps the
  same path for the replacement veil and the legacy sampled rail, so no stale
  panel survives a live toggle or a moving window under the bar.
- **No unmeasured animation.** Reveal is an immediate visibility flip. This
  honors the reduced-motion policy trivially and leaves the measured
  auto-hide curve out of scope.

## Consequences

- `never` over fullscreen is a real composition requirement: the bar and its
  veil stay visible and the output is not a direct-scanout candidate. Hidden
  fullscreen chrome leaves no panel, so `direct_scanout_candidate` can pass.
- The menu-bar reservation is now per output and live: `query menubar` reports
  `reserved-top=0` while concealed and the 28 pt strip while visible.
- The shell no longer needs a second visibility source; the compositor's
  `MenuBarVisibility` is the sole reveal owner, and the QML `visibleIn()` rule
  exists only for scene tests/introspection.
- `tst_menubar` covers the QML fill/panel toggle and the three mode rules;
  `shell_protocol_conformance` covers edge reveal, live toggles, active focus,
  fullscreen, and the reservation/no-panel state. `tst_settings_menu_bar`
  continues to prove the pane writes the keys.
