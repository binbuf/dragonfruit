# Total desktop experience

| | |
|---|---|
| **Status** | target design |
| **Date** | 2026-11-06 |
| **Release** | v0.2.0 |
| **Scope** | every core shell surface, interaction, animation, HUD, menu-bar extra, and System Settings pane |
| **Related** | [Track 22](tracks/22-total-experience-parity.md), [settings map](settings-business-logic-map.md), [distro contract](distro-compatibility.md), [naming/licensing](interface-naming-and-licensing.md), [motion/feedback](motion-and-feedback.md), [evidence/acceptance](evidence-and-acceptance.md) |

## 1. Why this design exists

The T-20/T-21 and liquid-glass task sets answered the **material** question:
what the Dock, menu bar, popovers, Control Center, notifications, and window
chrome are made of, and how to render them on the GPU. That work is necessary
but not sufficient for a desktop that feels finished.

The remaining gap is the **experience**: what each surface does, how it moves,
what happens on hover/press/disabled/error/cancel, how it is labelled, and
which Linux component owns the setting behind it. A user does not judge a Dock
by its blur sigma; they judge it by whether a click launches, focuses, opens a
window chooser, bounces, or fails visibly. They do not judge System Settings by
its sidebar width; they judge it by whether every row changes something real.

This document defines the target for that experience. The liquid-glass
specification remains the material source of truth; this document is the
behaviour, information-architecture, motion, and ownership source of truth.

## 2. Product goal

**Reproduce the interaction model and perceived quality of macOS's core
desktop as an original, measured, cross-distribution Linux desktop
environment.**

The target is behavioural and perceptual parity, not a pixel copy:

- the same mental model and spatial organisation;
- the same information architecture and wording, adapted only where Linux has
  a different concept;
- the same motion language, measured from the reference;
- the same quality bar at 1×/1.5×/2×, with reduced motion and accessibility
  variants;
- original assets, names, sounds, and code.

### 2.1 Must match vs may differ

| Must match | May differ |
|---|---|
| The click/key/gesture tree and its outcomes | Exact glyph artwork, icon shapes, brand colour |
| Pane/section/row order, labels, control types | Linux-specific rows added by a distro adapter |
| Focus, hover, press, disabled, empty, error, cancel states | The exact material (Dragonfruit Glass, not Apple's) |
| Motion duration, direction, interruption, reduced-motion behaviour | The exact hardware-dependent animation (e.g. genie warp) |
| Spatial hierarchy, spacing scale, typography scale | The default accent colour and wallpaper |
| Keyboard paths, accessible names/roles, focus visibility | App-specific chrome (CSD apps keep their own titlebars) |
| Absence behaviour: no fake toggles, no silent no-ops | Omitted Apple services (iCloud, Siri, Screen Time, etc.) |
| Settings ownership and persistence | The underlying host service or package manager |
| Multi-display/active-display rules | The exact display-manager integration |

The "may differ" column is not a licence to skip behaviour. It is the list of
places where a Linux concept or Dragonfruit's own identity replaces Apple's
implementation without changing the mental model.

## 3. Design principles

1. **One owner per state class.** Windows, workspaces, outputs, input, and
   materials belong to the compositor; desktop settings to `settingsd`;
   identity to `app-index`; menus to `menu-broker`; notifications to the
   notification service. The shell projects and requests; it never becomes a
   second source of truth.
2. **Evidence before implementation.** Every geometry, timing, material, label,
   and state comes from a capture or a named host API. If evidence is missing,
   the task writes `OPEN`; it does not invent a value.
3. **Vertical slices.** A task crosses compositor → shell → service → app where
   needed, so integration failures surface early.
4. **No dead affordances.** Every click, key, gesture, and menu row either does
   something visible or is disabled with a reason. No silent no-ops.
5. **Interruptible, progress-based motion.** Gesture-driven transitions are
   progress-based and reversible. Discrete "instant" paths are bugs except
   where the reference is discrete.
6. **Reduced motion and accessibility are not variants.** Every animation has a
   reduced-motion path; every surface has a keyboard path and an accessible
   name. The accessibility settings are part of the design, not a later audit.
7. **Original assets and generic names.** Reproduce the interaction, never
   Apple's bitmaps, symbols, sounds, trademarks, or code. The naming map is in
   [interface-naming-and-licensing.md](interface-naming-and-licensing.md).
8. **Distro-neutral.** Reuse the host's NetworkManager, BlueZ, PipeWire,
   UPower, UDisks2, CUPS, SANE, accountsservice, PackageKit, portals, and
   display manager. Distro differences live behind adapters.
9. **One material system.** All chrome consumes the compositor material roles
   and the design-system tokens. No surface invents its own glass.
10. **One motion system.** All animations consume the shared motion tokens and
    the animation clock. No surface invents its own duration or curve.

## 4. Surface inventory and target state

`Implemented` means the core exists and has tests/captures. `Partial` means a
named part is missing. `Design-only` means this document is the first target.
The detailed gap register is in [STATUS.md](STATUS.md).

### 4.1 Always-visible shell

| Surface | Current | Target |
|---|---|---|
| Menu bar | Implemented | Full app/global menu, system menu, status items, extras, clock/date, overflow/notch, auto-hide, per-display |
| Dock | Implemented | Full click tree, launch/quit, bounce, minimize/genie, magnification, auto-hide, stacks, Trash, badges, chooser, drag/drop, keyboard, multi-display |
| Desktop | Implemented | Icons, selection, context menus, Quick Look, stacks, widgets, Trash |
| Control Center | Implemented | Full module set, expanded sheets, customization, live service state |
| Notification Center | Partial | Banners, history, stack, lower panel, widgets, Focus, actions, per-display |
| Applications drawer | Implemented | Search, categories, folders, paging, keyboard, launch feedback |

### 4.2 Navigation and window management

| Surface | Current | Target |
|---|---|---|
| Overview / Mission Control | Implemented | Paging, spaces strip, drag between workspaces, add/remove, fullscreen, gestures, hot corners, Desktop Reveal, multi-display |
| App switcher | Implemented | App/window modes, cycling, previews, cancel, Cmd+`, multi-display |
| Stage Manager | Design-only | Stage strip, recent apps, stage groups, drag, fullscreen |
| Window tiling / Split View | Design-only | Halves, quarters, drag-to-edge, menu, keyboard |
| Window lifecycle | Implemented | Open/close/minimize/restore/zoom/fullscreen/genie, launch/quit, Force Quit |
| Quick Look | Design-only | Image/PDF/video/multi-item, navigation, actions, fullscreen |
| Login / lock / first run | Partial | Lock, login, fast user switching, Setup Assistant, power actions |

### 4.3 Search and launching

| Surface | Current | Target |
|---|---|---|
| Search | Design-only (app-index exists) | Invocation, query states, categories, results, previews, actions, keyboard, privacy, indexing |
| Applications drawer | Implemented | Full parity and launch feedback |
| Add Application picker | Implemented | Full parity |

### 4.4 Settings

| Area | Current | Target |
|---|---|---|
| Pane catalog | 22 shipped | Full catalog, with explicit omit/adapt/delegate decisions |
| Search/deep links | Partial | Every pane/row searchable; deep links from shell and apps |
| Live apply | Implemented | Validation, admin auth, absence, per-row ownership |
| Row-level mapping | Partial | Every row mapped in [settings-business-logic-map.md](settings-business-logic-map.md) |
| Cross-distro | Partial | Fedora/Ubuntu/Debian/Arch contract in [distro-compatibility.md](distro-compatibility.md) |

### 4.5 Transients and feedback

| Surface | Current | Target |
|---|---|---|
| HUDs/OSDs | Partial (volume/brightness) | Volume, brightness, keyboard backlight, charging, low battery, mirroring, Focus, and other confirmations |
| Menus/popovers | Implemented | Full item states, keyboard, submenus, scroll, sheets, alerts |
| Notifications | Partial | Banner/stack/history/actions/Focus |
| Sound | Partial (settings keys) | UI sound language, alerts, startup, volume feedback |
| Cursor | Partial | Pointer shapes, hotspots, hover/drag states, HiDPI |

## 5. Behaviour contracts

These are the rules a task must implement and a test must pin. They are
deliberately written as observable behaviour, not implementation detail.

### 5.1 Menu bar

- The bar is always present unless auto-hidden/fullscreen-hidden.
- The leftmost system menu is shell-owned and always present.
- The focused application's name and menus follow the system menu; an app that
  exports nothing still gets the fixed application menu.
- The global menu is the application's menu model, not a screenshot or a
  duplicate model. Focus changes update it; no focus means Files (the desktop
  owner).
- Status items are a registry: Wi-Fi, Bluetooth, Battery, Sound, Search,
  Control Center, Notification Center, Date & Time, Focus, Screen Mirroring,
  and third-party StatusNotifier items. Each item has a menu, a tooltip, a
  keyboard path, and an absent/error state.
- Menu rows share one anatomy: leading icon/check gutter, label, shortcut
  hint, trailing chevron/badge. Disabled rows explain why; checked/mixed states
  are visible; submenus open in the correct direction and stay on screen.
- Menu-bar auto-hide reveals on the top edge and hides after the pointer
  leaves, with a measured delay/curve. Fullscreen apps follow the same policy.
- On a notched/overflowing display, items are prioritised, not clipped. A
  third-party item never covers a system item.
- Multi-display: each display has a bar; the active display owns the menu;
  status items follow the active display or the configured policy.

### 5.2 Dock

The click tree is the contract:

```text
Pinned app
  ├─ not running → launch (bounce, launch-origin tile)
  └─ running
      ├─ one window → activate (switch workspace, restore if minimized)
      └─ multiple windows → window chooser (most recent first)
Running but not pinned
  └─ temporary entry with the same click tree
Folder stack
  └─ open popover list; Open in Files; drop moves into folder
Trash
  └─ open Files/Trash; drop moves to Trash; Empty Trash is async
Divider
  └─ Add Application picker; drag resize; reorder
```

- A click never dies silently. A missing identity, vanished window, failed
  launch, or unavailable Trash raises a visible notice.
- The running indicator distinguishes focused vs merely running.
- The window chooser lists every window across workspaces, with the focused
  checkmark, minimized marker, workspace name, per-row Minimize/Restore and
  Close, bounded height, and keyboard navigation.
- Magnification is a continuous warp of the resting row; the pointer maps to
  itself; the plate grows once on engagement and does not track horizontal
  movement; reduced motion tracks directly.
- Auto-hide is interactive and progress-based; the reserved zone is removed
  while hidden; a fullscreen app does not leave a reserved strip.
- Drag/reorder persists through `settingsd`; the Dock never writes its own
  config.
- Stacks, Trash, badges, and overflow have distinct states and accessible
  names; none is a dead glyph.
- Multi-display: one Dock per configured display or one active-display Dock,
  per the setting; window targets follow the display.
- The Dock is keyboard-operable: arrows, Return, Escape, type-ahead,
  Ctrl+Shift+Arrow reorder, and a visible focus ring.

### 5.3 Overview / Mission Control and workspaces

- One state machine owns workspace switch, overview, and Desktop Reveal; there
  is no second animation path.
- Enter/exit is progress-based and interruptible; a gesture can reverse mid-flight.
- The grid shows live surfaces, not screenshots; video keeps playing; the
  window under the pointer is hit-testable and selectable.
- The workspaces strip shows every workspace, the active one, fullscreen
  spaces, and add/remove affordances. Paging is readable at the floor.
- Dragging a window between workspaces moves the real window and settles the
  grid; dropping outside a workspace cancels.
- Desktop Reveal moves windows aside without losing focus/stacking.
- Hot corners and gestures share one policy and one settings source.
- Multi-display: each display has its own grid/active workspace; the overview
  never mixes outputs.
- Reduced motion: the grid appears/disappears without travel; state changes
  remain legible.

### 5.4 App switcher

- Cmd+Tab opens the switcher; holding Cmd keeps it open; Tab/Shift+Tab cycles;
  releasing Cmd commits; Escape cancels.
- The default mode is applications; a window mode is available where the
  reference exposes it. Cmd+` cycles windows within the focused app.
- The selected app/window is visibly selected and scrolled into view.
- Minimized/hidden apps are represented with their state; committing restores
  or unhides as needed.
- The switcher is one overlay on the active display; it does not steal focus
  from the app until commit.
- Reduced motion: the selection still moves; the open/close animation collapses.

### 5.5 Search

- Cmd+Space (or the configured shortcut) opens Search on the active display.
- The field is focused immediately; typing updates results without a modal
  spinner for local results.
- Results are grouped by category (applications, files, actions, calculations,
  definitions, web). The top hit is selected; arrows move; Return opens;
  Cmd+Return reveals; Escape closes.
- A preview (Quick Look) is available without leaving Search.
- Actions are explicit: Open, Show in Files, Copy Path, etc. A result that
  cannot act is not shown as actionable.
- Indexing state, no-results, permission-denied, and service-absent states are
  visible and honest.
- Search privacy settings control what is indexed; the index is local by
  default.
- The Applications drawer's search is separate: it filters the app corpus, not
  the system index.

### 5.6 Applications drawer

- Opens as a full-output overlay on the active display; Escape/launch closes.
- Every launchable app appears once, alphabetically by default, with its
  themed icon and localized name.
- Search filters by name/id; category pills filter by freedesktop categories;
  the default is All.
- The grid is bounded by columns/rows and scrolls/pages without clipping; a
  narrow output drops columns.
- Folders group apps and open in place; a folder never hides apps from search.
- Keyboard: arrows, Return, Escape, type-to-search; focus is visible.
- Launch uses the same path as the Dock and shows the same failure notice.

### 5.7 Control Center and notifications

- Control Center is a panel of modules with live state; each module opens an
  expanded sheet where the reference does.
- A module with an absent service hides or shows an explicit unavailable
  state; it never shows a fake toggle.
- Notification banners enter/exit with the measured motion, stack, and timeout;
  actions are reachable by keyboard.
- Notification Center shows history, grouped stacks, the lower panel/widgets,
  and a clear-all action; clicking the clock toggles it.
- Focus/DND is one policy shared by the menu bar, Control Center, Notification
  Center, and the notification service.
- Per-app notification settings are real; the app list comes from the
  notification service, not a hardcoded list.

### 5.8 Window lifecycle, Stage Manager, Quick Look

- Window open/close/minimize/restore/zoom/fullscreen use the shared animation
  clock and motion tokens; each has a reduced-motion path.
- Minimize target follows `dock.minimizeIntoTileIcon` and
  `dock.minimizedAnimation`; the Dock icon reacts if enabled.
- App quit closes its windows with the same lifecycle; the last window close
  may keep the app running per platform policy.
- Force Quit lists apps, warns about unsaved changes, and terminates; it is
  keyboard-operable.
- Stage Manager groups windows into stages, shows recent apps, and switches
  stages with the measured motion; it is a window-management mode, not a
  second workspace model.
- Window tiling/split view uses compositor geometry and the same snap targets
  as drag-to-edge.
- Quick Look opens on Space for the selection, renders the real file, supports
  navigation, and closes on Space/Escape.

### 5.9 Settings

- Every visible row maps to an owner in
  [settings-business-logic-map.md](settings-business-logic-map.md).
- Controls apply live unless the reference requires an explicit action; a
  failed write is visible and reverts.
- The pane catalog is the single source of truth: a row without a body cannot
  appear; a body without a row cannot be reached.
- Search indexes pane titles, section titles, row labels, and synonyms; a
  result deep-links to the row and highlights it.
- Absent providers show a one-line explanation and disable only the affected
  controls; settingsd-backed controls stay live on schema defaults.
- Admin actions use polkit/PAM; the prompt is the system prompt, not a custom
  password field.
- The pane adapts to distro differences through adapters; it never shells out
  to a distro command from QML.

### 5.10 HUDs, sound, cursor

- HUDs appear on the active display, show the real value/state, time out, and
  are replaced (not stacked) by a newer HUD of the same kind.
- Volume/brightness/keyboard-backlight HUDs follow the real key path and
  respect mute/limits.
- Charging/low-battery HUDs are distinct; low battery is an alert, not a
  silent state.
- UI sounds are original or permissively licensed; they follow the
  `sound.uiEffects` and volume-feedback settings; no Apple sounds.
- Cursor shapes are original/permissively licensed, HiDPI-crisp, and correct
  for resize/move/drag/text/busy states.

## 6. Ownership boundary

| State / behaviour | Owner | Consumers |
|---|---|---|
| Windows, focus, stacking, workspaces, outputs, input, gestures, lock, materials | Compositor | Shell, apps |
| Desktop settings, schema, persistence, migrations | `settingsd` | Compositor, shell, apps |
| App identity, desktop entries, icons, launch registry, recency | `app-index` | Shell, Settings, Files |
| Menu model and dispatch | `menu-broker` | Shell menu bar |
| Notifications, Focus/DND policy, history | Notification service | Shell, Control Center, apps |
| Wallpaper catalogue/default | `wallpaperd` | Shell, compositor |
| Host service state (network, Bluetooth, audio, power, storage, printers, accounts, updates) | Existing daemons via adapters | Shell, Settings |
| Permissions | xdg-desktop-portal `PermissionStore` | Settings, apps |
| File operations, Trash, desktop icons | `files-core` / Files process | Dock, desktop |
| Search index | Search service (to be chosen) | Search UI |
| Distro administration | Distro adapter | Settings |

The shell is a **projection and interaction** layer. It may cache a projection
for a frame, but it does not own the authoritative state.

## 7. Cross-surface rules

- **Focus.** One focused window per seat; the shell chrome takes focus only
  when it needs it (menu, Dock keyboard, overview, switcher, search). Releasing
  focus returns it to the previous window.
- **Z-order.** Chrome layers: desktop < windows < Dock < menu bar < popovers <
  overview/switcher/search/drawer < lock. Notifications and HUDs sit above
  windows but below lock. The compositor enforces the order; the shell does not
  guess.
- **Animation.** One clock, one token set, one reduced-motion switch. A
  surface may have its own duration token, but it may not invent a curve.
- **Materials.** Every chrome surface consumes a `MaterialRole` and the
  design-system `Glass`/`Material` singleton. No opaque fill on a surface that
  should show the material; no material on opaque content.
- **Absence.** Every host service can be absent. Absence is a visible, named
  state, not an error dialog and not a silent no-op.
- **Multi-display.** Every surface has an explicit active-display rule. A
  surface never spans displays unless the reference does.
- **Scaling.** Layout is logical; rendering is physical. Hit targets, reserved
  zones, and popover anchors are logical. Blur/lens/rim dimensions scale with
  the output.
- **Localization.** No hardcoded English in QML; use the translation catalog.
  Long strings must not clip controls. The reference is en-US; capture at least
  one long-string locale.
- **Accessibility.** Every interactive element has an accessible name, role,
  state, and keyboard path. Focus is visible. Reduced motion/transparency/
  contrast are live.

## 8. Cross-distribution stance

Dragonfruit is a desktop environment, not a distribution. It must work on:

- Fedora 44+ (reference);
- Ubuntu 24.04+/26.04+;
- Debian 13+;
- Arch Linux (rolling);
- compatible derivatives where the same host services exist.

The contract is in [distro-compatibility.md](distro-compatibility.md). The
short version:

- **Host services are dependencies, not forks.** NetworkManager, BlueZ,
  PipeWire/WirePlumber, UPower, UDisks2, CUPS, SANE, accountsservice, polkit,
  PackageKit, xdg-desktop-portal, and the display manager stay with the distro.
- **Distro differences live behind adapters.** Package management, updates,
  firewall, time sync, locale, users/groups, and default handlers have a
  distro-neutral interface and a tested backend per distro.
- **No distro-specific QML.** The Settings app talks to D-Bus/services; it
  never embeds `dnf`, `apt`, or `pacman` commands.
- **Optional third-party services are declared.** Search, backup, online
  accounts, accessibility, and IME may delegate to an existing project; the
  dependency, license, and absence behaviour are documented.
- **Packaging is a first-class deliverable.** RPM, DEB, and Arch packages; CI
  on each; a clean-VM install test.

## 9. Evidence and acceptance

The target is accepted when:

- Every `S-*`, `N-*`, `Q-*`, `M-*`, `H-*`, and `SET-*` row in the capture
  inventory has a still/motion/AX artifact or an `OPEN` row.
- Every animation has a measured duration/curve and a reduced-motion path.
- Every Settings row has an owner and an absent-provider state.
- Every surface has a keyboard path and an accessible name.
- Every host service has an absence test and a distro adapter test where
  applicable.
- The nested and native visual checks pass at 1×/1.5×/2×, light/dark,
  Full/Reduced/Minimal material.
- The performance budgets in [ROADMAP.md](../ROADMAP.md) still hold, with new
  per-surface budgets for the added animations.
- `make gates` stays green; no task silently defers a user-facing half.

## 10. Non-goals

- Pixel-identical macOS screenshots, Apple assets, SF Symbols, sounds, or
  trademarks.
- Apple account services (iCloud, Siri, Apple Intelligence, Screen Time,
  Wallet, Game Center, Find My, App Store).
- A new display server, renderer, package manager, network stack, audio stack,
  or privilege-escalation scheme.
- A second settings daemon, menu model, notification service, or window model.
- Live compositor handoff or in-place DE swap.
- Full parity with every third-party app's custom chrome; CSD apps keep their
  own titlebars.
- Guaranteed AirDrop/Continuity/Handoff compatibility; optional local-sharing
  integrations are evaluated separately.

## 11. Scope decisions made in this design

The original ask named Settings, the Applications drawer, Overview/Mission
Control, the app switcher, Search, the Dock, the menu bar, menu-bar extras,
animations, and HUDs. The following adjacent core surfaces were added because a
desktop that omits them still feels unfinished; each has a task and an
inventory id:

| Added | Why | Task |
|---|---|---|
| Stage and window tiling/split view | Core macOS window-management modes; the reference exposes them in Desktop & Dock | T242 |
| Preview (Quick Look) | Core file-browsing affordance; Space is muscle memory | T244 |
| System dialogs, sheets, alerts, file/print/color/emoji pickers | Core OS chrome; the reference inventory includes them | T245a |
| Desktop items and interaction | Core desktop surface; Files owns it, but the experience is part of the DE | T245b |
| Screen saver runtime | Core idle/lock surface; the Settings pane already exists | T245a |
| Login, lock, first run | Core session experience; lock/session machinery exists | T245 |
| Notification widgets | Part of the reference Notification Center | T234 |
| HUDs, sound, cursor | Core feedback language; the ask explicitly named volume/brightness HUDs | T249–T251 |
| Accessibility and IME | Core usability; existing gaps G-19/G-20 | T252/T253 |

The following are deliberately **not** added: Apple account services (iCloud,
Siri, Apple Intelligence, Screen Time, Wallet, Game Center, Find My, App Store),
AirDrop/Continuity/Handoff by default, a new package manager/network/audio
stack, and a second settings/menu/notification/window model. Optional
third-party substitutions are listed in
[interface-naming-and-licensing.md](interface-naming-and-licensing.md).

## 12. Risks

| Risk | Mitigation |
|---|---|
| Scope explosion: every macOS surface becomes a task | The inventory is the contract; omit/delegate decisions are explicit; tasks are one session |
| Capture evidence is incomplete or from the wrong build | Every task records build/axes; missing evidence is `OPEN`, never invented |
| Settings rows become fake toggles | The settings map requires an owner and an absence test per row |
| Distro differences leak into QML | Adapter trait + per-distro backend; no shell commands in QML |
| Motion becomes a pile of per-surface durations | One motion token set + one clock; measured tokens only |
| Accessibility is deferred again | Every task's done-when includes keyboard/AT-SPI/reduced-motion checks |
| Third-party licensing contaminates MIT code | Separate-process/dynamic-link policy; ADR 0220; packaging review |
| Performance regresses with more chrome | Per-surface budgets, T-21 presentation path, degrade tiers |
| Naming/branding risk | Generic-name map + asset provenance + gate |

## 13. References

- [Track 22 — Total experience parity](tracks/22-total-experience-parity.md)
- [Settings business-logic map](settings-business-logic-map.md)
- [Cross-distro compatibility](distro-compatibility.md)
- [Interface naming and licensing](interface-naming-and-licensing.md)
- [Motion and feedback](motion-and-feedback.md)
- [Evidence and acceptance](evidence-and-acceptance.md)
- [macOS surface inventory](../reference/desktop-experience-inventory.md)
- [Existing shell design](04-shell.md)
- [Existing workspaces design](03-workspaces.md)
- [Existing settings design](08-settings.md)
- [Existing global menu design](06-global-menu.md)
- [ADR 0219](adr/0219-total-experience-scope-and-parity-bar.md) — scope and parity bar
- [ADR 0220](adr/0220-generic-naming-and-third-party-substitution.md) — naming/licensing
- [ADR 0221](adr/0221-cross-distro-host-service-contract.md) — host-service contract
