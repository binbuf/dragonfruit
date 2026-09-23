# 0002 — The window menu is compositor-owned; the shell only renders it

## Status

accepted

## Context

The SSD titlebar's window menu (Move to Space, Minimize, Zoom, Close) opens
from a right/Control-click on compositor-drawn chrome (T-01.4). The
design-system has a QML `ContextMenu`, and the shell renders all chrome, so
the menu could plausibly be shell-owned. But the compositor must be able to
open and drive it with no shell attached (the headless conformance tests, and
a shell crash/restart must not lose the affordance), and the four commands
must resolve through the compositor's single window state machine — the
project rule is that the shell never duplicates compositor state.

## Decision

The compositor owns the window menu: `compositor/src/window/menu.rs` is the
menu's geometry, hit-testing, and keyboard model, and `DfState::window_menu`
is the only live menu state. A row resolves to a `WindowMenuCommand` and is
applied by `DfState::window_menu_command`, the same primitives the traffic
lights and `df_toplevel` requests use. Rendering is a compositor draw pass
over design-system `component.contextMenu` tokens. The shell is not involved
in this unit. We rejected a shell-rendered menu (would need a new
compositor→shell event and could not be exercised headlessly, and would make
the menu unavailable without the shell) and rejected per-command protocol
requests from a shell menu (duplicates the state machine).

## Consequences

- T-14 (decoration themes) reuses `WindowMenu`/`WindowMenuCommand`; it must
  not add a second menu state machine.
- T-04 draws the menu's real labels and material; it replaces the flat
  `WindowMenu::render_elements` fill only, keeping the geometry and routing.
- If a later unit moves rendering to the shell, it must keep the compositor
  as the source of truth and send only presentation state, not a copy of the
  menu model.
