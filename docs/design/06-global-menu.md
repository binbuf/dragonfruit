# Global Menu

## Summary

A global menu (macOS-style, in the menu bar) is not a fundamental Wayland
capability — an application has to export a meaningful menu model somehow. We
build a `menu-broker` service with a strict priority order and a hard
never-break-apps rule.

## System menu and application menu

Two menus are always present, before any application-exported model:

- The **system menu** (the dragonfruit mark, far left) is shell/session-owned.
  It carries session operations: About This System, System Settings, App
  Store, Sleep, Restart, Shut Down, Lock Screen, Log Out. These are not an
  application's menu and never move.
- The **application menu** (the focused app's name, bold) carries the standard
  About / Settings / Hide / Hide Others / Show All / Quit items. On macOS the
  application itself owns these; here the shell synthesizes them from app
  metadata, and the **menu-broker owns the model** (T-14.2a) so enable/disable
  and per-app state are live. On the empty desktop the application menu is
  **Files**. Hide/Hide Others/Show All execute as tracked compositor
  minimize/restore operations (T230).

An application's exported menus (File, Edit, View, …) render *after* these
two. This is what makes an app that exports nothing still feel macOS-like: it
keeps the application menu and its name, rather than collapsing to a bare
name. The Tier-3 "no exporter" case therefore means "fixed system + application
menus, no app-exported menus."

## Menu broker priority

```text
                Active window
                     │
                     ▼
                menu-broker
                     │
   ┌─────────────────┼─────────────────┐
   │                 │                 │
our native API      DBusMenu         no exporter
   │                 │                 │
   ▼                 ▼                 ▼
perfect menu      compatible menu    fixed menus only
```

1. **Our native API.** Settings and Files publish a menu model directly,
   giving essentially perfect macOS-like behavior.
2. **DBusMenu / AppMenu.** Third-party applications that export DBusMenu /
   AppMenu-style menus get a compatible global menu. Linux has an existing
   ecosystem here (Vala Panel AppMenu, Ayatana indicators descend from the
   Ubuntu Unity era).
3. **No exporter.** The fixed system and application menus (including the
   application's name) render; no app-exported menus appear.

## Known limitation

Contemporary example: GTK applications frequently do not export the expected
DBusMenu data under Wayland (e.g., Xournal++ under KDE's global menu), so the
feature cannot work automatically for them. This is an application-cooperation
problem, not something we can force from the compositor side.

**Current status (2026-11).** All three tiers are wired. The **native** tier is
published by first-party apps (Settings is the reference) and resolved by the
broker. The **DBusMenu** tier is projected by app-index, stored by the broker,
and consumed by the shell: focus maps to the focused window's registration,
the registration is re-keyed to the canonical desktop id, a fresh `WindowMenu`
read picks up dynamic rows, and a `dbusmenu:<id>` click is routed back through
`WindowMenuEvent` as `Event(id, "clicked")`. The fixed application menu's
Hide/Hide Others/Show All rows compute live enabled state and execute as
tracked compositor minimize/restore operations. The tray (StatusNotifier) tier
is live and re-reads on a timer; live icon/attention signals are not subscribed.
Read "compatible Linux apps" in the table below as **shipped for apps that
register a DBusMenu**, not universal: a toolkit that exports nothing still gets
the fixed menus only.

## Interaction and shortcut dispatch

Menus follow macOS interaction conventions: click to open, drag through
submenus with delayed hover, Escape or focus loss dismisses, and an open menu
tracks the focused window switching underneath.

Shortcuts are **dispatched, not merely displayed**:

- First-party menu models carry actionable accelerators executed by the app
  itself.
- DBusMenu exports embed accelerator strings; the broker parses them and
  asks the compositor to register the keybindings while the owning window is
  focused.
- Conflicts resolve by focus: the active window's menu wins, and system
  shortcuts (workspace switching, Mission Control, app switcher) take
  precedence over application accelerators.

The visual grammar for every menu row is the design-system
[menu layout language](10-design-system.md#menu-layout-language): a reserved
check gutter, an eliding label, a right-aligned `ShortcutHint` glyph sequence,
and a reserved chevron gutter. The top bar's dropdowns and pointer context
menus share the same `MenuRow`, so a chord such as `Super+Shift+Z` reads the
same everywhere and never appears as raw `+`-separated text.

### Services, Recent Items, and the dynamic Window menu (T230)

- The fixed application menu always carries **Services**. Linux has no
  macOS-style service registry, so the standard submenu is present with an
  explicit disabled reason until an application registers a service; an app
  that publishes its own fixed menu may replace the row.
- The **system menu** carries **Recent Items**, fed by the shell's live
  recency list and capped by `menu.recentItems` (0–50). With no history the
  submenu holds one disabled row with its reason; a chosen row opens the app.
- An app that exports no **Window** menu gets one synthesized from the live
  compositor projection: Minimize, one checked row per open window, then Bring
  All to Front. An exported Window menu wins; the broker and the shell's
  fallback share the same rows and disabled reasons.
- Every row has a command or is disabled with a reason. The reason is exposed
  to accessibility and is available to the row renderer; the model never ships
  an inert row without one.

## Hard rules

- **Never remove a third-party application's internal menu merely because
  global menu mode is enabled.** If the application successfully exports a
  menu, show it globally; otherwise leave its existing menu alone.
- **The feature is a toggle.** When switched off, our first-party
  applications restore their local menu presentation immediately.

Settings switch behavior:

```text
Global application menu
[ On ]

Compatible apps:
    [mark]  Application Name   File  Edit  View  Window  Help

Non-compatible apps:
    [mark]  Application Name
```

This is far more robust than forcing Unity-era injection modules into every
toolkit.

## Menu-broker design notes

- The broker tracks the focused window via the compositor's private protocol
  and resolves which menu model (if any) to display.
- First-party apps publish declarative menu models through the design
  system's `MenuBarMenu` component (see
  [10-design-system.md](10-design-system.md)). The published form is the
  component's normalized, JSON-serializable entry shape, with an `action`
  string per row; the shell's `MenuBar` consumes it as
  `applicationMenuItems` + `appMenuModel` (ADR
  [0041](adr/0041-native-menu-model-publication-shape.md)).
- Settings publishes its model from `apps/settings/SettingsMenu.qml` (T-09.6a):
  the app's `App` / `File` / `Edit` / `View` / `Window` / `Help` menus. T230
  wires the app-side transport (`apps/settings/MenuPublisher.cpp`) to
  `Publish("org.dragonfruit.Settings.desktop", model)`; the shell resolves the
  focused raw `app_id` through app-index to that canonical desktop id.
- DBusMenu bridges map third-party models into the same menu-broker
  representation.

## Implementation note (T-14.2a)

The broker is real: `services/menu-broker` serves
`org.dragonfruit.MenuBroker1`. First-party apps publish their ADR-0041 model
with `Publish(appId, model)`; the shell pushes the focus and window state with
`SetFocusedApp`/`SetWindowStates` and reads `Resolve`/`ResolveFocused` (or
`Policy` for the fixed menu alone). The channel is D-Bus, chosen for the
restartable-service model (ADR [0095](adr/0095-menu-broker-resolution-and-fixed-menu.md)).

The fixed **application menu**'s Hide/Hide Others/Show All verbs carry live
state derived from the window list: Hide needs the focused app to have a
visible window, Hide Others needs another visible app, and Show All needs at
least one fully-hidden app. A publisher's own `applicationMenuItems` is used
verbatim except for those three rows. The shell mirrors this rule in
`shell/src/menubrokerpolicy` so the demo bar is live while the service is
absent; the broker remains the model's owner. Hide/Hide Others/Show All execute
through the compositor's per-window minimize/restore requests; the shell tracks
the ids it hid so Show All never restores a window the user minimized itself
(T230, ADR [0240](adr/0240-menu-bar-focus-dbusmenu-and-hide-verbs.md)).

## Implementation note (T-14.2b)

Accelerators are focus-scoped and dispatched. The broker parses each published
row's `shortcut`/`action` into a per-app table
(`services/menu-broker/src/accelerators.rs`), resolves only the focused app's
chords, and always lets a reserved system chord win. `Resolve` carries the
table; `Dispatch` resolves a chord, and `SetSystemAccelerators` declares the
reserved set. The shell mirrors the focused app's table to the compositor over
`df_toplevel_manager.set_app_accelerators` (protocol v7, one `action<TAB>chord`
per line); the compositor's existing shortcut engine matches it and returns the
existing `app_accelerator` event, which the shell routes through the same
action seam as a menu click. The broker never installs a grab.

The **toggle** is `menu.global` (settingsd schema v6, default on). When off the
shell sets `MenuBar.globalMenuEnabled` false: the focused app's exported menus
are suppressed and first-party apps keep their local menus, while the fixed
system and application menus stay. Contract frozen in ADR
[0096](adr/0096-focus-scoped-accelerators-and-global-menu-toggle.md).

## Implementation note (T-14.3)

StatusNotifierItem/AppIndicator tray items are live. `dragonfruit-app-index`
serves `org.kde.StatusNotifierWatcher`, normalizes the registrations into a tray
registry (`services/app-index/src/tray.rs`), and exposes the items and their
DBusMenus to the shell over `org.dragonfruit.AppIndex1`
(`TrayItems`/`TrayMenu`/`TrayMenuEvent`/`TrayActivate`). Each item is resolved
fresh from its owning process; an owner that disconnects is pruned via
`NameOwnerChanged`. The DBusMenu `GetLayout` tree is projected to the
design-system menu row shape, so the shell renders a tray item's menu with the
same `ContextMenu` component as every other menu, and a click is forwarded as
the DBusMenu `Event(id, "clicked")`. The shell joins third-party items to the
one status row (`tray:<name>` plus a file-backed `iconSource`), never a separate
tray strip. Live `NewIcon`/attention signals are not subscribed yet (the shell
re-reads on a short timer); icon animation polish is deferred. Contract frozen
in ADR [0097](adr/0097-statusnotifier-tray-host-and-dbusmenu-projection.md).

## Implementation note (T-14.4)

The DBusMenu/AppMenu **projection** is live in app-index/menu-broker, and T230
completed the shell half (focus → registration → model, click routing).
`dragonfruit-app-index` also serves
`com.canonical.AppMenu.Registrar` at `/com/canonical/AppMenu/Registrar` (name
best-effort, like the tray watcher): `RegisterWindow(windowId, path)` maps a
window to its DBusMenu object, and the additive
`RegisterWindowForApp(windowId, appId, path)` supplies a desktop id for the
broker key. The pure bridge (`services/app-index/src/menubridge.rs`) reuses the
tray parser (`tray::parse_layout`) and projects the `GetLayout` root's children
into the menu-broker's `menus` shape (`[{title, items}]`), preserving each
row's `enabled`/`checked` and tagging clickable rows with
`action: "dbusmenu:<id>"`. On registration app-index pushes the projection into
the broker with the new additive `PublishDbusMenu(appId, model)`, which stores
it at `Tier::DbusMenu`; `Withdraw` clears it again, and a disconnected owner is
pruned via `NameOwnerChanged`. `org.dragonfruit.AppIndex1` additionally exposes
`AppMenuWindows`/`WindowMenu`/`WindowMenuEvent` for local queries and clicks.
T230 wires the shell to that surface: `FocusWindow(windowId, appId)` re-keys a
standard registration to the canonical desktop id and re-publishes it, the bar
reads a fresh `WindowMenu` at open time, and a click calls `WindowMenuEvent`.
Contract frozen in ADR [0098](adr/0098-dbusmenu-bridge-in-app-index.md); the
completion decisions are in ADR
[0240](adr/0240-menu-bar-focus-dbusmenu-and-hide-verbs.md).

## Implementation note (T-14.7)

The shell now consumes the broker over D-Bus, retiring the interim
`demoAppMenu()` stand-in. `shell/src/menubrokerclient.{h,cpp}` pushes the
focused app id (`SetFocusedApp`) and the running-window projection
(`SetWindowStates`, built by `menubrokerpolicy::windowStatesJson`) and reads
`ResolveFocused`; the reply supplies the fixed application menu (with live hide
verbs) and the focused app's exported top-level menus, which the bar renders as
`applicationMenuItems` + `appMenuModel`. When the service is absent the shell
falls back to its own `fixedApplicationMenu` and no exported menus. T230 closed
the T-14.4 follow-up: a bridged `dbusmenu:<id>` row routes back through
app-index's `WindowMenuEvent`, and focus resolves the canonical desktop id
before the broker read.

The Dock's interim local `.desktop` scan/parse was deleted the same session;
`org.dragonfruit.AppIndex1` is now the shell's only identity source (the
`DesktopEntryIndex` cache is populated solely from `Enumerate`). The dev/demo
harness starts `dragonfruit-app-index` and `dragonfruit-menu-broker` so the
nested session exercises the real services.
