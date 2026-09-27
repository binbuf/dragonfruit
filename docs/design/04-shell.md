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

**System-menu action wiring (T-09 follow-up).** System Settings opens the
first-party Settings app through the same `.desktop` launch path as the Dock
(the installed `org.dragonfruit.Settings.desktop`); if Settings is already
running its window is focused instead of a second launch. Lock Screen routes to
the same `ext-session-lock-v1` path as the Cmd+Ctrl+Q shortcut. About This
System awaits the General > About pane (T-15.10); Sleep / Restart / Shut Down /
Log Out await the real-session logind work (T-12.6/T-16.4); App Store ships
disabled (no equivalent). Launched apps inherit the session environment with
the shell's own `QT_QPA_PLATFORM=offscreen` replaced by the session's Wayland
platform, so a Qt client maps on the real session platform.

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

### Dock activation and launch

A click is a request to the one activation authority, never a re-derivation of
"which window": the shell asks the compositor to activate the app's most recent
window (`df_toplevel_manager.activate_app`) and the compositor replies with
`activation_result(found)` (T-14.7g, protocol v8). When no window was found the
shell falls back to launching the installed `.desktop` entry (the app exited
between the projection and the click) or raises a notice — a click never dies
silently. A pinned identity the app-index cannot resolve produces a notice and
keeps its not-found mark. A near-stationary click always activates: the entry's
tap and drag handlers share one 8 px slop (`dockEntry.dragSlop`), so only a real
drag lifts into a rearrangement.

Every launch path (Dock, menu-bar `openApp`, Files reveal) goes through one
helper (`appLaunchEnvironment`) that replaces the shell's forced
`QT_QPA_PLATFORM=offscreen` with `wayland` and exports the socket the shell
connected on as `WAYLAND_DISPLAY`; a shell started with `--socket-name` therefore
still launches a client that maps a window. See
[ADR 0101](adr/0101-dock-activation-result-and-launch-display.md).

The difficult part is not drawing the Dock; it is getting all the lifecycle
details and edge cases polished — launch failures, app exit while animating,
windows opening on other workspaces, and apps with inconsistent identifiers.

### Dock plate and materials

The Dock is a **floating plate**: `controls.dock.edgeMargin` keeps it off its
anchored screen edge, `controls.dock.padding` is the artwork's cross-axis
(artwork ↔ plate edge) inset, and `controls.dock.paddingAlong` is the along-axis
inset at the plate's two ends. One live `plateRect` in `Dock.qml` derives the
plate geometry, the input region, and the live panel rect; it is the union of
the entry rects about the *resting* plate's fixed anchored edge (so the geometry
never feeds back into the entry layout) and `restingPlateRect` fixes that edge.
The shell reads
`surfaceThickness`/`reservedThickness` from it rather than re-deriving the math. The visual language follows the macOS Tahoe Dock
reference (local captures under `docs/reference/macos/`, never shipped): a
rimmed translucent glass plate, rounded-square icon tiles at a consistent
inset, and coherent hover/press/running states — reproduced with our own
geometry and tokens, never Apple assets (ADR
[0091](adr/0091-dock-tahoe-floating-glass-language.md)). The reserved zone is
the resting plate plus that margin; auto-hide reserves nothing. The plate is
**cosmetic**: under magnification it grows to wrap the magnified row in both
axes, inside the pre-reserved magnify band, while the reserved zone stays at the
resting thickness so windows never re-lay-out when the pointer sweeps the Dock.
The pointer is smoothed with `motion.dock-magnify` (the slight overshoot comes
from the token curve); reduced motion tracks the pointer directly. The
compositor's frosted backdrop follows the live plate rect that the shell
declares on each commit, so the material always sits under the artwork. A
hovered entry reveals its name (and state) in a `Tooltip` capsule above the
icon (T-14.7i). See
[ADR 0089](adr/0089-dock-plate-geometry-and-live-panel-rect.md) and the
T-14.7a/T-14.7b/T-14.7j units.

The plate's material is layered (T-14.7j, ADR
[0102](adr/0102-dock-material-role-and-qml-glass-layers.md)): the compositor
frosts the declared panel with the Dock's own `material.dockBlur`/`dockOpacity`
tokens — selected by the `dock` namespace, so it is independent of the menu
bar's `chrome*` frost — and `Dock.qml` draws a translucent `dockFill`, a bright
inner top-edge rim, a hairline `dockBorder`, and a soft shadow above it. Every
value is a `controls.dock.plate` token. The plate group is clipped at its top
edge so the shadow never bleeds into the transparent magnify band above it
(T-10 section 2). Icon tiles are rounded squares at
`controls.dock.icon.radiusRatio`, themed artwork is inset by
`controls.dock.icon.inset`, and the hover wash, lift shadow, focus ring,
running dot, and divider follow `controls.dock.hover`/`indicator`/`divider`.
At the `Minimal` degrade tier the compositor draws no frost and the plate reads
as a clean capsule with no highlight claim.

### Dock hover name label

Pausing over an entry for `controls.tooltip.dwell` (600 ms) shows the
design-system `Tooltip` — a passive, pointer-anchored capsule with one elided
line — above the icon on a bottom Dock and on the interior side on a vertical
one (ADR [0093](adr/0093-tooltip-joins-the-design-system.md), T-14.7i). The
label text is the entry `name` plus its state ("3 windows", "Trash — empty",
"Downloads — 2 items"), computed once in `DockEntry.tooltipLabel` so no QML
consumer re-derives it and no text returns to the artwork (ADR 0092). The Dock
owns the dwell timer and the suppression rules: a click, a drag, an external
drag, a resize, or any open context menu/chooser/stack hides it, and it is
never up at the same time as a popover. `anchorItem` is the live entry
delegate, so the capsule follows a magnified icon frame by frame and is clamped
to the Dock's ends; it rides the same `popoverRect` overlay/headroom path as a
menu so a bottom Dock's label is committed and never clipped. It is
presentational only — no focus, no key capture; the entry's accessible name
already carries the state.

### Dock motion and frame discipline

Continuous Dock motion is deliberately separate from the entry model. Launch
and attention bounce phases are pushed as an `entry id -> { phase, attention }`
map (`bouncePhases`) every committed frame; the Repeater model is never
reassigned for a bounce, so delegates are not recreated and hover/press state
survives. The 16 ms tick only advances the map and stops the moment both
clocks are empty, so the settled Dock contributes zero wakeups. Opening a
menu/chooser/stack never resizes the offscreen window: the shell pre-sizes the
buffer to a fixed `popoverHeadroom`/`popoverGutter` budget and `Dock.qml`
clamps every popover into it. `dock.size`/overflow changes and the drag-gap
close spring with `motion.dock-magnify` (reduced motion is duration 0); the
first configure snaps and magnification stays progress-based. The T-14.7c
trace lives at `docs/captures/t14-dock-motion-trace.txt`. See
[ADR 0100](adr/0100-dock-motion-phase-map-and-popover-buffer.md) and the
T-14.7c unit.

### Trash entry artwork

The Trash is a designed, original object (T-14.7d), not a wireframe, and it
matches the app tiles' quality without copying any Apple artwork. Its glyph is
drawn in a `controls.dock.trashSize`-derived box centred inside the entry's
iconSize box, so it scales with magnification and keeps the app tiles' baseline.
The anatomy is a metallic body with a subtle vertical fill/gradient and a rim
edge, an overhanging lid, and a clean handle. Empty is a tidy neutral bin; full
adds a crumpled-paper silhouette overflowing behind the rim plus an accent rim
cue — a shape change as well as a colour change, so it stays legible in
grayscale and for colour-vision differences. Unavailable keeps the dim plus the
status badge. The fill/rim/paper colours are semantic tokens (`trashFillTop`,
`trashFillBottom`, `trashRim`, `trashHighlight`, `trashPaper`, `trashPaperEdge`)
resolved per scheme from primitives; the glyph is pure QML rectangles and a
`Shape`, so it renders Image-less at any DPI and headless.

### Adding and removing apps

The Dock manages its own contents: the divider menu opens an **Add
Application** picker (an overlay popover anchored to the divider) fed by
app-index, listing installed applications with their themed icon and name, a
design-system `SearchField`, and a pin/unpin row state. The list is built by
the pure `buildAppPickerList` helper (`shell/src/apppicker.{h,cpp}`): it drops
`noDisplay` and non-launchable records, dedupes by desktop id, sorts by
localized name with an id tiebreak, and tags pin membership; the popover
filters that list live over name and id. app-index absence renders an explicit
"Application index unavailable" row, never a blank panel, and the shell
subscribes to app-index's coalesced `Changed` signal (T-14.1c) so an
install/uninstall while the picker is open stays fresh. Selecting a row toggles
membership through `dock.pinned` — the only persisted state, settingsd the
only writer — so the Dock and the picker cannot disagree. External drags show
the dragged app's real identity in the open gap and pin it on drop; files
dropped on an entry open with that app, on the Trash move to trash, and on the
Downloads stack move into it. The picker is not a launcher — Launchpad and the
Spotlight-equivalent search remain post-gate. See
[ADR 0090](adr/0090-dock-app-management-picker-and-drops.md) and the
T-14.7e/T-14.7f units.

**Drop identity and feedback (T-14.7f).** An external drag is read once, at
drag *enter* (`ShellProtocol::onDataDeviceEnter` → `beginDndRead`): the shell
parses the `text/uri-list`/`application/x-dragonfruit-app` payload, caches the
classification, and reuses it at drop instead of requesting the mime twice. If
the source has not finished writing by the drop, the read is completed
synchronously (bounded `poll`) before resolving, so a payload is never dropped
in flight. The shell then resolves an app alias through `m_index` (desktop id →
name + themed `iconPath`) and pushes only `name`/`iconPath`/count to QML: an
app drag opens the gap with the real tile and name, a file drag shows the file
count (or the single file's name). While hovering, the target entry draws a
capsule from the pure `dockDropAffordance` helper: `Open with <app>` for an app
entry, `Move to Trash` (or `Trash unavailable` when the backend is down),
`Move to Downloads` for the stack, and no highlight for the divider or empty
Dock. Dropping an alias that is already pinned pulses the existing entry
(`flashPin`) instead of silently no-opping. Failed launches, an unavailable
Trash, and partial Trash/Downloads moves raise the Dock's notification notice
(`raiseDockLaunchFailure`/`raiseDockNotice`), and multi-file semantics are one
launch with all arguments or one move/trash for every path. `dockdrops` stays
the single decision point; the ghost and affordances are presentation only.

### Dock folders and stacks

A folder entry is a **stack**, never an app launch: clicking it shows its
contents in the Dock popover (the macOS list equivalent; fan/grid layouts are
future polish), the popover and context menu open the folder in Files, and a
double-click opens the folder in Files directly. The popover header carries the
folder icon, the elided folder name, and the explicit **Open in Files** action
(reachable by keyboard, like the rows); a folder longer than the visible-row
budget scrolls and summarizes the remainder as "N more…", and an empty folder
shows an empty row rather than a blank panel. The Dock renders a designed
folder silhouette from our own geometry — a tab, a front face with a vertical
gradient, a rim, and an inner sheen — never Apple artwork. Any folder dragged
onto the Dock is pinned as a stack, persisted as a path list in
`dock.pinnedFolders` (settingsd-owned) and listed read-only through
`files-core`; the Downloads stack is the default member of the same widget. A
missing path degrades to a dimmed entry with a notice. The folder's name is
never drawn inside the artwork — it comes from the hover `Tooltip` and the
popover header. See
[ADR 0092](adr/0092-dock-folder-stacks-and-folder-pins.md) and the
T-14.7h/T-14.7k units.

### Activation and launch

Every click resolves through the documented click tree (pinned not running →
launch; running → focus, switching Space and restoring if minimized; multiple
windows → chooser; a stack → its contents; Trash → Files). A launch must never
silently no-op: the shell provides launched clients the session's display
environment, and a missing identity or a vanished window surfaces a notice
rather than a dead click. See T-14.7g.

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

**T-11.4b status.** The OSD is keyboard/AT-SPI accessible and the T-11 capture
set is committed. The alert carries an `Accessible.Alert` name and description
("Volume 60 percent. Press Escape to dismiss.") and an `Accessible.onPressAction`
that clears it; because the surface never takes keyboard focus, `Escape` is
handled at the shell level (`ShellController::onKeyEvent` hides a visible OSD
without stealing focus) and the view's `dismissed()` signal routes to the same
`hideOsd()` path as the auto-dismiss. The T-11.4a capture exposed the
design-system `volume` glyph as a battery read-alike; it is now a speaker. The
committed stills are `docs/captures/t11-control-center.*`,
`docs/captures/t11-osd.png` (+ `-context`), and `docs/captures/t11-dnd.png`
(the menu-bar DND crescent), produced by `scripts/capture-osd-dnd.sh`
(`make osd-dnd-capture`). See [adr/0063](adr/0063-osd-keyboard-atspi.md).

## Relationship to compositor and services

- Compositor state (windows, workspaces, outputs) arrives via private
  protocols; the shell never duplicates compositor state machines.
- System state (network, audio, power) arrives via the adapters in
  [07-system-integration.md](07-system-integration.md); the shell never talks
  to hardware directly.
- The shell is crashable and restartable without taking down the compositor.
