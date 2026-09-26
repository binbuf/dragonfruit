# 0097 — The StatusNotifier tray host lives in app-index; DBusMenu projects to the design-system shape

## Status

accepted — implements the tray half of
[06-global-menu.md](../06-global-menu.md) and the compatibility-bridges item
in [14-global-menu-app-index-compat.md](../tracks/14-global-menu-app-index-compat.md)

## Context

Third-party Linux tray items are StatusNotifierItems (SNI) registered with an
`org.kde.StatusNotifierWatcher`. The compatibility phase requires them to
render in the menu bar with working menus, styled identically to first-party
status items, and T-14.4 will bridge DBusMenu into the global menu. Nothing in
the tree spoke SNI, and the `dragonfruit-app-index` service is the always-on
session component that already owns application identity and themed icons.

## Decision

- **app-index hosts the watcher.** `dragonfruit-app-index` also serves
  `org.kde.StatusNotifierWatcher` at `/StatusNotifierWatcher` and takes the
  well-known name when it is free; on a session where another desktop shell
  already owns it, the name is left alone and the watcher object still answers
  at that path, so an item can register directly against app-index. The pure
  registry (`services/app-index/src/tray.rs`) normalizes a registration
  (bus name → `/StatusNotifierItem`; object path → `sender:/path`) and drops an
  item when its owner disconnects (`NameOwnerChanged`).
- **One flat JSON surface on `org.dragonfruit.AppIndex1`.** The shell calls
  `TrayItems` (each item's SNI properties resolved fresh, with `iconPath`
  resolved through the app-index icon theme), `TrayMenu(name)`,
  `TrayMenuEvent(name, id)`, and `TrayActivate(name, kind)`. `TrayMenu` runs
  `AboutToShow` then `GetLayout` and projects the `com.canonical.dbusmenu` tree
  into the same `{id,type,label,enabled,checked,checkable,submenu}` rows the
  design-system menus use; `Event(id, "clicked")` is forwarded verbatim. An
  unresponsive item is pruned, never an error.
- **The shell renders tray items in the existing status slots.** A
  `TrayClient` (shell/dockcore) decodes the views; the controller appends each
  item to `statusItems` as `tray:<registered-name>` with `iconSource` (the
  resolved file URL) and re-reads on a short timer. Clicking one opens a
  design-system `ContextMenu` anchored to its slot, committed through the same
  overlay-popup geometry the app menus use; a chosen row is sent back with
  `TrayMenuEvent`. Third-party items therefore get the identical sizing,
  hover, and dark/light treatment (no separate tray strip).

## Consequences

- T-14.4's DBusMenu bridge consumes the same projection (`tray::MenuNode` and
  `TrayMenu`) for the global application menu; there is one parser and one row
  shape.
- Live `NewIcon`/`NewToolTip`/`NewStatus` signals are not subscribed yet: the
  shell re-reads on a 2 s timer, so attention/icon animation is deferred (the
  task's stated deferral). Crash cleanup is real (`NameOwnerChanged`).
- The host is app-index, not the shell, so a restartable app-index gets the
  tray back with no shell restart; the shell degrades to no tray items when the
  service is absent.