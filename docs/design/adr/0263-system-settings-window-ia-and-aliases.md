# 0263 — System Settings shell is one catalog, grouped, with alias links

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-14 |
| **Extends** | ADR [0035](0035-settings-shell-and-pane-catalog.md), [0164](0164-settings-category-tile.md), [0225](0225-first-party-app-chrome-presets.md), [0231](0231-executable-settings-map.md), [0262](0262-system-settings-completion-contract.md) |

## Context

T263 moves the System Settings shell from the 800×640 / flat-192 pt prototype
to the measured the reference design window. The move touches four contracts that later
pane tasks would otherwise re-invent:

- the sidebar needs groups, but must stay the one ordered catalog;
- Storage moves inside General while `settings://pane/storage` must keep
  working;
- VPN becomes an optional Connectivity entry, but the pane body lands in T272;
- the window remembers its last pane and geometry through settingsd.

## Decision

1. **One catalog, group metadata.** `SettingsPanes.catalog` keeps its id,
   title, icon, owner, deep link, and absence fields and gains `group`.
   `SettingsPanes.groups` is the ordered group table. `grouped()` projects the
   filtered catalog into the sidebar sections; there is no second pane list.
2. **Aliases resolve through the catalog.** An entry may be `hidden` with
   `aliasOf`/`subPane` (Storage → General → Storage) or a navigation link with
   `linkTo`/`anchor` and an `optional` host predicate (VPN → Network). `resolve()`
   returns the pane the shell opens; `isVisible()` follows the target for a
   hidden alias and the link's own row for an optional link. A deep link uses
   the same resolver, so the old `settings://pane/storage` link lands on
   General → Storage.
3. **The split toolbar is two surfaces.** The sidebar column spans the full
   window height and paints `settingsSidebar` under the titlebar (traffic
   lights included). The detail column owns a 52 pt `settingsToolbar` sheet
   with Back/Forward and the pane title centred over the detail pane. The
   compositor/taskbar title is the current pane or open sub-pane, never a
   constant.
4. **Persistence is six settingsd keys.** `settings.lastPane`,
   `settings.window.width`, `.height`, `.x`, `.y`, and `.positioned` (schema
   revision 27, owner and consumer `apps/settings`). First launch opens
   General; a deep link wins over the restored pane. Geometry is written only
   in the windowed state and debounced; `positioned` keeps a first-run
   compositor placement from being overwritten by (0,0).
5. **VPN is a link until T272.** The optional Connectivity row appears only
   when NetworkManager reports a VPN profile and targets Network's VPN group.
   T272 replaces it with a real pane; it does not add a second Network model.
6. **A stationary press is routed at the live pointer position.** The
   compositor re-resolves pointer focus with `surface_under` before a button
   press when no grab or overlay owns input. A window that mapped (or a list
   that scrolled) under a resting pointer therefore receives the first click;
   no QML focus/motion workaround is added.

## Consequences

- T264 (history/deep links) and T265 (search) reuse `resolve()`/`isVisible()`
  instead of special-casing Storage or VPN.
- T272 owns the real VPN pane; until then the link is a navigation shortcut,
  not a half pane. Online Accounts is always listed and T279 owns its absent
  state.
- Any new sidebar entry must add group metadata and (if it is an alias) a
  resolver branch; `tst_settings_map` fails on catalog/map drift and
  `tst_settings_shell` covers the measured sizes, groups, aliases, and
  persistence.

## References

- [24-system-settings.md](../24-system-settings.md) §3.1–3.2
- [08-settings.md](../08-settings.md) window-chrome section
- [settings-keys.md](../settings-keys.md) revision 27
