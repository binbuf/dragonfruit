# 0336 — The trusted shell renders the compositor-owned window menu

## Status

accepted

Supersedes [ADR 0002](0002-window-menu-ownership.md).

## Context

ADR 0002 made the SSD titlebar menu compositor-owned and kept the shell out
of it, accepting a flat, label-less draw because the compositor has no text or
icon renderer. The menu has shipped that way since T-01.4: rows are token fills
with no readable copy, tracked as the first-day usability defect G-06 in
[STATUS.md](../STATUS.md). The shell already owns the design-system
`ContextMenu` anatomy, the Tabler/Phosphor resources, the per-output chrome
surfaces, and the material roles for popups.

T329 asks for a readable menu without creating a second window state machine.
ADR 0002 itself anticipated a later move: if rendering moved to the shell, the
compositor must stay the source of truth and send presentation state only.

## Decision

The compositor remains the sole owner of the window menu. `WindowMenu` in
`compositor/src/window/menu.rs` is still the geometry, hit-testing, keyboard
model, and Space submenu; `DfState::window_menu_command` is still the only
path that applies Move to Space / Minimize / Zoom / Close. The compositor also
keeps pointer hover, keyboard navigation, Escape, click-away, and focus-loss
dismissal, so the menu works with no shell attached (headless conformance and a
shell restart).

The trusted shell renders one presentation projection of that state:

- `df_toplevel_manager` v16 adds a `window_menu` event (window id, output,
  surface/panel/submenu rects, scheme, highlight/submenu state), one
  `window_menu_row`/`window_menu_space` event per entry, and
  `window_menu_closed`, batched with `done`.
- The shell maps one transient `windowmenu` overlay surface on the menu's
  output. It anchors top-left at the output-local menu rect, reserves no zone,
  takes no keyboard focus, and has an empty input region. The compositor still
  routes all input; the popup is pixels only.
- The QML `Dragonfruit.WindowMenu` scene uses the shared `MenuRow` anatomy and
  `component.contextMenu` tokens. `MenuRow` gained an optional leading glyph
  for the command icons. The panel and a flipped submenu are separate material
  regions.
- `window_menu_activate`, `window_menu_activate_space`, and
  `window_menu_dismiss` are the programmatic activation path. They resolve
  through the same `WindowMenu` model a pointer or key press uses; the shell
  cannot invent a command.
- While a `windowmenu` surface is live the compositor suppresses its flat
  fallback draw. The shell destroys the surface on close, so a no-shell run
  keeps the fallback.

## Consequences

- ADR 0002 is superseded for rendering only; its ownership rule still holds.
- The protocol is additive at v16. Older shells keep the compositor fallback
  (still label-less); the lockstep set is unchanged.
- Menu copy, icons, and the Space list live in one row model
  (`compositor/src/window_menu.rs`) that the protocol, the fallback draw, and
  the conformance tests share.
- The menu is output-scoped (T324): its layer surface is created for the named
  output, and the second-monitor case uses that output's geometry.
- A future menu row or command must update the model, the protocol projection,
  and the QML view together; adding a second menu state machine remains
  forbidden.
