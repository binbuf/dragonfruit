# Shell

## Summary

The shell is a Qt Quick process (or set of processes) rendering the always-
visible desktop chrome: menu bar, Dock, Control Center, notification center,
and OSD. It talks to the compositor over private, versioned Wayland protocols
and to system services over D-Bus.

Components:

```text
shell/
├── menubar/           # top menu bar: app menus, status items, clock
├── dock/
├── control-center/
├── notifications/
└── screenshot/       # capture UI over the portal capture path
```

## Shell surfaces

Menu bar, Dock, Control Center, notification banners, and OSD are Wayland
surfaces created by the shell process through the private shell protocol
(layer-shell-style reserved zones and stacking layers — see
[02-compositor.md](02-compositor.md)). The chrome renders independently of
client content: during a workspace switch, client surfaces shrink or slide
while the chrome follows its own animation curves, which is what makes the
transitions read as one system.

## Menu bar

The top menu bar hosts, from left to right:

- The active application's menu (via the menu-broker — see
  [06-global-menu.md](06-global-menu.md))
- System status items: Wi-Fi, Bluetooth, volume, battery, clock, Focus/DND,
  accessibility
- The Control Center entry point
- A Mission Control button/gesture target

Status items consume the system-service adapters described in
[07-system-integration.md](07-system-integration.md).

## Dock

The Dock obtains window/app state **directly from the compositor** rather than
attempting to infer everything through public protocols, using the app-index
service to resolve `app_id` / `WM_CLASS` to `.desktop` applications (see
[02-compositor.md](02-compositor.md)).

macOS-like click semantics:

```text
Pinned application
        │
        ├── not running → launch
        │
        └── running
              ├── one window → activate
              └── multiple windows → window chooser

Running but not pinned
        │
        └── temporary Dock entry

Minimized window
        │
        └── optionally appear on right side
```

Also included: magnification, auto-hide, bounce feedback (launch attention),
running indicators, drag rearrangement, and contextual menus. The Dock's
right end hosts **Trash**: a Files-backed location whose badge watches the
same GVfs `trash://` mount Files does — one source of truth that also
catches deletions made by other applications (see
[09-files.md](09-files.md)).

The difficult part is not drawing the Dock; it is getting all the lifecycle
details and edge cases polished — launch failures, app exit while animating,
windows opening on other workspaces, and apps with inconsistent identifiers.

## App switcher

The Cmd-Tab-style app switcher is **compositor-driven and shell-rendered**:
the compositor owns the global keybind, the window list, and workspace
awareness; the shell draws the overlay. Switching is **app-level, not
window-level** — the macOS mental model:

```text
hold switch key   → overlay lists running apps by recency
cycle            → apps; modifier cycles windows within the selected app
release          → focus the selection, animated out of the overlay
```

Per-window selection is covered by the Dock window chooser and Mission
Control (see [03-workspaces.md](03-workspaces.md)); the switcher stays
app-first.

## Hot corners

Configurable screen-corner triggers (Mission Control, notification center,
desktop reveal, lock screen) are detected in the compositor's input path and
dispatched to the shell, so they behave identically whether triggered by
pointer, gesture, or keyboard.

## Desktop background

The desktop background is **compositor-drawn**: each Space's wallpaper is
part of the workspace scene and slides with it during switches (see
[03-workspaces.md](03-workspaces.md)). The shell draws chrome only.

Desktop icons are a later work item and belong to **Files** — the macOS
model, where the file manager owns the desktop — rendered through a
compositor desktop-layer surface. The MVP ships plain wallpaper. The Desktop
Reveal hot corner works regardless: it moves windows aside to expose the
background (see [09-files.md](09-files.md)).

## Screenshot and screen recording

The capture UI is a shell surface; the capture itself goes through the portal
capture path (single-frame and PipeWire streams — see
[07-system-integration.md](07-system-integration.md)), and the keybind is a
compositor global shortcut. Screenshot and OSD feedback follow the same
design-system motion rules as the rest of the chrome.

## Authentication agent

The shell hosts the polkit authentication agent, so privileged operations
requested by Settings, Control Center, and our services surface one
consistent, design-system prompt rather than a toolkit default (see
[07-system-integration.md](07-system-integration.md)).

## Third-party status items

The menu bar's system area additionally hosts StatusNotifierItem / AppIndicator
exports — the de-facto Linux tray standard — through a bridge, shipped in the
compatibility phase (see [ROADMAP.md](../ROADMAP.md)). Third-party items
get the same sizing, hover, and dark/light treatment as first-party items.

## Control Center

The user sees one highly curated panel. Internally it is sensibly reusing
Linux components:

```text
Control Center
│
├── Wi-Fi ───────────── NetworkManager
├── Bluetooth ───────── BlueZ
├── Sound ───────────── PipeWire / WirePlumber
├── Battery ─────────── UPower
├── Displays ────────── our compositor
├── Brightness ──────── compositor / kernel interfaces
├── Focus / DND ─────── our notification service
├── Keyboard ────────── compositor / xkbcommon
└── Accessibility ───── shell + toolkit services
```

## Notifications and OSD

A notification service and an on-screen-display service (volume/brightness
changes, caps lock, battery warnings) run as independent components, following
the modern-desktop pattern of separate notifications, OSD, and idle services.
Focus/DND state lives with the notification service so Control Center and the
menu bar share one source of truth.

## Relationship to compositor and services

- Compositor state (windows, workspaces, outputs) arrives via private
  protocols; the shell never duplicates compositor state machines.
- System state (network, audio, power) arrives via the adapters in
  [07-system-integration.md](07-system-integration.md); the shell never talks
  to hardware directly.
- The shell is crashable and restartable without taking down the compositor.
