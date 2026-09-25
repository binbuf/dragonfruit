# 0041 — First-party menu models publish in the design-system entry shape; the broker is the transport

## Status

accepted

## Context

The global menu ([06-global-menu.md](../06-global-menu.md)) has a three-tier
priority: our native API, DBusMenu, then the fixed menus. The native tier is
"Settings and Files publish a menu model directly." T-14.2a's menu-broker owns
the focus→model resolution and the transport; T-09.6a is the Settings app's
publication step and must produce the artifact that broker consumes. T-09.6a
explicitly defers the broker, so the shell keeps the fixed application menu in
the meantime.

The design system already defines the normalized, JSON-serializable entry shape
`MenuBarMenu.menuModel` renders and re-emits (`{id, type, label, shortcut,
enabled, checked, checkable, keepOpen, hasSubmenu}`). Two shapes would drift;
one must be the contract.

## Decision

- **The design-system entry shape is the native publication contract.** A
  first-party app declares its menus once in that shape; the same declaration
  feeds its local presentation and the global bar. `apps/settings/SettingsMenu.qml`
  (a QML singleton) is Settings' single source: `applicationMenuItems` (the
  fixed application menu) and `menus` (the app's own top-level menus), wrapped
  in a JSON-serializable `publishedModel` (`{appName, applicationMenuItems,
  menus}`). No QML objects leak into the published form.
- **Rows carry an `action` string**, not a callback. `activate(action, item)`
  is the app-side dispatch seam; the broker routes the same action back to the
  app (T-14.2b). `actionFor(menuIndex, itemIndex)` follows the `MenuBarMenu`
  `triggered(index, item)` contract.
- **The shell's `MenuBar` is the consumption surface.** Its `applicationMenuItems`
  + `appMenuModel` properties accept exactly the published shape; the shell
  renders the model at indices 2.. after the fixed system and application
  menus. The transport (focus→app→model) belongs to the menu-broker (T-14.2a).
- **Until T-14.2a the shell keeps the fixed application menu** for Settings; the
  published model may be fixed (no live enable/disable state) — the T-09.6a
  scope note "fixed app menu is enough until then."

## Consequences

- T-14.2a has one model shape to consume and one dispatch verb (`action`) to
  route; no translation layer between an app and the bar.
- Files (T-10) and later first-party apps publish the same way; the model lives
  with the app, never duplicated in the shell.
- The cross-process channel (D-Bus or Wayland-adjacent) is still open (legacy
  T-22 risk note); this ADR fixes only the payload and the consumption surface,
  so the broker can choose the channel without touching the apps' declarations.
- A first-party app's in-window menu presentation (the toggle-off path of
  [06-global-menu.md](../06-global-menu.md)) builds from the same singleton, so
  global and local menus cannot diverge.