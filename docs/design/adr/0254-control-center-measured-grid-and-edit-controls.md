# ADR 0254 — Control Center measured grid, module registry, and Edit Controls

- **Status:** accepted
- **Date:** 2026-11-10
- **Task:** T235
- **Related:** ADR 0222, `docs/design/23-macos-structural-surface-review.md` §6,
  `docs/design/04-shell.md` Control Center

## Context

The T-11/T-15 Control Center was a curated list of full-width tiles. The
2026-11-08 structural review measured the reference as a nested dark-glass
grid: a 292 pt module field made of four 64 pt cells with 12 pt gaps, modules
spanning 1×1/2×1/2×2/4×1, expanded sheets that grow the outer panel, and a
separate 736×900 pt Edit Controls catalogue with a 245 pt sidebar and 22
categories. T227 had already made the module order and menu-bar-only set
durable, but the live panel still rendered the old list and there was no
management surface for pages, spans or the full inventory.

## Decision

1. **One registry drives both surfaces.** `ControlCenterRegistry.qml` is the
   single source for the 22-category catalogue, the module labels/owners/
   glyphs/spans, the measured 13-module default inventory, the 4-column
   packing algorithm, the page model and the menu-bar mapping. The live panel
   and `EditControls.qml` consume it; they cannot drift.
2. **The grid is measured, not approximated.** The field is exactly
   `4×64 + 3×12 = 292 pt`. Spans resolve to 64×64, 140×64, 140×140 and
   292×64. The default panel is a subset of the catalogue; the catalogue is
   not the panel.
3. **Expanded modules grow the outer panel.** The expanded sheet is 308 pt
   wide (the 292 pt field plus the 16 pt inset) and is a separate
   `controlCenterExpanded` region inside a separate
   `controlCenterExpandedOuter` plate (α=0.286/tint=0.257, measured). The
   collapsed outer plate stays α=0.265/tint=0.373 and the module capsules
   stay α=0.391/tint=0.317. Control Center remains dark glass in Light.
4. **Customization is one typed write path.** The Edit Controls surface writes
   `quickSettings.modules`, `page2`, `page3`, `pageCount`, `activePage`,
   `menuBarOnly`, `spans` and `perDisplay` through the shell's one settingsd
   client. The shell observes the same keys and re-renders the live panel.
5. **Menu-bar pinning is a separate target.** The catalogue's menu-bar toggle
   removes a control from every panel page and keeps its `menu.control.*`
   mark; it does not reuse the grid drag. Pointer drag between the panel and
   the menu bar remains an explicit OPEN row.
6. **Absence is explicit.** A module with no provider shows its unavailable
   state and reason; it is never invented. The catalogue carries the measured
   addable modules even when their provider is absent.
7. **One page is still one surface.** `quickSettings.perDisplay` is durable and
   the panel reads it, but the shell still creates one Control Center surface;
   per-display surfaces remain an explicit OPEN row.

## Consequences

- The measured grid, spans, expanded sheets, 22-category catalogue, pages,
  resizing and persistence are testable headlessly
  (`tst_controlcenter`, `tst_settings_quick_settings`) and captured by
  `make t235-control-center-capture`.
- The Settings pane and the Edit Controls surface write the same keys, so a
  change in either reaches the live panel through settingsd.
- Motion (open/close, expand/collapse, edit mode), pointer drag-to-menu-bar and
  per-display surfaces remain OPEN with narrower rows; no second Control
  Center implementation was added.
