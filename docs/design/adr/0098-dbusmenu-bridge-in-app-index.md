# 0098 — The DBusMenu global-menu bridge lives in app-index and publishes the `dbusmenu` tier

## Status

accepted — implements the DBusMenu half of
[06-global-menu.md](../06-global-menu.md) and the compatibility-bridges item
in [14-global-menu-app-index-compat.md](../tracks/14-global-menu-app-index-compat.md)

## Context

The menu-broker resolves the focused application's global menu with the
priority order native → DBusMenu → none, but until now only native
publications populated it: `Tier::DbusMenu` was defined and empty. A
third-party app exports a global menu over `com.canonical.dbusmenu` and maps a
window to the menu object with `com.canonical.AppMenu.Registrar`. T-14.3
already gave `dragonfruit-app-index` the tray host and the one DBusMenu parser
(`tray::parse_layout`) that projects a `GetLayout` tree into the design-system
row shape; T-14.4 must reuse that parser rather than add a second.

## Decision

- **app-index hosts the AppMenu.Registrar.** `dragonfruit-app-index` serves
  `com.canonical.AppMenu.Registrar` at `/com/canonical/AppMenu/Registrar` and
  takes the well-known name best-effort (exactly like the SNI watcher): on a
  session where another shell owns the name, the registrar object still
  answers at that path. The pure table (`services/app-index/src/menubridge.rs`)
  maps window id → `{app_id, destination, menu path, owner}`. The standard
  `RegisterWindow(windowId, path)` keys the model by the caller's unique bus
  name; the additive `RegisterWindowForApp(windowId, appId, path)` lets an app
  that knows its desktop id supply it. Owner disconnect
  (`NameOwnerChanged`) prunes the registration and withdraws the model.
- **One parser, one new top layer.** The bridge reuses `tray::parse_layout`;
  `menubridge::menus_from_layout` only turns the DBusMenu root's children into
  the menu-broker's `menus` shape (`[{title, items}]`) and tags each clickable
  row with `action: "dbusmenu:<itemId>"` so the broker's accelerator extraction
  and click routing have an action string. `enabled`/`checked`/`shortcut`/
  `id` pass through `MenuNode::to_json` unchanged.
- **Push, don't pull.** On a successful registration app-index projects the
  window's menu and calls the new, additive
  `org.dragonfruit.MenuBroker1.PublishDbusMenu(appId, model)`, which stores the
  payload with `Tier::DbusMenu` (`Broker::publish_dbusmenu`). `Withdraw` clears
  the tier again. The push is best-effort: an absent broker is not an error,
  and the bridge still answers local `WindowMenu`/`WindowMenuEvent` queries on
  `org.dragonfruit.AppIndex1`.

## Consequences

- The `dbusmenu` tier is populated and `Resolve`/`ResolveFocused` return it;
  the fixed application menu is still synthesized by the broker and the app's
  exported menus follow it.
- The shell does not consume the bridge yet: it still computes its menu
  locally, and mapping the focused app (or window id) to a registration and
  routing a `dbusmenu:<id>` action back through `WindowMenuEvent` is the
  remaining wiring (T-14.6/T-14.7). Live re-projection on `about-to-show`/state
  change is also deferred; a registration projects once.
- Because the registrar, watcher, and DBusMenu parser all live in app-index,
  there is still exactly one compatibility host and one row shape.