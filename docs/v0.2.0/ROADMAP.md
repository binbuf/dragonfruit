# v0.2.0 Roadmap — Total desktop experience parity

This is the executable plan for the v0.2.0 release. It is a dedicated
symphony task set so the release can run independently of the completed
T-01…T-21 and liquid-glass work in [`docs/ROADMAP.md`](../ROADMAP.md) and
[`docs/tasks/glass/`](../tasks/glass/README.md).

- Run with `./.symphony/symphony run --set 0.2.0`.
- Check readiness with `./.symphony/symphony doctor --set 0.2.0`.
- Inspect progress with `./.symphony/symphony status --set 0.2.0`.

The release is deliberately broad but ordered: **capture the reference,
map every setting to an owner, then close the visible experience gaps.** The
material/presentation work from T-20/T-21 and the liquid-glass set stays; this
set adds behaviour, information architecture, motion, HUDs, and cross-distro
integration.

## Gates and evidence rules

- **Evidence gate** — T209–T219 must land the inventory, manifest, timing
database, and acceptance thresholds before implementation tasks tune values.
  Missing evidence is an explicit `OPEN` row, never an invented value.
- **Naming/licensing gate** — T220 lands the naming map, asset policy, and
third-party table before new user-facing strings/assets.
- **Distro gate** — T221 lands the host-service/adapter contract before
Settings panes that depend on distro administration.
- **Settings map gate** — T222 lands before any new Settings row. No row
without an owner and an absence path.
- **No dead affordance gate** — every click/key/gesture either works or is
disabled with a reason. A dead control is a task failure.
- **Reduced motion/a11y gate** — every animation has a reduced-motion path;
every surface has a keyboard and AT-SPI path.
- **Glass prerequisite check** — any task touching Dock, menu bar, menus,
Quick Settings, Notifications, Settings/Files chrome, or OSD must first check
the liquid-glass set (`docs/tasks/glass/`, T194–T208) and the current material
path. Implement only missing glass work; do not duplicate or regress it.
- **Performance gate** — T261 shows the new surfaces within budget; T-21's
presentation path is not regressed.
- **Distro gate** — T259 passes install/smoke on Fedora, Ubuntu, Debian, and
Arch before close-out.

Every task reports the capture/measurement artifact and the exact command
that produced it. Nested captures are reference evidence; native DRM evidence
is separate. A checked task is agent-done, not human-signed-off. If hardware is
unavailable, write an explicit `OPEN` artifact naming the missing
seat/GPU/panel and the exact command to rerun.

## Phase 22.0 — Evidence, naming, and contracts

- [ ] T209 — Total-experience inventory and capture manifest → [209-t-22.0-experience-inventory.md](../tasks/0.2.0/209-t-22.0-experience-inventory.md)
- [ ] T210 — Capture harness: animation, HUD, and accessibility evidence → [210-t-22.0-capture-harness.md](../tasks/0.2.0/210-t-22.0-capture-harness.md)
- [ ] T211 — Capture: System Settings pane sweep → [211-t-22.0-settings-capture.md](../tasks/0.2.0/211-t-22.0-settings-capture.md)
- [ ] T212 — Capture: menu bar and menu extras → [212-t-22.0-menubar-capture.md](../tasks/0.2.0/212-t-22.0-menubar-capture.md)
- [ ] T213 — Capture: Dock behavior matrix → [213-t-22.0-dock-capture.md](../tasks/0.2.0/213-t-22.0-dock-capture.md)
- [ ] T214 — Capture: Mission Control and workspaces → [214-t-22.0-overview-capture.md](../tasks/0.2.0/214-t-22.0-overview-capture.md)
- [ ] T215 — Capture: app switcher and app lifecycle → [215-t-22.0-switcher-lifecycle-capture.md](../tasks/0.2.0/215-t-22.0-switcher-lifecycle-capture.md)
- [ ] T216 — Capture: Search and Applications drawer → [216-t-22.0-search-drawer-capture.md](../tasks/0.2.0/216-t-22.0-search-drawer-capture.md)
- [ ] T217 — Capture: animations and HUDs → [217-t-22.0-animation-hud-capture.md](../tasks/0.2.0/217-t-22.0-animation-hud-capture.md)
- [ ] T218 — Motion measurement and timing database → [218-t-22.0-motion-measurement.md](../tasks/0.2.0/218-t-22.0-motion-measurement.md)
- [ ] T219 — Surface/state schema and acceptance thresholds → [219-t-22.0-surface-schema.md](../tasks/0.2.0/219-t-22.0-surface-schema.md)
- [ ] T220 — Naming, licensing, and third-party equivalence → [220-t-22.0-naming-licensing.md](../tasks/0.2.0/220-t-22.0-naming-licensing.md)
- [ ] T221 — Cross-distro compatibility contract → [221-t-22.0-distro-contract.md](../tasks/0.2.0/221-t-22.0-distro-contract.md)

## Phase 22.1 — Settings

- [ ] T222 — Settings business-logic map and pane catalog → [222-t-22.1-settings-map.md](../tasks/0.2.0/222-t-22.1-settings-map.md)
- [ ] T223 — Settings IA, search, and deep-link polish → [223-t-22.1-settings-ia-search.md](../tasks/0.2.0/223-t-22.1-settings-ia-search.md)
- [ ] T224 — Settings live apply, validation, admin auth, and absence → [224-t-22.1-settings-live-auth-absence.md](../tasks/0.2.0/224-t-22.1-settings-live-auth-absence.md)
- [ ] T225 — Settings panes: Network, Wi-Fi, VPN, and firewall → [225-t-22.1-network-panes.md](../tasks/0.2.0/225-t-22.1-network-panes.md)
- [ ] T226 — Settings panes: General, Date & Time, Language, Sharing, Backup, Login Items, Updates → [226-t-22.1-general-panes.md](../tasks/0.2.0/226-t-22.1-general-panes.md)
- [ ] T227 — Settings panes: Appearance, Quick Settings, Search, Screen Saver, Desktop & Stage, Displays advanced → [227-t-22.1-quick-settings-search-displays.md](../tasks/0.2.0/227-t-22.1-quick-settings-search-displays.md)
- [ ] T228 — Settings panes: Privacy & Security, Biometrics & Password, Online Accounts → [228-t-22.1-privacy-biometrics-accounts.md](../tasks/0.2.0/228-t-22.1-privacy-biometrics-accounts.md)
- [ ] T229 — Settings panes: Sound, Keyboard/Input, Mouse/Trackpad, Controllers, Users/Groups, Printers, Battery/Storage → [229-t-22.1-input-users-printers.md](../tasks/0.2.0/229-t-22.1-input-users-printers.md)
- [ ] T229a — Settings panes: Wallpaper, Notifications, Focus, Lock Screen → [229a-t-22.1-wallpaper-notifications-focus-lock.md](../tasks/0.2.0/229a-t-22.1-wallpaper-notifications-focus-lock.md)

## Phase 22.2 — Menu bar and system panels

- [ ] T230 — Menu bar business logic and global menu completion → [230-t-22.2-menubar-logic.md](../tasks/0.2.0/230-t-22.2-menubar-logic.md)
- [ ] T231 — Menu-bar extras framework and Date & Time/Calendar → [231-t-22.2-extras-framework-clock.md](../tasks/0.2.0/231-t-22.2-extras-framework-clock.md)
- [ ] T232 — Wi-Fi, Bluetooth, Battery, Sound, Search, and Quick Settings extras → [232-t-22.2-extras-parity.md](../tasks/0.2.0/232-t-22.2-extras-parity.md)
- [ ] T233 — Menu bar visibility, auto-hide, notch/overflow, and multi-display → [233-t-22.2-menubar-visibility.md](../tasks/0.2.0/233-t-22.2-menubar-visibility.md)
- [ ] T234 — Notifications, Focus, widgets, and banners → [234-t-22.2-notifications-focus-widgets.md](../tasks/0.2.0/234-t-22.2-notifications-focus-widgets.md)
- [ ] T235 — Quick Settings customization and expanded modules → [235-t-22.2-quick-settings-customization.md](../tasks/0.2.0/235-t-22.2-quick-settings-customization.md)

## Phase 22.3 — Dock

- [ ] T236 — Dock behavior completion and app lifecycle → [236-t-22.3-dock-behavior.md](../tasks/0.2.0/236-t-22.3-dock-behavior.md)
- [ ] T237 — Dock motion: launch, bounce, minimize/genie, auto-hide, multi-display → [237-t-22.3-dock-motion.md](../tasks/0.2.0/237-t-22.3-dock-motion.md)
- [ ] T238 — Dock visual/state parity: stacks, Trash, badges, hover, magnification → [238-t-22.3-dock-visual.md](../tasks/0.2.0/238-t-22.3-dock-visual.md)
- [ ] T239 — Dock drag/drop, keyboard, accessibility, and app management → [239-t-22.3-dock-drag-keyboard-a11y.md](../tasks/0.2.0/239-t-22.3-dock-drag-keyboard-a11y.md)

## Phase 22.4 — Window management and navigation

- [ ] T240 — Overview/workspaces UX completion → [240-t-22.4-overview-workspaces.md](../tasks/0.2.0/240-t-22.4-overview-workspaces.md)
- [ ] T241 — App switcher UX completion → [241-t-22.4-app-switcher.md](../tasks/0.2.0/241-t-22.4-app-switcher.md)
- [ ] T242 — Stage and window tiling/split view → [242-t-22.4-stage-tiling.md](../tasks/0.2.0/242-t-22.4-stage-tiling.md)
- [ ] T243 — Window lifecycle motion and app launch/quit/Force Quit → [243-t-22.4-window-lifecycle.md](../tasks/0.2.0/243-t-22.4-window-lifecycle.md)
- [ ] T244 — Preview (Quick Look) → [244-t-22.4-preview.md](../tasks/0.2.0/244-t-22.4-preview.md)
- [ ] T245 — Login, lock, and first-run experience → [245-t-22.4-login-lock-first-run.md](../tasks/0.2.0/245-t-22.4-login-lock-first-run.md)
- [ ] T245a — System dialogs, sheets, alerts, and pickers → [245a-t-22.4-system-dialogs-sheets.md](../tasks/0.2.0/245a-t-22.4-system-dialogs-sheets.md)
- [ ] T245b — Desktop items and interaction → [245b-t-22.4-desktop-items.md](../tasks/0.2.0/245b-t-22.4-desktop-items.md)

## Phase 22.5 — Search and launcher

- [ ] T246 — Search service: app-index, file index, actions, and previews → [246-t-22.5-search-service.md](../tasks/0.2.0/246-t-22.5-search-service.md)
- [ ] T247 — Search UI/UX parity → [247-t-22.5-search-ui.md](../tasks/0.2.0/247-t-22.5-search-ui.md)
- [ ] T248 — Applications drawer UX parity → [248-t-22.5-apps-drawer.md](../tasks/0.2.0/248-t-22.5-apps-drawer.md)

## Phase 22.6 — Feedback, HUDs, accessibility

- [ ] T249 — HUD/OSD system → [249-t-22.6-hud-osd.md](../tasks/0.2.0/249-t-22.6-hud-osd.md)
- [ ] T250 — UI sound design and feedback → [250-t-22.6-sound.md](../tasks/0.2.0/250-t-22.6-sound.md)
- [ ] T251 — Cursor and pointer aesthetics → [251-t-22.6-cursor.md](../tasks/0.2.0/251-t-22.6-cursor.md)
- [ ] T252 — Accessibility integration and shell AT-SPI → [252-t-22.6-accessibility.md](../tasks/0.2.0/252-t-22.6-accessibility.md)
- [ ] T253 — IME integration → [253-t-22.6-ime.md](../tasks/0.2.0/253-t-22.6-ime.md)

## Phase 22.7 — Distro integration

- [ ] T254 — App discovery and install locations across distros → [254-t-22.7-app-discovery.md](../tasks/0.2.0/254-t-22.7-app-discovery.md)
- [ ] T255 — Startup apps and login items → [255-t-22.7-startup-apps.md](../tasks/0.2.0/255-t-22.7-startup-apps.md)
- [ ] T256 — Users/groups, admin auth, and polkit → [256-t-22.7-users-auth.md](../tasks/0.2.0/256-t-22.7-users-auth.md)
- [ ] T257 — Software updates and package management → [257-t-22.7-updates.md](../tasks/0.2.0/257-t-22.7-updates.md)
- [ ] T258 — Default apps, MIME handlers, and portals → [258-t-22.7-defaults-portals.md](../tasks/0.2.0/258-t-22.7-defaults-portals.md)
- [ ] T259 — Cross-distro validation matrix and packaging → [259-t-22.7-distro-matrix.md](../tasks/0.2.0/259-t-22.7-distro-matrix.md)

## Phase 22.8 — Acceptance and close-out

- [ ] T260 — Motion/pixel acceptance harness and CI → [260-t-22.8-acceptance-harness.md](../tasks/0.2.0/260-t-22.8-acceptance-harness.md)
- [ ] T261 — Performance budgets for new surfaces → [261-t-22.8-performance.md](../tasks/0.2.0/261-t-22.8-performance.md)
- [ ] T262 — Total-experience regression matrix and release close-out → [262-t-22.8-closeout.md](../tasks/0.2.0/262-t-22.8-closeout.md)

## Release definition of done

A unit is agent-done when `make gates` passes, the task's capture/evidence
artifact exists, any hardware-dependent half is explicitly `OPEN`, docs and
design references are updated, and the task's `## Hand-off` is filled in.
Human sign-off remains batched at the release boundary and is not part of an
agent task.

<!-- symphony:status -->
**Pipeline status** — not started

- Completed: none
- Blocked: none
- Failed: none
- Remaining: T209–T262
- Last finished: —
<!-- /symphony:status -->