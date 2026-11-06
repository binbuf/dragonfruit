# Track 22 — Total desktop experience parity (v0.2.0)

| | |
|---|---|
| **Status** | proposed |
| **Date** | 2026-11-06 |
| **Release** | v0.2.0 |
| **Scope** | every core shell surface, interaction, animation, HUD, menu-bar extra, and System Settings pane; cross-distro integration; evidence/acceptance |
| **Related** | [22-total-experience.md](../22-total-experience.md), [settings map](../settings-business-logic-map.md), [distro contract](../distro-compatibility.md), [naming/licensing](../interface-naming-and-licensing.md), [motion/feedback](../motion-and-feedback.md), [evidence/acceptance](../evidence-and-acceptance.md) |

## 1. Why this track

T-20/T-21 and the liquid-glass set fixed the **material** and the
**presentation path**. The product gap is now the **experience**: the Dock's
click tree, the menu bar's global menu and extras, Overview/workspaces, the app
switcher, Search, the Applications drawer, System Settings, animations, HUDs,
and the Linux ownership of every setting.

A desktop is judged as a whole. Adding another settings pane without a real
owner, or another animation without a measured curve, widens the surface
without improving the product. This track closes the visible dead ends and
completes the core experience before adding breadth.

## 2. Shape

```text
T-22.0  evidence → naming/licensing → distro contract → settings map
T-22.1  Settings IA → live apply/auth → pane groups
T-22.2  menu bar → extras → visibility/overflow → notifications/Quick Settings
T-22.3  Dock behaviour → motion → visual/state → drag/keyboard/a11y
T-22.4  Overview/workspaces → app switcher → Stage/tiling → window lifecycle → Quick Look → login
T-22.5  Search service → Search UI → Applications drawer
T-22.6  HUDs → sound → cursor → accessibility → IME
T-22.7  app discovery → startup → users/groups → updates → defaults → distro matrix
T-22.8  motion/pixel harness → performance → regression/close-out
```

Every phase ends with a capture or measurement artifact and a hand-off. The
human sign-off is batched at the phase boundary, not per task.

## 3. Tasks

The executable task files live in [`docs/tasks/0.2.0/`](../../tasks/0.2.0/README.md).

| Task | Unit | Depends on | Host |
|---|---|---|---|
| T209 | Total-experience inventory and capture manifest | — | macOS |
| T210 | Capture harness: animation, HUD, and accessibility evidence | T209 | macOS |
| T211 | Capture: System Settings pane sweep | T209, T210 | macOS |
| T212 | Capture: menu bar and menu extras | T209, T210 | macOS |
| T213 | Capture: Dock behavior matrix | T209, T210 | macOS |
| T214 | Capture: Mission Control and workspaces | T209, T210 | macOS |
| T215 | Capture: app switcher and app lifecycle | T209, T210 | macOS |
| T216 | Capture: Search and Applications drawer | T209, T210 | macOS |
| T217 | Capture: animations and HUDs | T209, T210 | macOS |
| T218 | Motion measurement and timing database | T217 | macOS/Linux |
| T219 | Surface/state schema and acceptance thresholds | T209–T218 | Linux |
| T220 | Naming, licensing, and third-party equivalence | T209 | Linux |
| T221 | Cross-distro compatibility contract | T209 | Linux |
| T222 | Settings business-logic map and pane catalog | T211, T220, T221 | Linux |
| T223 | Settings IA, search, and deep-link polish | T222 | Linux |
| T224 | Settings live apply, validation, admin auth, and absence | T222 | Linux |
| T225 | Settings panes: Network/Wi-Fi/VPN/firewall | T224 | Linux |
| T226 | Settings panes: General, Date & Time, Language, Sharing, Backup, Login Items, Updates | T224 | Linux |
| T227 | Settings panes: Appearance, Quick Settings, Search, Screen Saver, Desktop & Stage, Displays advanced | T224 | Linux |
| T228 | Settings panes: Privacy & Security, Biometrics, Online Accounts | T224 | Linux |
| T229 | Settings panes: Sound, Keyboard/Input, Mouse/Trackpad, Controllers, Users/Groups, Printers, Battery/Storage | T224 | Linux |
| T229a | Settings panes: Wallpaper, Notifications, Focus, Lock Screen | T224 | Linux |
| T230 | Menu bar business logic and global menu completion | T212, T220 | Linux |
| T231 | Menu bar extras framework and Date & Time/Calendar | T230 | Linux |
| T232 | Wi-Fi, Bluetooth, Battery, Sound, Search, Quick Settings extras | T231 | Linux |
| T233 | Menu bar visibility, auto-hide, notch/overflow, multi-display | T230 | Linux |
| T234 | Notifications, Focus, widgets, banners | T212 | Linux |
| T235 | Quick Settings customization and expanded modules | T212 | Linux |
| T236 | Dock behavior completion and app lifecycle | T213, T220 | Linux |
| T237 | Dock motion: launch, bounce, minimize/genie, auto-hide, multi-display | T213, T218 | Linux |
| T238 | Dock visual/state parity: stacks, Trash, badges, hover, magnify | T213, T218 | Linux |
| T239 | Dock drag/drop, keyboard, accessibility, app management | T213 | Linux |
| T240 | Overview/workspaces UX completion | T214, T218 | Linux |
| T241 | App switcher UX completion | T215, T218 | Linux |
| T242 | Stage and window tiling/split view | T214, T218 | Linux |
| T243 | Window lifecycle motion and app launch/quit/Force Quit | T215, T218 | Linux |
| T244 | Preview (Quick Look) | T215 | Linux |
| T245 | Login, lock, and first-run experience | T215 | Linux |
| T245a | System dialogs, sheets, alerts, and pickers | T245 | Linux |
| T245b | Desktop items and interaction | T245 | Linux |
| T246 | Search service: app-index, file index, actions, previews | T216 | Linux |
| T247 | Search UI/UX parity | T246 | Linux |
| T248 | Applications drawer UX parity | T216 | Linux |
| T249 | HUD/OSD system | T217, T218 | Linux |
| T250 | UI sound design and feedback | T217 | Linux |
| T251 | Cursor and pointer aesthetics | T217 | Linux |
| T252 | Accessibility integration and shell AT-SPI | T220, T229 | Linux |
| T253 | IME integration | T220, T229 | Linux |
| T254 | App discovery and install locations | T221 | Linux |
| T255 | Startup apps and login items | T221, T254 | Linux |
| T256 | Users/groups, admin auth, and polkit | T221, T229 | Linux |
| T257 | Software updates and package management | T221 | Linux |
| T258 | Default apps, MIME handlers, and portals | T221 | Linux |
| T259 | Cross-distro validation matrix and packaging | T254–T258 | Linux/VM |
| T260 | Motion/pixel acceptance harness and CI | T218, T219 | Linux |
| T261 | Performance budgets for new surfaces | T237, T240, T241, T249 | Linux |
| T262 | Total-experience regression matrix and release close-out | all | Linux |

## 4. Gates

1. **Evidence gate.** T209–T219 must produce the inventory, manifest, and
   acceptance thresholds before implementation tasks tune values. Missing
   evidence is `OPEN`, not invented.
2. **Naming/licensing gate.** T220 must land the naming map, asset policy, and
   third-party table before new user-facing strings/assets.
3. **Distro gate.** T221 must land the host-service/adapter contract before
   Settings panes that depend on distro administration.
4. **Settings map gate.** T222 must land before any new Settings row. No row
   without an owner and an absence path.
5. **No dead affordance gate.** Every click/key/gesture in a task either works
   or is disabled with a reason. A dead control is a task failure.
6. **Reduced motion/a11y gate.** Every animation has a reduced-motion path;
   every surface has a keyboard and AT-SPI path.
7. **Performance gate.** T261 must show the new surfaces within budget; the
   presentation path (T-21) is not regressed.
8. **Distro gate.** T259 must pass install/smoke on Fedora, Ubuntu, Debian, and
   Arch before close-out.

## 5. Acceptance

- [ ] Every `S-*`, `N-*`, `Q-*`, `M-*`, `H-*`, and `SET-*` row in the capture
      inventory has an artifact or an `OPEN` row.
- [ ] Every Settings row has an owner and an absence path.
- [ ] Every animation has a measured duration/curve and a reduced-motion path.
- [ ] Every HUD has a trigger, timeout, accessibility announcement, and
      per-display rule.
- [ ] Every surface has a keyboard and AT-SPI path.
- [ ] The Dock click tree, window chooser, stacks, Trash, drag/drop, keyboard,
      and multi-display behaviour are complete.
- [ ] The menu bar has a complete global menu, system menu, extras, clock,
      overflow, auto-hide, and multi-display behaviour.
- [ ] Overview/workspaces, app switcher, Search, Applications drawer, Stage,
      tiling, Preview, and login/lock meet the behaviour contracts.
- [ ] Quick Settings and Notifications are complete and live.
- [ ] The desktop installs and works on Fedora 44, Ubuntu 24.04, Debian 13, and
      Arch.
- [ ] `make gates` stays green; no hidden deferral.
- [ ] Human sign-off has watched the release demo.

## 6. Non-goals

- Apple account services (iCloud, Siri, Apple Intelligence, Screen Time,
  Wallet, Game Center, Find My, App Store).
- Pixel-identical macOS assets, names, sounds, or code.
- A new display server, renderer, package manager, network stack, audio stack,
  or privilege-escalation scheme.
- A second settings daemon, menu model, notification service, or window model.
- Full parity with every third-party app's custom chrome.
- Guaranteed AirDrop/Continuity/Handoff compatibility.
- Live compositor handoff.

## 7. Rollback

- Each surface change stays behind the existing feature/settings flag until
  accepted.
- The material rollback flags from T-21 remain (`DRAGONFRUIT_DRM_MATERIAL=0`,
  `DRAGONFRUIT_DRM_GLASS=0`).
- The old Dock/menu/Settings path remains selectable until the new path passes
  the track's nested/native acceptance.
- Distro adapters are additive; a missing adapter falls back to the previous
  pane behaviour with an absence note.
- The session can return to the previous desktop with `make native-return` /
  `make native-recover`.

## 8. Risks

| Risk | Mitigation |
|---|---|
| Scope explosion | Inventory is the contract; explicit omit/delegate; one-session tasks |
| Evidence gaps | `OPEN` rows; do not invent values |
| Settings fake toggles | T222 owner map + absence tests |
| Distro leakage into QML | Adapter trait; no shell commands in QML |
| Motion inconsistency | One token set/clock; measured curves |
| Accessibility deferred | Per-task a11y/reduced-motion done-when |
| Licensing contamination | ADR 0220; separate-process policy; packaging review |
| Performance regression | T-21 presentation path; per-surface budgets; degrade tiers |
| Human sign-off skipped | Batched at phase boundaries; release gate |

## 9. References

- [Total experience design](../22-total-experience.md)
- [Settings business-logic map](../settings-business-logic-map.md)
- [Cross-distro contract](../distro-compatibility.md)
- [Naming/licensing](../interface-naming-and-licensing.md)
- [Motion/feedback](../motion-and-feedback.md)
- [Evidence/acceptance](../evidence-and-acceptance.md)
- `macos-liquid-glass-review/docs/CAPTURE-PLAN.md`
- `macos-liquid-glass-review/docs/SURFACE-INVENTORY.md`
- [ADR 0219](../adr/0219-total-experience-scope-and-parity-bar.md)
- [ADR 0220](../adr/0220-generic-naming-and-third-party-substitution.md)
- [ADR 0221](../adr/0221-cross-distro-host-service-contract.md)
