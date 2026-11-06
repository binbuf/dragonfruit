# Desktop experience inventory

This is the Dragonfruit-facing distillation of the macOS 27 reference
inventory. The full capture checklist with stable ids lives in the reference
repository:

- `macos-liquid-glass-review/docs/SURFACE-INVENTORY.md`
- `macos-liquid-glass-review/docs/CAPTURE-PLAN.md`

Use this file to scope a task; use the reference inventory to capture or
measure it. The local macOS screenshots are **reference only and never ship**.

## 1. Always-visible shell

### 1.1 Menu bar

| Area | States / behaviours |
|---|---|
| Bar | Light/dark, desktop/app focus, fullscreen, auto-hide, notch/overflow, multi-display, keyboard focus, reduced transparency/contrast |
| System menu | Closed/open, About, Recent Items, Force Quit, Sleep/Restart/Shut Down, Lock/Log Out, update badge |
| Application menu | Native export, fixed fallback, enabled/disabled/checked/mixed, submenu, icons, shortcuts, dynamic items |
| Status items | Wi-Fi, Bluetooth, Battery, Sound, Search, Quick Settings, Notifications, Date & Time, Focus, Screen Casting, third-party |
| Menus | Item anatomy, hover/press/disabled, submenu direction, scroll, keyboard, close animation |
| Clock | Date/time formats, seconds, 24-hour, calendar popover |

### 1.2 Dock

| Area | States / behaviours |
|---|---|
| Rest | Bottom/left/right, size, magnification off/on, light/dark, reduced transparency/contrast |
| Hover | Wash, magnification, tooltip, plate growth, reduced motion |
| Press/launch | Press, launch bounce, launch-origin hand-off, failure notice |
| Running | Focused/running indicator, minimized state, window count badge, app badge |
| Click tree | Pinned not running → launch; running one window → activate; multiple → chooser |
| Window chooser | Rows, focused check, minimized marker, workspace, per-row Minimize/Close, overflow, keyboard |
| Stacks | Folder list, header, Open in Files, empty/missing, drop into folder |
| Trash | Empty/full/unavailable, empty progress, failure/retry |
| Drag/drop | Reorder, external file/app/folder, affordance, pin |
| Auto-hide | Hidden, reveal, hide, interactive slide, reserved zone |
| Overflow | Overflow cell, More Windows, bounded list |
| Keyboard | Focus ring, arrows, Return, Escape, type-ahead, Ctrl+Shift+Arrow reorder |
| Multi-display | Per-display Dock, active-display policy, fullscreen |

### 1.3 Desktop

- Wallpaper, desktop icons, grid, labels, selection (single/multi/marquee),
  context menu, Quick Look, stacks, mounted volumes, widgets, Trash.
- Desktop is owned by Files; the shell does not own the file model.

### 1.4 Quick Settings (Control Center)

- Outer panel, modules, expanded sheets, customization, live service state.
- Modules: Wi-Fi, Bluetooth, Focus, Screen Casting, Now Playing, Sound,
  Display, Battery, Dark Mode, Stage, Screen Recording, Keyboard Brightness,
  Hearing.
- Menu-bar-only items: Clock, Search, Wi-Fi, Bluetooth, Battery, Focus, Screen
  Casting, Display, Sound.

### 1.5 Notifications

- Banners, stack, history, lower panel, widgets, calendar, weather, Focus,
  actions, clear, per-app settings, per-display.

## 2. Navigation and window management

### 2.1 Overview / Mission Control

- Enter/exit, grid, workspaces strip, 1/many windows, fullscreen workspace,
  minimized strip, hover/selection, drag between workspaces, add/remove
  workspace, Desktop Reveal, multi-display, keyboard, gesture progress,
  reduced motion.

### 2.2 App switcher

- App mode, window mode, forward/backward cycling, many apps, minimized/hidden,
  cancel, commit, Cmd+` window cycle, multi-display, reduced motion.

### 2.3 Stage and tiling

- Stage off/on, empty/one/many apps, drag to stage, recent apps, fullscreen,
  halves/quarters, drag-to-edge, Move & Resize menu, Split View.

### 2.4 Window lifecycle

- Open, close, minimize (genie/scale), restore, zoom, fullscreen, drag, resize,
  focus/unfocus, titlebar double-click, app quit, unexpected quit, Force Quit,
  launch bounce, reopen.

### 2.5 Preview (Quick Look)

- Image, PDF/document, video, multiple items, navigation, fullscreen, actions.

### 2.6 Login, lock, first run

- Lock screen, password, fingerprint, login window, fast user switching,
  sleep/wake, shut down/restart, Welcome (Setup Assistant) panes.

## 3. Search and launching

### 3.1 Search

- Closed, empty, app/file/calculation/definition/web results, categories, top
  hit, preview, actions, no results, indexing, privacy, keyboard, multi-display.

### 3.2 Applications drawer

- Closed, all apps, search empty/results, category, many apps/scroll, folder,
  launch, keyboard, no apps, multi-display.

### 3.3 Add Application picker

- Open, search, pin/unpin, app-index absent.

## 4. Menus, popovers, sheets, dialogs

- Menu open/close, submenu, hover/press/disabled/checked/mixed, separator,
  scroll, keyboard.
- Context menus: desktop, Dock, Files, menu bar, status item.
- Tooltip, popover with arrow, sheet, alert, dialog with checkbox, file
  open/save, print, color picker, emoji & symbols.

## 5. HUDs and transient feedback

| HUD | Trigger | Notes |
|---|---|---|
| Volume | Volume keys/menu | Bar, percentage, mute |
| Brightness | Brightness keys/Quick Settings | Per-display |
| Keyboard backlight | Backlight keys | Hardware-dependent |
| Mute | Mute key | Distinct from 0 |
| Charging | Plug in | Percentage/state |
| Low battery | Threshold | Alert |
| Battery full | Charge complete | Optional |
| Screen casting | Start/stop | Device name |
| Focus | Toggle | Mode |
| Wi-Fi/Bluetooth off/on | Toggle | Optional |
| Nearby transfer | Start/stop | If adapted |
| Screenshot flash | Capture | Very short |
| Screen recording | Start/stop | Persistent indicator |
| Camera/mic in use | Privacy | Persistent dot |

## 6. System Settings

The full pane catalog and row-level owner map is
[settings-business-logic-map.md](../design/settings-business-logic-map.md).
The capture checklist is the reference inventory's `SET-*` section.

Shipped/adapted pane groups:

- Wi-Fi, Bluetooth, Network (VPN/firewall), Battery.
- General: About, Updates, Storage, Login Items & Extensions, Language &
  Region, Date & Time, Sharing, Backup, Reset, Startup Disk.
- Appearance, Accessibility, Quick Settings, Search, Privacy & Security,
  Desktop & Dock, Displays, Wallpaper, Screen Saver, Notifications, Sound,
  Focus, Lock Screen, Biometrics & Password, Users & Groups, Online Accounts,
  Printers & Scanners, Keyboard, Mouse, Trackpad, Game Controllers.
- Omitted: Apple Account, AirDrop & Handoff, Device Management, Screen Time,
  Game Center, Wallet & Apple Pay, Siri, Apple Intelligence.

## 7. Motion inventory

See [motion-and-feedback.md](../design/motion-and-feedback.md) and the
reference `A-*` list. Priority transitions:

- Dock magnification/launch/minimize/auto-hide.
- Window open/close/minimize/restore/zoom/fullscreen.
- Overview enter/exit/workspace switch/Desktop Reveal.
- App switcher open/cycle/commit/cancel.
- Search open/close/results.
- Applications drawer open/close/page.
- Quick Settings/Notifications.
- Menu open/close/submenu.
- HUDs.
- Lock/unlock, Preview, Stage/tiling.

## 8. Current implementation snapshot

| Surface | State | Where |
|---|---|---|
| Menu bar | Implemented | `shell/menubar/`, `shell/src/shellmenus.*` |
| Dock | Implemented | `shell/dock/`, `shell/src/dock*.cpp` |
| Overview | Implemented | `compositor/src/overview/`, `shell/overview/` |
| App switcher | Implemented | `compositor/src/app_switcher.rs`, `shell/switcher/` |
| Applications drawer | Implemented | `shell/apps-drawer/`, `shell/src/appsdrawer.*` |
| Quick Settings | Implemented | `shell/control-center/`, `shell/src/controlcenterpolicy.*` |
| Notifications | Partial | `shell/notifications/`, `services/notifications/` |
| Search | Design-only | `services/app-index/` is the base |
| Settings | 22 panes | `apps/settings/` |
| Stage/tiling | Design-only | — |
| Preview | Design-only | — |
| HUDs | Partial | `shell/osd/`, `shell/src/osdmodel.*` |
| Sound | Partial | `sound.*` keys |
| Cursor | Partial | `compositor/src/cursor_theme.rs` |

The authoritative status/gap register is [STATUS.md](../design/STATUS.md).

## 9. Evidence ids

When a task needs a reference artifact, cite the stable id from the reference
inventory, e.g. `S-DOCK-11` (window chooser), `A-WIN-03` (zoom), `H-VOL-01`
(volume HUD), `SET-DD-01` (Desktop & Dock). The task's hand-off should record
the artifact path and the measured value.

## 10. Related

- [Total experience design](../design/22-total-experience.md)
- [Settings business-logic map](../design/settings-business-logic-map.md)
- [Motion/feedback](../design/motion-and-feedback.md)
- [Cross-distro contract](../design/distro-compatibility.md)
- [Naming/licensing](../design/interface-naming-and-licensing.md)
- [Evidence/acceptance](../design/evidence-and-acceptance.md)
- [Existing macOS UI inventory](macos-ui-inventory.md)
- [Existing System Settings reference](System_Preferences.md)
