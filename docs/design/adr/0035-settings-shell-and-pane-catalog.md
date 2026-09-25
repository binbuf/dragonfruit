# 0035 — The Settings shell owns one pane catalog and ships no half panes

## Status

accepted

## Context

T-09 builds the first flagship app as one shell unit (T-09.1a) plus one unit
per Wave-1 pane (T-09.1b…T-09.5). The shell must define the sidebar information
architecture, the window chrome, and local search once, and the pane units must
add their bodies without reworking the shell. The track also carries the
**no-half-panes rule** ([09-settings-app-wave-1.md](../tracks/09-settings-app-wave-1.md)):
a sidebar row appears only when its pane is complete and every control on it
works. The reference IA is fixed and ordered
([System_Preferences.md](../../reference/System_Preferences.md)), so the shell
can own the list even though most panes are not built yet.

## Decision

- **`apps/settings` is a reusable QML module (`Dragonfruit.Settings`)** — a
  static library plus a thin executable — mirroring the design-system gallery
  split, so the app and its headless QML test render the same surfaces.
- **One ordered catalog singleton, `SettingsPanes.catalog`**: every reference
  pane as `{ id, title, icon, description, shipped }`, in the captured order.
  `shipped` is the no-half-panes gate; the sidebar lists only
  `SettingsPanes.shippedPanes`. Local search (`SettingsPanes.filter`) runs over
  the shipped set only. A pane unit flips its entry to `shipped: true` when its
  body and controls land; it never adds a row earlier.
- **The shell owns navigation and chrome, not pane content**: the design-system
  `TitleBar`/`TrafficLights`, `Sidebar`, `SearchField`, `Toolbar`, and
  `SettingsGroup`; back/forward history and Alt+Left/Right; the header card.
  Settings is a Tier-1 app and draws its own titlebar on a frameless window.
- Rejected: one `Loader` per pane with placeholder controls (dead controls
  violate the rule); listing unshipped rows greyed out (the rule says rows
  appear as their panes ship).

## Consequences

- T-09.1b…T-09.5 add a pane by writing its body, wiring settingsd, and flipping
  its catalog `shipped` flag (and registering its `Icon` glyph if new); they do
  not touch the sidebar, search, chrome, or history.
- Until a pane ships it is not reachable from the shell, so the Wave-1 shell
  correctly advertises fewer rows than the final IA. T-09.6b records the
  absence matrix and the wave captures.
- The pane `id` is the stable key shared by the catalog, the sidebar, history,
  and the future per-pane tests; renaming one is a coordinated change.