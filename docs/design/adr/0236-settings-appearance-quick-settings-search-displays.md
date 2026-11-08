# 0236 — T227 Settings panes: Appearance completion, Quick Settings, Search, Screen Saver, Desktop & Stage, Displays advanced

## Status

Accepted (T227, 2026-11-08).

## Context

T222 mapped every Settings row to an owner; T224 built the one apply path;
T225/T226 shipped the Network and General groups. The remaining shell-owned
panes still had planned rows: Appearance was accent/scheme/transparency only,
Quick Settings/Search/Screen Saver had no bodies, Desktop & Dock omitted the
Desktop & Stage Manager group, and Displays stopped at scale/rotation.

The Displays pane is the highest fake-control risk: the compositor accepts
`set_night_light`/`set_vrr` but does not apply them, has no runtime mode switch
or multi-output arrangement UI, and colord is not integrated. Advertising
those as live toggles would violate the no-fake-controls rule.

## Decision

1. **Appearance completion is one live theme path.** New keys
   `appearance.highlight`, `appearance.sidebarIconSize`,
   `appearance.wallpaperTint`, and `appearance.scrollBars` are written by the
   pane and consumed by `Theme.highlightOverride`/the generated `selection`
   colour, the design-system `Sidebar` slots, `Glass.wallpaperTint`, and
   `ScrollView`. `shell/src/themebinding.*` and the Settings app's local
   bindings set the same values, so the pane and the shell restyle on one
   `Changed` signal. Motion is a deep link to the existing
   `accessibility.reduceMotion` toggle, not a second key.

2. **Quick Settings customization writes settingsd; the panel observes it.**
   `quickSettings.modules` is the ordered module id list and
   `quickSettings.menuBarOnly` removes an item from the panel while keeping its
   `menu.control.*` menu-bar mark. `ControlCenter.qml` filters its live tile
   model and each tile's visibility from those keys. The static Column keeps
   the built-in scene order; the ordered model is the contract the T235
   expanded-module layout consumes. Per-display panels are disabled with a
   T235 reason (one Control Center surface today).

3. **Search ships the durable T246 contract, not a fake index.** Categories,
   indexed locations, shortcut, web search, and `menu.control.search` are
   settingsd keys. Indexing status and Rebuild are disabled with the T246
   reason while the service is absent.

4. **Screen Saver reuses the first-party lock scene.** `screenSaver.style`,
   `screenSaver.clock`, `showDate`, and `showSeconds` are pushed to
   `LockScreen`; Start After is the existing `idle.blank` session-idle key.
   Animated savers are not advertised.

5. **Desktop & Stage Manager only advertises what exists.** `desktop.showItems`
   is consumed by the Files desktop process. Click-wallpaper, Stage Manager,
   recent apps, and widgets are disabled with T245b/T242 or an explicit OPEN
   reason.

6. **Displays advanced is a capability table.** `libs/settings-client/
   displaycapabilities.*` is the single list. Brightness and rotation are live;
   refresh, arrangement, night light, color profile, HDR, and VRR are disabled
   with the exact missing compositor/colord half. `tst_displayspolicy` asserts
   that only the two supported rows are advertised.

## Consequences

- A new Appearance preference must be added to the theme/glass/sidebar
  consumer, the Settings local binding, `settingsSchemaDefaults()`, and
  `docs/settings-keys.md`; the schema-doc test keeps the table honest.
- A future Quick Settings reorder that changes the live scene order can replace
  the static Column with the ordered model without changing the key contract.
- Promoting a Displays advanced row means implementing the compositor/colord
  half first and removing its reason from the capability table; the policy test
  fails on an advertised unsupported row.
- T235/T242/T245b/T246 own the deferred halves named in the pane reasons.
