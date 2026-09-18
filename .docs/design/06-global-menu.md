# Global Menu

## Summary

A global menu (macOS-style, in the menu bar) is not a fundamental Wayland
capability — an application has to export a meaningful menu model somehow. We
build a `menu-broker` service with a strict priority order and a hard
never-break-apps rule.

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
perfect menu      compatible menu    app name only
```

1. **Our native API.** Settings and Files publish a menu model directly,
   giving essentially perfect macOS-like behavior.
2. **DBusMenu / AppMenu.** Third-party applications that export DBusMenu /
   AppMenu-style menus get a compatible global menu. Linux has an existing
   ecosystem here (Vala Panel AppMenu, Ayatana indicators descend from the
   Ubuntu Unity era).
3. **No exporter.** The menu bar shows just the application name.

## Known limitation

Contemporary example: GTK applications frequently do not export the expected
DBusMenu data under Wayland (e.g., Xournal++ under KDE's global menu), so the
feature cannot work automatically for them. This is an application-cooperation
problem, not something we can force from the compositor side.

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
    File  Edit  View  Window  Help

Non-compatible apps:
    Application Name
```

This is far more robust than forcing Unity-era injection modules into every
toolkit.

## Menu-broker design notes

- The broker tracks the focused window via the compositor's private protocol
  and resolves which menu model (if any) to display.
- First-party apps publish declarative menu models through the design
  system's `MenuBarMenu` component (see
  [10-design-system.md](10-design-system.md)).
- DBusMenu bridges map third-party models into the same menu-broker
  representation.
