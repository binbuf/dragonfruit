# ADR 0305 — Menu Bar restyle: OS glyph radio group, controls list, advanced sheet

- **Status:** accepted (T293a)
- **Date:** 2026-11-20

## Context

The mockup's Menu Bar is four blocks in one order: **Menu Bar Behaviour**,
**OS Menu Glyph** (a project addition), **Menu Bar Controls** (the status
items, each with its options), and **Allow in the Menu Bar** (third-party
items), then a footer. The pane before T293a rendered its own order: Behaviour,
a small two-plate glyph chooser, a controls list, a "Menu Bar Behavior" group
holding the Linux-only rows, and a separate Reset card.

Two problems went with that shape:

1. The glyph choice was two rows with 112 pt plates, not actual size, and the
   rows were radio buttons only by AT-SPI role — the pane had no group, the
   arrow keys did nothing, and a click did not move focus.
2. The Linux-only rows sat among the controls with no separation from the
   mockup's rows, so a user could not tell a desktop-specific setting from a
   mockup row.

## Decision

1. **The pane order is the mockup order.** Behaviour, OS Menu Glyph, Menu Bar
   Controls, Allow in the Menu Bar, footer. The Linux-only rows move into one
   **Advanced Menu Bar** sheet opened from the footer; their owner, key, and
   reason are unchanged, and `SettingsMap.rowSurface("menu-bar", …)` answers
   `dialog` for them so a deep link opens the sheet first.
2. **One footer action row.** Add Controls…, Customize Controls…, Advanced…,
   and Reset Menu Bar… share one derived width (ghost buttons), the same
   approach the Desktop & Dock footer uses.
3. **The OS glyph is one radio group of actual-size strips.** Each row is
   `SettingsGlyphOptionRow`: the strip is the real menu-bar height, at least
   184 pt wide, and the mark is `Theme.controls.menuBar.iconSize` with the
   bar's wallpaper-contrast shadow and hover/open plate. The strip never
   scales the glyph; it clips instead. `menu.osGlyph` stays the only key, and
   Dragonfruit is the default and the Reset value.
4. **Rows are not FocusScopes, so the group owns its keys.** `SettingsRow` and
   the new row inherit an `Item`, so a `Keys` handler installed on a row does
   not receive key events (the event reaches the focused item, then its
   parents, and a `Keys.onPressed` declared on the focused item itself is what
   sees it). The group therefore installs one adapter item
   (`glyphKeyAdapter`) with `handleKey()`, and the row forwards its keys to it.
   Space/Return confirm the *focused* row; Up/Down/Left/Right move the
   selection and the roving focus, like `SettingsRadioGroup`. The row's
   `TapHandler` selects too; the shell's focused view is what receives key
   events, so the adapter is the single implementation both a key handler and
   a test can call.
5. **Customize Controls… is an alias, not a dead action.** The shell mounts
   `QuickSettingsPane` with `subPaneId: "controls"` when
   `menu-bar/controls` is open (the existing alias target; `PaneLoader`
   re-resolves the body for the same pane id). Rows whose section is
   `Quick Settings` answer `subpane` and land there. T293b replaces the alias
   with the in-place Menu Bar Controls sub-pane and drops the branch.
6. **A same-pane section request opens the alias.** The shell's pane-body
   `Connections` answers `sectionRequested(paneId, section)` with
   `openSubPane(section)` when `paneId` is the current pane, so the footer's
   Customize Controls… lands on the `menu-bar/controls` body without leaving
   the Menu Bar pane.

## Consequences

- The Menu Bar Controls list is the status-item list (Clock, Search, Wi-Fi,
  Bluetooth, Battery, Focus, Sound, Accessibility, Quick Settings, Workspaces,
  with Display as the disabled row). Now Playing, Text Input, Backups, and
  Timer are `omitted` map rows with reasons; they never render a fake toggle.
- Reset Menu Bar… writes the behaviour keys, the glyph, the clock options,
  every control visibility, the per-control options, the tray allowances, and
  the Linux-only rows.
- `SettingsGlyphOptionRow` is a pane-level component, not a design-system one,
  because the glyph strip is the Menu Bar's vocabulary; T286's shared
  components stay unchanged.
- T293b must not keep the `menu-bar/controls` alias branch once the in-place
  sub-pane lands; the alias stays for the legacy `control-center` and
  `quick-settings` deep links.
