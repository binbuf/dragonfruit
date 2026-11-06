# v0.2.0 — Total desktop experience task set

This directory is the v0.2.0 task set. It continues the repository-wide task
number sequence at **T209** and implements
[Track 22 — Total desktop experience parity](../../design/tracks/22-total-experience-parity.md).

The release is deliberately broad but ordered: **capture the reference, map
every setting to an owner, then close the visible experience gaps.** The
material/presentation work from T-20/T-21 and the liquid-glass set stays; this
set adds the behaviour, information architecture, motion, HUDs, and
cross-distro integration.

## Order

| Task | File | Purpose | Host |
|---|---|---|---|
| T209 | [209-t-22.0-experience-inventory.md](209-t-22.0-experience-inventory.md) | Inventory every surface/state/animation/HUD/Settings pane; manifest skeleton. | macOS |
| T210 | [210-t-22.0-capture-harness.md](210-t-22.0-capture-harness.md) | Extend the capture harness for motion, HUDs, AX, and manifests. | macOS |
| T211 | [211-t-22.0-settings-capture.md](211-t-22.0-settings-capture.md) | Capture every Settings pane, sub-pane, popup, and search state. | macOS |
| T212 | [212-t-22.0-menubar-capture.md](212-t-22.0-menubar-capture.md) | Capture menu bar, system/app menus, status items, and extras. | macOS |
| T213 | [213-t-22.0-dock-capture.md](213-t-22.0-dock-capture.md) | Capture the Dock state/behaviour/motion matrix. | macOS |
| T214 | [214-t-22.0-overview-capture.md](214-t-22.0-overview-capture.md) | Capture Overview/Mission Control and workspaces. | macOS |
| T215 | [215-t-22.0-switcher-lifecycle-capture.md](215-t-22.0-switcher-lifecycle-capture.md) | Capture app switcher, app lifecycle, Force Quit. | macOS |
| T216 | [216-t-22.0-search-drawer-capture.md](216-t-22.0-search-drawer-capture.md) | Capture Search and Applications drawer. | macOS |
| T217 | [217-t-22.0-animation-hud-capture.md](217-t-22.0-animation-hud-capture.md) | Capture animations, HUDs, and transient feedback. | macOS |
| T218 | [218-t-22.0-motion-measurement.md](218-t-22.0-motion-measurement.md) | Fit durations/curves/overshoot into a timing database. | macOS/Linux |
| T219 | [219-t-22.0-surface-schema.md](219-t-22.0-surface-schema.md) | Freeze the surface/state schema and acceptance thresholds. | Linux |
| T220 | [220-t-22.0-naming-licensing.md](220-t-22.0-naming-licensing.md) | Naming map, asset policy, third-party equivalence, license gate. | Linux |
| T221 | [221-t-22.0-distro-contract.md](221-t-22.0-distro-contract.md) | Cross-distro host-service contract and adapter traits. | Linux |
| T222 | [222-t-22.1-settings-map.md](222-t-22.1-settings-map.md) | Pane catalog + row-level business-logic map. | Linux |
| T223 | [223-t-22.1-settings-ia-search.md](223-t-22.1-settings-ia-search.md) | Settings IA, search, deep links, row anatomy. | Linux |
| T224 | [224-t-22.1-settings-live-auth-absence.md](224-t-22.1-settings-live-auth-absence.md) | Live apply, validation, polkit/PAM, absence paths. | Linux |
| T225 | [225-t-22.1-network-panes.md](225-t-22.1-network-panes.md) | Network/Wi-Fi/VPN/firewall panes. | Linux |
| T226 | [226-t-22.1-general-panes.md](226-t-22.1-general-panes.md) | General, Date & Time, Language, Sharing, Backup, Login Items, Updates. | Linux |
| T227 | [227-t-22.1-quick-settings-search-displays.md](227-t-22.1-quick-settings-search-displays.md) | Appearance, Quick Settings, Search, Screen Saver, Desktop & Stage, Displays advanced. | Linux |
| T228 | [228-t-22.1-privacy-biometrics-accounts.md](228-t-22.1-privacy-biometrics-accounts.md) | Privacy & Security, Biometrics & Password, Online Accounts. | Linux |
| T229 | [229-t-22.1-input-users-printers.md](229-t-22.1-input-users-printers.md) | Sound, Keyboard/Input, Mouse/Trackpad, Controllers, Users/Groups, Printers, Battery/Storage. | Linux |
| T229a | [229a-t-22.1-wallpaper-notifications-focus-lock.md](229a-t-22.1-wallpaper-notifications-focus-lock.md) | Wallpaper, Notifications, Focus, Lock Screen panes. | Linux |
| T230 | [230-t-22.2-menubar-logic.md](230-t-22.2-menubar-logic.md) | Menu bar business logic and global menu completion. | Linux |
| T231 | [231-t-22.2-extras-framework-clock.md](231-t-22.2-extras-framework-clock.md) | Menu-bar extras framework and Date & Time/Calendar. | Linux |
| T232 | [232-t-22.2-extras-parity.md](232-t-22.2-extras-parity.md) | Wi-Fi, Bluetooth, Battery, Sound, Search, Quick Settings extras. | Linux |
| T233 | [233-t-22.2-menubar-visibility.md](233-t-22.2-menubar-visibility.md) | Auto-hide, notch/overflow, multi-display. | Linux |
| T234 | [234-t-22.2-notifications-focus-widgets.md](234-t-22.2-notifications-focus-widgets.md) | Notifications, Focus, widgets, banners. | Linux |
| T235 | [235-t-22.2-quick-settings-customization.md](235-t-22.2-quick-settings-customization.md) | Quick Settings customization and expanded modules. | Linux |
| T236 | [236-t-22.3-dock-behavior.md](236-t-22.3-dock-behavior.md) | Dock click tree, launch/quit, recent apps, failure paths. | Linux |
| T237 | [237-t-22.3-dock-motion.md](237-t-22.3-dock-motion.md) | Launch/bounce, minimize/genie, auto-hide, multi-display motion. | Linux |
| T238 | [238-t-22.3-dock-visual.md](238-t-22.3-dock-visual.md) | Stacks, Trash, badges, hover, magnification (glass check first). | Linux |
| T239 | [239-t-22.3-dock-drag-keyboard-a11y.md](239-t-22.3-dock-drag-keyboard-a11y.md) | Drag/drop, keyboard, accessibility, app management. | Linux |
| T240 | [240-t-22.4-overview-workspaces.md](240-t-22.4-overview-workspaces.md) | Overview/workspaces paging, strip, drag, gestures, hot corners. | Linux |
| T241 | [241-t-22.4-app-switcher.md](241-t-22.4-app-switcher.md) | App/window modes, cycling, previews, cancel, Cmd+`. | Linux |
| T242 | [242-t-22.4-stage-tiling.md](242-t-22.4-stage-tiling.md) | Stage and window tiling/split view. | Linux |
| T243 | [243-t-22.4-window-lifecycle.md](243-t-22.4-window-lifecycle.md) | Open/close/minimize/restore/zoom/fullscreen, launch/quit/Force Quit. | Linux |
| T244 | [244-t-22.4-preview.md](244-t-22.4-preview.md) | Preview (Quick Look) panel and actions. | Linux |
| T245 | [245-t-22.4-login-lock-first-run.md](245-t-22.4-login-lock-first-run.md) | Login, lock, fast user switching, Welcome. | Linux |
| T245a | [245a-t-22.4-system-dialogs-sheets.md](245a-t-22.4-system-dialogs-sheets.md) | File/print/color/emoji dialogs, alerts, sheets, screen saver. | Linux |
| T245b | [245b-t-22.4-desktop-items.md](245b-t-22.4-desktop-items.md) | Desktop icons, selection, context menu, stacks, volumes, widgets. | Linux |
| T246 | [246-t-22.5-search-service.md](246-t-22.5-search-service.md) | Search service: app-index, file index, actions, previews. | Linux |
| T247 | [247-t-22.5-search-ui.md](247-t-22.5-search-ui.md) | Search UI/UX parity. | Linux |
| T248 | [248-t-22.5-apps-drawer.md](248-t-22.5-apps-drawer.md) | Applications drawer UX parity. | Linux |
| T249 | [249-t-22.6-hud-osd.md](249-t-22.6-hud-osd.md) | HUD/OSD system for volume, brightness, backlight, charging, etc. | Linux |
| T250 | [250-t-22.6-sound.md](250-t-22.6-sound.md) | UI sound design and feedback. | Linux |
| T251 | [251-t-22.6-cursor.md](251-t-22.6-cursor.md) | Cursor/pointer aesthetics and HiDPI. | Linux |
| T252 | [252-t-22.6-accessibility.md](252-t-22.6-accessibility.md) | Shell AT-SPI, Orca, magnifier, contrast/reduced motion. | Linux |
| T253 | [253-t-22.6-ime.md](253-t-22.6-ime.md) | ibus/fcitx5 integration. | Linux |
| T254 | [254-t-22.7-app-discovery.md](254-t-22.7-app-discovery.md) | App discovery and install locations across distros. | Linux |
| T255 | [255-t-22.7-startup-apps.md](255-t-22.7-startup-apps.md) | XDG autostart, systemd user units, login items. | Linux |
| T256 | [256-t-22.7-users-auth.md](256-t-22.7-users-auth.md) | Users/groups, admin groups, polkit/PAM. | Linux |
| T257 | [257-t-22.7-updates.md](257-t-22.7-updates.md) | Updates/package management adapter. | Linux |
| T258 | [258-t-22.7-defaults-portals.md](258-t-22.7-defaults-portals.md) | Default apps, MIME handlers, portals. | Linux |
| T259 | [259-t-22.7-distro-matrix.md](259-t-22.7-distro-matrix.md) | Fedora/Ubuntu/Debian/Arch validation and packaging. | Linux/VM |
| T260 | [260-t-22.8-acceptance-harness.md](260-t-22.8-acceptance-harness.md) | Motion/pixel acceptance harness and CI. | Linux |
| T261 | [261-t-22.8-performance.md](261-t-22.8-performance.md) | Performance budgets for new surfaces. | Linux |
| T262 | [262-t-22.8-closeout.md](262-t-22.8-closeout.md) | Regression matrix, docs reconciliation, release close-out. | Linux |

## Gates

1. **Evidence gate.** T209–T219 produce the inventory, manifest, timing
   database, and acceptance thresholds before implementation tasks tune values.
   Missing evidence is an explicit `OPEN` row, never an invented value.
2. **Naming/licensing gate.** T220 lands the naming map, asset policy, and
   third-party table before new user-facing strings/assets.
3. **Distro gate.** T221 lands the host-service/adapter contract before
   Settings panes that depend on distro administration.
4. **Settings map gate.** T222 lands before any new Settings row. No row
   without an owner and an absence path.
5. **No dead affordance gate.** Every click/key/gesture either works or is
   disabled with a reason. A dead control is a task failure.
6. **Reduced motion/a11y gate.** Every animation has a reduced-motion path;
   every surface has a keyboard and AT-SPI path.
7. **Glass prerequisite check.** Any task touching Dock, menu bar, menus,
   Quick Settings, Notifications, Settings/Files chrome, or OSD must first
   check the liquid-glass task set (`docs/tasks/glass/`, T194–T208) and the
   current material path. Implement only the missing glass work; do not
   duplicate or regress it.
8. **Performance gate.** T261 shows the new surfaces within budget; T-21's
   presentation path is not regressed.
9. **Distro gate.** T259 passes install/smoke on Fedora, Ubuntu, Debian, and
   Arch before close-out.

## Evidence rules

- Record the build, display, scale, refresh, appearance, transparency/contrast,
  reduced motion, locale, focus, and power state for every capture.
- Stills measure material/geometry; motion measures timing/curves. Never infer
  a duration from two stills.
- Report p50/p95/p99 and missed deadlines, not only averages.
- Nested captures are reference evidence; native DRM evidence is separate.
- A checked task is agent-done, not human-signed-off.
- If a capability is unavailable, write an `OPEN` row naming the missing
  hardware/seat/capture and the exact rerun command.

## Hosts

- **T209–T218** are capture/measurement tasks. They run on the macOS capture
  host (reference repo `macos-liquid-glass-review`) and commit artifacts there.
  The Dragonfruit task records the artifact paths and measurements.
- **T219–T262** are implementation/validation tasks. They run on Linux in the
  Dragonfruit repo. If a capture artifact is missing, use the closest existing
  evidence and mark the gap `OPEN`; do not invent values.
- If the `macos-liquid-glass-review` checkout is not present on the Linux
  workstation, use the in-repo [`docs/reference/desktop-experience-inventory.md`](../../reference/desktop-experience-inventory.md)
  and [`docs/design/settings-business-logic-map.md`](../../design/settings-business-logic-map.md);
  do not block a task on a sibling checkout.

## Commands

```bash
make gates                       # full gate: e2e + qml-test
make e2e                         # required global verify
make qml-test                    # shell/QML tests
cargo test -p dragonfruit-compositor
make check                       # lint + test + soak
make native-rebuild              # release session install/relogin
make native-capture              # native diagnostics snapshot
```

New measurement targets added by this set must be documented in the task that
adds them and must not be wired into `make e2e` if they need a host session or
hardware.

## Harness

This directory is the task-file source for the dedicated `0.2.0` symphony task
set. The set's roadmap, progress notes, and run logs live in
[`docs/v0.2.0/`](../../v0.2.0/ROADMAP.md); the task files stay here.

```bash
./.symphony/symphony doctor --set 0.2.0
./.symphony/symphony lint --set 0.2.0
./.symphony/symphony status --set 0.2.0
./.symphony/symphony run --set 0.2.0
```

The harness uses `make gates` as the independent verify command. Hardware-only
or macOS-only halves must be committed as explicit `OPEN` evidence, never
accepted silently.
