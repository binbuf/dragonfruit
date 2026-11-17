# 0287 — System Settings visual parity and reduced IA

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-17 |
| **Supersedes** | the chrome/IA clauses of [`24-system-settings.md`](../24-system-settings.md) that conflict with [`25-system-settings-visual-parity.md`](../25-system-settings-visual-parity.md); row ownership and absence rules remain |
| **Task set** | [`docs/tasks/system-settings-2/`](../../tasks/system-settings-2/README.md) |

## Context

The T263–T284 completion pass made System Settings truthful and testable. It
did not make the app match the 2026-11-17 UI/UX study, which reduces the
macOS-derived information architecture to the parts that have a Linux owner and
changes the shell geometry, density, and pane shapes.

The study also adds Mouse and Game Controllers, keeps the OS menu glyph choice
(`menu.osGlyph`) as a project addition, and deliberately leaves out Apple-only
features. Three conflicts could not be resolved from the mockup alone:

- Quick Settings was a top-level pane; the mockup treats Control Center as a
  shell panel and puts its customization under Menu Bar.
- Overview was a top-level pane; the mockup folds workspace/window overview
  controls into Desktop & Dock.
- Biometrics & Password had a real fprintd implementation but no product
  commitment; the mockup omits it.

The 2026-11-17 review decided to match the mockup's reduced IA, keep Screen
Saver as its own pane, remove Biometrics and Overview, and move Quick Settings
under Menu Bar → Menu Bar Controls. Mouse and Game Controllers stay top-level.
The OS menu glyph stays with Dragonfruit as the install default.

## Decision

1. **The mockup is the visual and IA target.** The reference checkout is
   `macos-design-review` at commit `b60d97c`, `mockups/settings`. Use its
   section order, labels, control types, and geometry. Do not copy its CSS
   approximations or SVG paths into production; use the design-system tokens
   and glyphs.
2. **Sidebar order** is the three-spacer list in
   [`25-system-settings-visual-parity.md`](../25-system-settings-visual-parity.md)
   §3.2. Screen Saver stays after Wallpaper. Mouse and Game Controllers stay in
   the bottom group. The update status row is a real provider-backed row.
3. **Removed panes.** Biometrics & Password is deleted. Overview is removed as
   a top-level pane; its controls move to Desktop & Dock → Workspace Overview.
   Quick Settings is removed as a top-level pane; it becomes Menu Bar → Menu Bar
   Controls. `settings://pane/mission-control`, `settings://pane/overview`,
   `settings://pane/control-center`, and `settings://pane/quick-settings` are
   aliases to the new locations. `settings://pane/biometrics` is refused.
4. **Renames.** Online Accounts → Internet Accounts (id stays
   `online-accounts`), Autofill & Passwords → Passwords & Keys, Backup →
   Backups, Mission Control → Workspace Overview, Stage → Workspace Focus.
5. **Chrome and density** follow the mockup: left-aligned title beside the
   history pill, 20 pt content inset, 24 pt top inset, 33 pt plain rows,
   53 pt icon rows, 32 pt sidebar rows, neutral selected row, no large header
   cards. The shared `TitleBar` keeps its centred default for other apps.
6. **Row ownership policy.** A mockup row is shipped only with an owner and a
   consumer, a delegate, a read-only owner, or a real absence state. Ownerless
   mockup rows are omitted and recorded in `SettingsMap` with an explicit
   reason. No new settingsd key is added without a consumer in the same change.
7. **OS menu glyph.** `menu.osGlyph` stays. Dragonfruit is the default on
   install and after Reset; Tux is the alternate. The UI is a radio group with
   actual-size menu-bar previews, not scaled-up icon tiles.
8. **One task per category.** The task set `system-settings-2` continues at
   T285 and splits the work by sidebar category so each change can be reviewed
   without losing context.

## Consequences

- The previous T283/T284 evidence set is superseded for the visual matrix by
  T302. The old captures remain historical; the new manifest is the current
  evidence.
- `SettingsPanes.qml`, `SettingsMap.qml`, `SettingsSearch.qml`, and
  `SettingsMenu.qml` change together in T285. A pane task must not fork the
  catalog or add a second sidebar list.
- `BiometricsPane.qml` and its tests are deleted in T299. The schema has no
  biometrics keys to migrate.
- `QuickSettingsPane.qml` is reused as the Menu Bar Controls sub-pane; the
  `quickSettings.*` keys and the Control Center panel do not change.
- The mockup's browser-only preview controls (theme/transparency/contrast
  toggles) are not product UI. They are not implemented.
- Apple-only rows and assets stay out. The reference checkout is not a build
  dependency and is not vendored.

## References

- [`25-system-settings-visual-parity.md`](../25-system-settings-visual-parity.md)
- [`24-system-settings.md`](../24-system-settings.md)
- [`08-settings.md`](../08-settings.md)
- [`settings-business-logic-map.md`](../settings-business-logic-map.md)
- `docs/settings-keys.md`
- [`docs/tasks/system-settings-2/`](../../tasks/system-settings-2/README.md)
