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

Visual and interaction reference for these surfaces (menu bar, system menu,
Dock, Control Center quick-settings sheets, Spotlight, About This System):
[macos-ui-inventory.md](../reference/macos-ui-inventory.md) — distilled from
local-only macOS screenshots that never ship.

## Menu bar

The top menu bar hosts, from left to right:

- The **system menu** — the dragonfruit mark, always present. Its items are
  session/system operations: About This System, System Settings, App Store
  (no equivalent yet), Sleep, Restart, Shut Down, Lock Screen, and Log Out.
- The **application menu** — the focused application's name, always present
  and bold, carrying the standard About / Settings / Hide / Hide Others /
  Show All / Quit items. On the empty desktop this is **Files** (the Finder
  model: the file manager owns the desktop — see [09-files.md](09-files.md)).
- The active application's own menus (File, Edit, View, …) via the
  menu-broker (see [06-global-menu.md](06-global-menu.md)).
- System status items: Wi-Fi, Bluetooth, volume, battery, clock, Focus/DND,
  accessibility
- The Control Center entry point
- A Mission Control button/gesture target

The system menu and application menu are **shell-owned chrome**, not part of
the application's exported model: an app that exports nothing still gets a
complete menu bar, and an app that exports menus gets them *after* the fixed
two. The menu-broker is expected to take over synthesizing the application
menu from app metadata (T-22) so the items can reflect per-app state, while
the system menu stays with the session.

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

The compositor-side state machine (T-06.1) lives in `app_switcher.rs`: Cmd+Tab
opens/closes, Tab and the arrows cycle apps, Cmd+` (Cmd+Shift+`) cycles windows
within the selected app, Command release commits the selected window through
the existing activation path (cross-Space, restore-if-minimized, and the same
`activate_app` resolver the Dock uses), and Escape cancels with no focus
change. It broadcasts the `df_toplevel_manager.app_switcher` event plus one
`app_switcher_entry` event per app in recency order. The shell consumes that
projection to draw the centered `app-switcher` `overlay` chrome surface
(`shell/switcher/AppSwitcher.qml`, T-06.2a): a scrim, one card per app in
recency order with the selection highlighted and an accessible `app_id`/name
fallback, and the reduced-motion variant. The compositor renders the **live**
window surfaces through the T-04 scene transform underneath — never thumbnails
— and follows the window cursor, so Cmd+` swaps the preview. The shell does not
re-derive recency, cycle, or commit; pointer presses on the live previews are
hit-tested by the compositor and commit or cancel the overlay (T-06.2b).

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
background (see [09-files.md](09-files.md)). Desktop Reveal shares the one
overview pipeline and scene transform: the live surfaces slide out of their
nearest screen edge (reduced motion fades them in place), and `query reveal`
exposes the progress headlessly (see [02-compositor.md](02-compositor.md)).

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

**T-11.1a status.** `services/notifications` (`dragonfruit-notifications`)
serves the standard `org.freedesktop.Notifications` to apps and a
shell-facing `org.dragonfruit.Notifications1` (banners/history JSON,
dismiss/expire, `Changed`) at the same object path; the service owns the
queue, a bounded history, and banner expiry. The shell (`NotificationClient` +
`NotificationModel`) renders the newest active banner into a top-right
`notification` overlay surface and keeps the history for the notification
center. See [adr/0056](adr/0056-notification-service-surface-and-shell-banner.md).

**T-11.1b status.** Actions round-trip: the service advertises the `actions`
capability and adds `Invoke(id, action_key)` to the shell interface, which
emits the freedesktop `ActionInvoked` to the originating app and dismisses the
banner. The banner surface takes pointer input (a card-sized input region) and
the card renders an inline action row; a body click fires the app's `default`
action or dismisses. The Dock's transient launch-failure badge is replaced by
a real `Notify` raised through the same client
(`shell/src/launchfailure.{h,cpp}`, `ShellController::failDockLaunch`). See
[adr/0057](adr/0057-notification-actions-and-dock-failure-notice.md).

**T-11.2a status.** The service owns the Focus/DND policy:
`services/notifications/src/policy.rs` defines `off` / `focus` / `dnd` and
the per-app allow list, and the queue asks it whether a `Notify` banners.
`focus` admits allow-listed apps and `critical` urgency; `dnd` admits only
allow-listed apps; everything suppressed is still recorded in the history
with `suppressed: true` and counted in the policy's batch (cleared on return
to `off`). The shell-facing interface adds `FocusPolicy()` (JSON `mode`,
`allowList`, `batchedCount`), `SetFocusMode`, and `SetFocusAllowList`, and
keeps the T-11.1a `DoNotDisturb`/`SetDoNotDisturb` pair as a compat mapping.
The menu-bar reflection and Control Center tiles are T-11.2b/T-11.3b. See
[adr/0058](adr/0058-focus-dnd-policy-semantics.md).

**T-11.2b status.** The menu bar reflects the policy. The shell reads
`FocusPolicy()` through `NotificationClient` (re-read on `Changed`, coded in
`shell/src/notificationclient.{h,cpp}`) and decodes it in `NotificationModel`;
`shell/src/focusstatus.{h,cpp}` maps it to the `focus` status item. The item
hides in `off`, shows the crescent in `focus`, and shows it *selected*
(accent) in `dnd`; the suppressed batch count is the label when non-zero and
always part of the accessible name. The existing Dock launch-failure
notification (T-11.1b) remains the failure path. See
[adr/0059](adr/0059-menu-bar-focus-reflection.md).

**T-11.3a status.** The Control Center panel exists: the menu-bar item (or the
Control-Option-C shortcut, routed as the compositor's `control-center` input
action) opens a top-right `control-center` overlay surface rendered from
`shell/control-center/ControlCenter.qml`. The panel ships three tiles —
Wi-Fi (state + network, `Wi-Fi Settings…` link), Sound (volume slider + mute,
through the T-07 bridge host), and Display (the brightness slider, written to
settingsd `display.brightness` and forwarded to the compositor as
`df_output.set_brightness`). Escape and click-away dismiss it. The Wi-Fi radio
is read-only until T-15 adds the adapter write; Focus/DND, dark mode, and the
a11y pass are T-11.3b. See [adr/0060](adr/0060-control-center-panel-and-brightness.md).

**T-11.3b status.** The panel now ships five tiles: Focus/DND and Dark Mode
join Wi-Fi, Sound, and Display. The Focus switch is Do Not Disturb: on writes
the notification service's `SetFocusMode("dnd")`, off writes `off`; the mode
and suppression count come back through `FocusPolicy()` so the tile and the
menu-bar crescent share one source of truth. The Dark Mode switch writes
settingsd's `appearance.colorScheme` (`dark`/`light`, never `auto`) through the
same client the shell's `ThemeBinding` owns, so the whole design-system Theme
flips live. The `shell/src/controlcenterpolicy.{h,cpp}` mapping is pure and
unit-tested. Accessible roles are on every tile (grouping + switch/slider
names, operable text links), and the panel keeps the `ON_DEMAND` + deferred
dismissal rules of ADR 0060. The Focus and Appearance `… Settings…` links log
only until T-16. See [adr/0061](adr/0061-control-center-toggles-and-a11y.md).

**T-11.4a status.** The OSD exists: a volume or brightness change (the Control
Center sliders, the menu-bar volume menu, or mute) presents a brief centered
card on the active output — the design-system glyph, a level track, and the
percentage — then fades and dismisses. The shell renders it into a centered,
unanchored `overlay` surface (`namespace "osd"`), and the pure
`shell/src/osdmodel.{h,cpp}` owns the visibility window, the coalescing of
concurrent triggers, the fade, and the fullscreen suppression; the shell drives
it with a 16 ms timer while visible only, so an idle desktop still wakes no one.
Reduced motion collapses the fade to an immediate 1.0. A fullscreen surface that
owns the active Space suppresses the OSD entirely. The card consumes the new
`component.osd` / `motion.osd` tokens. `DF_OSD_FIXTURE=volume|brightness` is the
capture-only presentation seam; the AT-SPI/keyboard pass and the committed
captures are T-11.4b. See [adr/0062](adr/0062-osd-overlay.md).

## Relationship to compositor and services

- Compositor state (windows, workspaces, outputs) arrives via private
  protocols; the shell never duplicates compositor state machines.
- System state (network, audio, power) arrives via the adapters in
  [07-system-integration.md](07-system-integration.md); the shell never talks
  to hardware directly.
- The shell is crashable and restartable without taking down the compositor.
