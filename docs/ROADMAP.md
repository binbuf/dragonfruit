# Dragonfruit Roadmap — strict, single-session units

This is the current plan. It replaces the original phase-based ticket sequence,
which is archived in [legacy/](tasks/legacy/) with a status table and a mapping in
[§ Legacy mapping](#legacy-mapping). The design docs in
[../design/](design/) remain the source of truth for *what* to build; this
index is the source of truth for *the order and the definition of done*.

## Why the plan was re-cut

The legacy plan built a large amount of verified machinery — compositor core,
input, window model, Spaces, Xwayland, private protocols, design system, menu
bar, Dock — while deferring the user-facing affordance layer (SSD titlebars,
window lifecycle animations, materials) and the live chrome (status adapters,
settings, apps). The result was "done" tickets that could not be experienced
end to end, and a first honest product review that sat behind a twelve-ticket
dependency fan-in.

The remaining work is cut into **tracks** (the 17 named slices, kept as design
references) and executed as a strict, single-session **work-unit** sequence in
[tasks/](tasks/). Every unit ends with its headless tests green and a capture
artifact committed; human sign-off is batched at the track boundary. Each unit
extends the previous demo rather than starting a new subsystem. The underlying
design is unchanged; only the sequence, the size of a task, and the acceptance
bar are.

## The rule that makes it work: definition of done

The old rule ("a slice is done only when its demo is captured **and reviewed**")
required a human to watch every slice, so no slice of any size could be finished
in an agent session. It is split in two.

**A unit is agent-done when all of these hold:**

- [ ] **Merged and building** — `cargo build` and the CMake/QML build pass.
- [ ] **Scripted** — the compositor-observable parts are covered by a headless
      test that stays green in `make e2e`.
- [ ] **No regression** — `make check` (lint + test + soak) is unchanged, and
      the previous units' tests still pass.
- [ ] **Reduced motion / degrade** — every animation the unit adds has its
      reduced-motion variant; every budget-sensitive effect has a degrade path.
- [ ] **Capture artifact** — the unit's part of the demo is captured under
      `docs/captures/` and noted in `PROGRESS.md`.
- [ ] **No silently deferred half** — anything user-facing that is postponed is
      named in the unit's "Explicitly deferred" list.

**Human sign-off is batched at the track boundary:**

- watch the track's demo in a live nested session and compare it against the
  design docs and the design-system gallery goldens;
- tick the track's acceptance boxes;
- the T-17 unfamiliar-user test.

A green test suite is necessary but **not** sufficient. The canonical failure
this rule exists to prevent: a Dock with forty passing tests that nobody can
use because windows have no titlebar — which is why sign-off still happens, just
at the track boundary rather than per unit.

## The tracks

The 17 named slices are now **tracks**: design references, not single sessions.
Every track is decomposed into one-session **work units** in [tasks/](tasks/),
listed in strict execution order below. A unit ends with its headless test green
in `make e2e`, `make check`/`make soak` unchanged, and a capture artifact
committed. Human sign-off (watch the demo, compare to the design docs and the
gallery goldens) is **batched at the track boundary**, not part of a unit — see
[SLICING-REVIEW.md](SLICING-REVIEW.md) for why the old definition of done made
every task un-completable.

| Track | Design reference | Units | Demo you can watch |
|---|---|---|---|
| [T-01](design/tracks/01-loop-v0-window-controls.md) | Loop v0 — window controls | 7 | launch → titled window → drag/zoom/minimize/restore/close |
| [T-02](design/tracks/02-loop-v1-lifecycle-motion.md) | Loop v1 — lifecycle motion | 6 | the loop with appear/minimize/restore/zoom/close motion |
| [T-03](design/tracks/03-real-session-bringup-perf.md) | Real session — bring-up & perf | 5 | the loop on DRM/logind, budgets measured [hw] |
| [T-04](design/tracks/04-loop-v2-materials.md) | Loop v2 — materials | 6 | blur, shadows, rounded corners |
| [T-05](design/tracks/05-loop-v3-mission-control-live.md) | Loop v3 — Mission Control live | 7 | live-surface overview, wallpaper slide, Desktop Reveal |
| [T-06](design/tracks/06-loop-v4-app-switcher.md) | Loop v4 — app switcher | 3 | Cmd+Tab with live previews |
| [T-07](design/tracks/07-menu-bar-live-status.md) | Menu bar goes live | 10 | real Wi-Fi, volume, battery |
| [T-08](design/tracks/08-settingsd-live-settings.md) | settingsd — one owner | 6 | one owner for settings, live signals |
| [T-09](design/tracks/09-settings-app-wave-1.md) | Settings Wave 1 | 8 | Appearance, Wallpaper, Desktop & Dock, Displays-basic |
| [T-10](design/tracks/10-files-mvp.md) | Files MVP | 14 | browse, open, rename, trash; Dock↔Files Trash |
| [T-11](design/tracks/11-control-center-notifications.md) | Control Center + notifications | 8 | panel, banners, OSD |
| [T-12](design/tracks/12-session-lock-idle.md) | Session + lock + idle | 10 | real login session, lock, idle, suspend [hw] |
| [T-13](design/tracks/13-portals-capture-clipboard.md) | Portals + capture + clipboard | 12 | Flatpak browser walkthrough |
| [T-14](design/tracks/14-global-menu-app-index-compat.md) | Global menu + app index + compat | 11 | real menus, tray, DBusMenu, XDnD, zoo |
| [T-15](design/tracks/15-system-services-breadth.md) | System services + Settings Waves 2–3 | 31 | Bluetooth, storage, printers, users, … |
| [T-16](design/tracks/16-platform-polish-packaging.md) | Platform polish + packaging | 15 | multi-monitor, scaling, soak, a11y, i18n, packages [hw] |
| [T-17](design/tracks/17-premium-gate.md) | The premium experience gate | 9 | the full loop on nested + DRM, at the visual floor |

## Work units (strict order)

The 168 one-session tasks below are the executable sequence. symphony walks them in
file order, runs each in a fresh session, verifies with `make e2e` and commits.
The 10 `[hw]` tasks (DRM/logind, driver matrix, suspend soak, clean-VM packaging)
are collected into the final Hardware rail phase so the nested pipeline runs to
completion unattended; sweep them on a machine with a seat or a clean VM with
`./.symphony/symphony run --from T159`.

## Phase 1 — T-01 Loop v0: window controls

- [x] T01 — T-01.1 Titlebar render element → [tasks/001-t-01.1-titlebar-render-element.md](tasks/001-t-01.1-titlebar-render-element.md)
- [x] T02 — T-01.2 Traffic-light actions → [tasks/002-t-01.2-traffic-light-actions.md](tasks/002-t-01.2-traffic-light-actions.md)
- [x] T03 — T-01.3 Titlebar drag, double-click, fullscreen reveal → [tasks/003-t-01.3-titlebar-drag-double-click-fullscreen-reveal.md](tasks/003-t-01.3-titlebar-drag-double-click-fullscreen-reveal.md)
- [x] T04 — T-01.4 Window menu → [tasks/004-t-01.4-window-menu.md](tasks/004-t-01.4-window-menu.md)
- [x] T05 — T-01.5 Decoration tier policy and X11 correctness → [tasks/005-t-01.5-decoration-tier-policy-and-x11-correctness.md](tasks/005-t-01.5-decoration-tier-policy-and-x11-correctness.md)
- [x] T06 — T-01.6a `make demo` harness → [tasks/006-t-01.6a-make-demo-harness.md](tasks/006-t-01.6a-make-demo-harness.md)
- [x] T07 — T-01.6b Loop integration walkthrough and capture → [tasks/007-t-01.6b-loop-integration-capture.md](tasks/007-t-01.6b-loop-integration-capture.md)

## Phase 2 — T-02 Loop v1: lifecycle motion

- [x] T08 — T-02.1a Animation clock and frame discipline → [tasks/008-t-02.1a-animation-clock.md](tasks/008-t-02.1a-animation-clock.md)
- [x] T09 — T-02.1b Window appear transition → [tasks/009-t-02.1b-window-appear.md](tasks/009-t-02.1b-window-appear.md) ⟵ accepted
- [x] T10 — T-02.2 Minimize and restore motion → [tasks/010-t-02.2-minimize-and-restore-motion.md](tasks/010-t-02.2-minimize-and-restore-motion.md)
- [x] T11 — T-02.3 Zoom and fullscreen transitions → [tasks/011-t-02.3-zoom-and-fullscreen-transitions.md](tasks/011-t-02.3-zoom-and-fullscreen-transitions.md)
- [x] T12 — T-02.4a Close ghost → [tasks/012-t-02.4a-close-ghost.md](tasks/012-t-02.4a-close-ghost.md)
- [x] T13 — T-02.4b Close interruptibility and idle trace → [tasks/013-t-02.4b-close-interruptibility-and-idle-trace.md](tasks/013-t-02.4b-close-interruptibility-and-idle-trace.md)

## Phase 3 — T-03 Real session: bring-up and perf

- [x] T14 — T-03.1a Nested idle trace and animation frame budget → [tasks/014-t-03.1a-nested-idle-and-animation-budget.md](tasks/014-t-03.1a-nested-idle-and-animation-budget.md)
- [x] T15 — T-03.1b Latency instrument and direct-scanout template → [tasks/015-t-03.1b-latency-and-scanout-instruments.md](tasks/015-t-03.1b-latency-and-scanout-instruments.md)

## Phase 4 — T-04 Loop v2: materials

- [x] T16 — T-04.1a Real shadows → [tasks/016-t-04.1a-shadows.md](tasks/016-t-04.1a-shadows.md)
- [x] T17 — T-04.1b Rounded-corner clipping → [tasks/017-t-04.1b-rounded-corner-clipping.md](tasks/017-t-04.1b-rounded-corner-clipping.md)
- [x] T18 — T-04.2 Backdrop blur pass → [tasks/018-t-04.2-backdrop-blur-pass.md](tasks/018-t-04.2-backdrop-blur-pass.md)
- [x] T19 — T-04.3 Reusable scene-transform pass → [tasks/019-t-04.3-reusable-scene-transform-pass.md](tasks/019-t-04.3-reusable-scene-transform-pass.md)
- [x] T20 — T-04.4a Material degrade tiers and instrumentation → [tasks/020-t-04.4a-material-degrade-tiers.md](tasks/020-t-04.4a-material-degrade-tiers.md)
- [x] T21 — T-04.4b Light/dark, reduced motion, and sign-off package → [tasks/021-t-04.4b-material-schemes-and-sign-off-package.md](tasks/021-t-04.4b-material-schemes-and-sign-off-package.md)

## Phase 5 — T-05 Loop v3: Mission Control live

- [x] T22 — T-05.1a Live-surface transform into the grid → [tasks/022-t-05.1a-live-surface-transform.md](tasks/022-t-05.1a-live-surface-transform.md)
- [x] T23 — T-05.1b Live video at scale and degrade → [tasks/023-t-05.1b-live-video-and-degrade.md](tasks/023-t-05.1b-live-video-and-degrade.md)
- [x] T24 — T-05.2 Hit-testing and selection on live representations → [tasks/024-t-05.2-hit-testing-and-selection-on-live-representations.md](tasks/024-t-05.2-hit-testing-and-selection-on-live-representations.md)
- [x] T25 — T-05.3 Drag a live representation between Spaces → [tasks/025-t-05.3-drag-a-live-representation-between-spaces.md](tasks/025-t-05.3-drag-a-live-representation-between-spaces.md)
- [x] T26 — T-05.4 Image wallpaper and per-Space slide → [tasks/026-t-05.4-image-wallpaper-and-per-space-slide.md](tasks/026-t-05.4-image-wallpaper-and-per-space-slide.md)
- [x] T27 — T-05.5 Desktop Reveal → [tasks/027-t-05.5-desktop-reveal.md](tasks/027-t-05.5-desktop-reveal.md)
- [x] T28 — T-05.6 Overview frame budget and capture → [tasks/028-t-05.6-overview-frame-budget-and-capture.md](tasks/028-t-05.6-overview-frame-budget-and-capture.md)

## Phase 6 — T-06 Loop v4: app switcher

- [x] T29 — T-06.1 App-switcher state machine → [tasks/029-t-06.1-app-switcher-state-machine.md](tasks/029-t-06.1-app-switcher-state-machine.md)
- [x] T30 — T-06.2a Switcher overlay and live previews → [tasks/030-t-06.2a-switcher-overlay-and-previews.md](tasks/030-t-06.2a-switcher-overlay-and-previews.md)
- [x] T31 — T-06.2b Switcher commit, Cmd+` cycling, interruptibility → [tasks/031-t-06.2b-switcher-commit-and-cycling.md](tasks/031-t-06.2b-switcher-commit-and-cycling.md)

## Phase 7 — T-07 Menu bar goes live

- [x] T32 — T-07.1a Adapter contract, states, and mock → [tasks/032-t-07.1a-adapter-contract-and-mock.md](tasks/032-t-07.1a-adapter-contract-and-mock.md)
- [x] T33 — T-07.1b Event subscription, restart re-subscribe, absence → [tasks/033-t-07.1b-subscription-restart-and-absence.md](tasks/033-t-07.1b-subscription-restart-and-absence.md)
- [x] T34 — T-07.2a NetworkManager read path → [tasks/034-t-07.2a-networkmanager-read-path.md](tasks/034-t-07.2a-networkmanager-read-path.md)
- [ ] T35 — T-07.2b NetworkManager join and polkit degradation → [tasks/035-t-07.2b-networkmanager-join-and-polkit.md](tasks/035-t-07.2b-networkmanager-join-and-polkit.md)
- [ ] T36 — T-07.3 Audio adapter (PipeWire/WirePlumber) → [tasks/036-t-07.3-audio-adapter-pipewire-wireplumber.md](tasks/036-t-07.3-audio-adapter-pipewire-wireplumber.md)
- [ ] T37 — T-07.4 Power adapter (UPower) → [tasks/037-t-07.4-power-adapter-upower.md](tasks/037-t-07.4-power-adapter-upower.md)
- [ ] T38 — T-07.5a Wi-Fi and volume status menus → [tasks/038-t-07.5a-wifi-and-volume-menus.md](tasks/038-t-07.5a-wifi-and-volume-menus.md)
- [ ] T39 — T-07.5b Battery menu, placeholder removal, keyboard a11y → [tasks/039-t-07.5b-battery-menu-and-placeholder-removal.md](tasks/039-t-07.5b-battery-menu-and-placeholder-removal.md)
- [ ] T40 — T-07.6a Absent-daemon masking matrix → [tasks/040-t-07.6a-absent-daemon-matrix.md](tasks/040-t-07.6a-absent-daemon-matrix.md)
- [ ] T41 — T-07.6b Menu-bar idle trace and capture → [tasks/041-t-07.6b-idle-trace-and-capture.md](tasks/041-t-07.6b-idle-trace-and-capture.md)

## Phase 8 — T-08 settingsd: one owner

- [ ] T42 — T-08.1a settingsd config model and D-Bus API → [tasks/042-t-08.1a-settingsd-model-and-dbus-api.md](tasks/042-t-08.1a-settingsd-model-and-dbus-api.md)
- [ ] T43 — T-08.1b settingsd persistence and migrations → [tasks/043-t-08.1b-settingsd-persistence-and-migrations.md](tasks/043-t-08.1b-settingsd-persistence-and-migrations.md)
- [ ] T44 — T-08.2a Shell migration to settingsd → [tasks/044-t-08.2a-shell-migration-to-settingsd.md](tasks/044-t-08.2a-shell-migration-to-settingsd.md)
- [ ] T45 — T-08.2b Design-system Theme binding → [tasks/045-t-08.2b-design-system-theme-binding.md](tasks/045-t-08.2b-design-system-theme-binding.md)
- [ ] T46 — T-08.2c Compositor motion/input policy migration → [tasks/046-t-08.2c-compositor-policy-migration.md](tasks/046-t-08.2c-compositor-policy-migration.md)
- [ ] T47 — T-08.3 Restart, resync, and key-schema documentation → [tasks/047-t-08.3-restart-resync-and-key-schema-documentation.md](tasks/047-t-08.3-restart-resync-and-key-schema-documentation.md)

## Phase 9 — T-09 Settings Wave 1

- [ ] T48 — T-09.1a Settings app shell → [tasks/048-t-09.1a-settings-app-shell.md](tasks/048-t-09.1a-settings-app-shell.md)
- [ ] T49 — T-09.1b Settings live-apply plumbing → [tasks/049-t-09.1b-settings-live-apply.md](tasks/049-t-09.1b-settings-live-apply.md)
- [ ] T50 — T-09.2 Appearance pane → [tasks/050-t-09.2-appearance-pane.md](tasks/050-t-09.2-appearance-pane.md)
- [ ] T51 — T-09.3 Wallpaper pane → [tasks/051-t-09.3-wallpaper-pane.md](tasks/051-t-09.3-wallpaper-pane.md)
- [ ] T52 — T-09.4 Desktop & Dock pane → [tasks/052-t-09.4-desktop-dock-pane.md](tasks/052-t-09.4-desktop-dock-pane.md)
- [ ] T53 — T-09.5 Displays-basic pane → [tasks/053-t-09.5-displays-basic-pane.md](tasks/053-t-09.5-displays-basic-pane.md)
- [ ] T54 — T-09.6a Settings menu-model publication → [tasks/054-t-09.6a-menu-model-publication.md](tasks/054-t-09.6a-menu-model-publication.md)
- [ ] T55 — T-09.6b Settings absence matrix and wave captures → [tasks/055-t-09.6b-absence-matrix-and-wave-captures.md](tasks/055-t-09.6b-absence-matrix-and-wave-captures.md)

## Phase 10 — T-10 Files MVP

- [ ] T56 — T-10.1a files-core streaming listing and model → [tasks/056-t-10.1a-files-core-streaming-listing.md](tasks/056-t-10.1a-files-core-streaming-listing.md)
- [ ] T57 — T-10.1b files-core sorting and platform fallback → [tasks/057-t-10.1b-files-core-sorting-and-platform.md](tasks/057-t-10.1b-files-core-sorting-and-platform.md)
- [ ] T58 — T-10.2a files-core operations → [tasks/058-t-10.2a-files-core-operations.md](tasks/058-t-10.2a-files-core-operations.md)
- [ ] T59 — T-10.2b Optimistic semantics and state preservation → [tasks/059-t-10.2b-optimistic-semantics.md](tasks/059-t-10.2b-optimistic-semantics.md)
- [ ] T60 — T-10.3a files-core trash → [tasks/060-t-10.3a-files-core-trash.md](tasks/060-t-10.3a-files-core-trash.md)
- [ ] T61 — T-10.3b files-core folder watcher → [tasks/061-t-10.3b-files-core-folder-watcher.md](tasks/061-t-10.3b-files-core-folder-watcher.md)
- [ ] T62 — T-10.4a Files window, toolbar, and sidebar → [tasks/062-t-10.4a-files-window-toolbar-sidebar.md](tasks/062-t-10.4a-files-window-toolbar-sidebar.md)
- [ ] T63 — T-10.4b Files list and icon views → [tasks/063-t-10.4b-files-list-and-icon-views.md](tasks/063-t-10.4b-files-list-and-icon-views.md)
- [ ] T64 — T-10.4c Files context menus, multi-select, optimistic UI → [tasks/064-t-10.4c-files-context-menus-multiselect.md](tasks/064-t-10.4c-files-context-menus-multiselect.md)
- [ ] T65 — T-10.5 Files performance budgets → [tasks/065-t-10.5-files-performance-budgets.md](tasks/065-t-10.5-files-performance-budgets.md)
- [ ] T66 — T-10.6a Dock trash source → [tasks/066-t-10.6a-dock-trash-source.md](tasks/066-t-10.6a-dock-trash-source.md)
- [ ] T67 — T-10.6b Drop-to-trash, Empty Trash, trash:// → [tasks/067-t-10.6b-dock-drop-to-trash-and-empty.md](tasks/067-t-10.6b-dock-drop-to-trash-and-empty.md)
- [ ] T68 — T-10.6c Show in Files, Downloads, and .desktop identity → [tasks/068-t-10.6c-files-navigation-and-desktop-identity.md](tasks/068-t-10.6c-files-navigation-and-desktop-identity.md)
- [ ] T69 — T-10.7 Files capture and acceptance walkthrough → [tasks/069-t-10.7-files-capture-and-acceptance-walkthrough.md](tasks/069-t-10.7-files-capture-and-acceptance-walkthrough.md)

## Phase 11 — T-11 Control Center + notifications

- [ ] T70 — T-11.1a Notification service core → [tasks/070-t-11.1a-notification-service-core.md](tasks/070-t-11.1a-notification-service-core.md)
- [ ] T71 — T-11.1b Notification actions and Dock badge replacement → [tasks/071-t-11.1b-notification-actions-and-dock-badge.md](tasks/071-t-11.1b-notification-actions-and-dock-badge.md)
- [ ] T72 — T-11.2a DND/Focus policy → [tasks/072-t-11.2a-dnd-focus-policy.md](tasks/072-t-11.2a-dnd-focus-policy.md)
- [ ] T73 — T-11.2b DND/Focus menu-bar reflection and Dock failure path → [tasks/073-t-11.2b-dnd-reflection-and-dock-failure.md](tasks/073-t-11.2b-dnd-reflection-and-dock-failure.md)
- [ ] T74 — T-11.3a Control Center panel and core tiles → [tasks/074-t-11.3a-control-center-panel-and-tiles.md](tasks/074-t-11.3a-control-center-panel-and-tiles.md)
- [ ] T75 — T-11.3b Focus/DND, dark mode, and Control Center a11y → [tasks/075-t-11.3b-control-center-focus-dark-a11y.md](tasks/075-t-11.3b-control-center-focus-dark-a11y.md)
- [ ] T76 — T-11.4a OSD overlay → [tasks/076-t-11.4a-osd-overlay.md](tasks/076-t-11.4a-osd-overlay.md)
- [ ] T77 — T-11.4b OSD keyboard/a11y and captures → [tasks/077-t-11.4b-osd-a11y-and-captures.md](tasks/077-t-11.4b-osd-a11y-and-captures.md)

## Phase 12 — T-12 Session + lock + idle

- [ ] T78 — T-12.1a Session manager and restart policy → [tasks/078-t-12.1a-session-manager-and-restart-policy.md](tasks/078-t-12.1a-session-manager-and-restart-policy.md)
- [ ] T79 — T-12.1b Session environment, systemd units, second-VT → [tasks/079-t-12.1b-session-environment-and-units.md](tasks/079-t-12.1b-session-environment-and-units.md)
- [ ] T80 — T-12.2 Display-manager entry and logout teardown → [tasks/080-t-12.2-display-manager-entry-and-logout-teardown.md](tasks/080-t-12.2-display-manager-entry-and-logout-teardown.md)
- [ ] T81 — T-12.3a Lock protocol and lock UI → [tasks/081-t-12.3a-lock-protocol-and-ui.md](tasks/081-t-12.3a-lock-protocol-and-ui.md)
- [ ] T82 — T-12.3b Lock PAM authentication → [tasks/082-t-12.3b-lock-pam-authentication.md](tasks/082-t-12.3b-lock-pam-authentication.md)
- [ ] T83 — T-12.3c Lock input capture and kill-resistance → [tasks/083-t-12.3c-lock-input-capture-and-kill-resistance.md](tasks/083-t-12.3c-lock-input-capture-and-kill-resistance.md)
- [ ] T84 — T-12.4a Idle timers → [tasks/084-t-12.4a-idle-timers.md](tasks/084-t-12.4a-idle-timers.md)
- [ ] T85 — T-12.4b Idle inhibitors and wake restore → [tasks/085-t-12.4b-idle-inhibitors-and-wake.md](tasks/085-t-12.4b-idle-inhibitors-and-wake.md)
- [ ] T86 — T-12.5a Suspend/resume cycle → [tasks/086-t-12.5a-suspend-resume-cycle.md](tasks/086-t-12.5a-suspend-resume-cycle.md)
- [ ] T87 — T-12.5b Session policy keys, kill matrix, capture → [tasks/087-t-12.5b-session-policy-keys-and-kill-matrix.md](tasks/087-t-12.5b-session-policy-keys-and-kill-matrix.md)

## Phase 13 — T-13 Portals + capture + clipboard

- [ ] T88 — T-13.1a Portal backend and session service → [tasks/088-t-13.1a-portal-backend-and-session-service.md](tasks/088-t-13.1a-portal-backend-and-session-service.md)
- [ ] T89 — T-13.1b Settings and GlobalShortcuts portals → [tasks/089-t-13.1b-settings-and-globalshortcuts-portals.md](tasks/089-t-13.1b-settings-and-globalshortcuts-portals.md)
- [ ] T90 — T-13.2a FileChooser portal → [tasks/090-t-13.2a-filechooser-portal.md](tasks/090-t-13.2a-filechooser-portal.md)
- [ ] T91 — T-13.2b FileChooser picker UI → [tasks/091-t-13.2b-filechooser-picker-ui.md](tasks/091-t-13.2b-filechooser-picker-ui.md)
- [ ] T92 — T-13.3a Screenshot portal and selection UI → [tasks/092-t-13.3a-screenshot-portal-and-selection.md](tasks/092-t-13.3a-screenshot-portal-and-selection.md)
- [ ] T93 — T-13.3b Screenshot save/copy and portal-only gate → [tasks/093-t-13.3b-screenshot-save-copy-and-gate.md](tasks/093-t-13.3b-screenshot-save-copy-and-gate.md)
- [ ] T94 — T-13.4a ScreenCast portal and source picker → [tasks/094-t-13.4a-screencast-portal-and-picker.md](tasks/094-t-13.4a-screencast-portal-and-picker.md)
- [ ] T95 — T-13.4b ScreenCast stream and stills fallback → [tasks/095-t-13.4b-screencast-stream-and-fallback.md](tasks/095-t-13.4b-screencast-stream-and-fallback.md)
- [ ] T96 — T-13.5a Clipboard text/image/uri-list round-trips → [tasks/096-t-13.5a-clipboard-round-trips.md](tasks/096-t-13.5a-clipboard-round-trips.md)
- [ ] T97 — T-13.5b Clipboard history (if specified) → [tasks/097-t-13.5b-clipboard-history.md](tasks/097-t-13.5b-clipboard-history.md)
- [ ] T98 — T-13.6 polkit authentication agent → [tasks/098-t-13.6-polkit-authentication-agent.md](tasks/098-t-13.6-polkit-authentication-agent.md)
- [ ] T99 — T-13.7 Flatpak validation and capture → [tasks/099-t-13.7-flatpak-validation-and-capture.md](tasks/099-t-13.7-flatpak-validation-and-capture.md)

## Phase 14 — T-14 Global menu + app index + compat

- [ ] T100 — T-14.1a app-index identity resolution and icons → [tasks/100-t-14.1a-app-index-identity-and-icons.md](tasks/100-t-14.1a-app-index-identity-and-icons.md)
- [ ] T101 — T-14.1b app-index events, launch registry, recency → [tasks/101-t-14.1b-app-index-events-and-recency.md](tasks/101-t-14.1b-app-index-events-and-recency.md)
- [ ] T102 — T-14.1c app-index subscription API → [tasks/102-t-14.1c-app-index-subscription.md](tasks/102-t-14.1c-app-index-subscription.md)
- [ ] T103 — T-14.2a menu-broker export model and fixed menu → [tasks/103-t-14.2a-menu-broker-export-and-fixed-menu.md](tasks/103-t-14.2a-menu-broker-export-and-fixed-menu.md)
- [ ] T104 — T-14.2b menu-broker accelerators and toggle → [tasks/104-t-14.2b-menu-broker-accelerators-and-toggle.md](tasks/104-t-14.2b-menu-broker-accelerators-and-toggle.md)
- [ ] T105 — T-14.3 StatusNotifier/AppIndicator tray → [tasks/105-t-14.3-statusnotifier-appindicator-tray.md](tasks/105-t-14.3-statusnotifier-appindicator-tray.md)
- [ ] T106 — T-14.4 DBusMenu bridge → [tasks/106-t-14.4-dbusmenu-bridge.md](tasks/106-t-14.4-dbusmenu-bridge.md)
- [ ] T107 — T-14.5 XDnD bridge → [tasks/107-t-14.5-xdnd-bridge.md](tasks/107-t-14.5-xdnd-bridge.md)
- [ ] T108 — T-14.6a Strange-app zoo run and matrix → [tasks/108-t-14.6a-strange-app-zoo-run.md](tasks/108-t-14.6a-strange-app-zoo-run.md)
- [ ] T109 — T-14.6b Strange-app zoo fixes → [tasks/109-t-14.6b-strange-app-zoo-fixes.md](tasks/109-t-14.6b-strange-app-zoo-fixes.md)
- [ ] T110 — T-14.7 Retire interim paths → [tasks/110-t-14.7-retire-interim-paths.md](tasks/110-t-14.7-retire-interim-paths.md)

## Phase 15 — T-15 System services + Settings Waves 2–3

- [ ] T111 — T-15.1a Bluetooth adapter → [tasks/111-t-15.1a-bluetooth-adapter.md](tasks/111-t-15.1a-bluetooth-adapter.md)
- [ ] T112 — T-15.1b Bluetooth pane and tile → [tasks/112-t-15.1b-bluetooth-pane-and-tile.md](tasks/112-t-15.1b-bluetooth-pane-and-tile.md)
- [ ] T113 — T-15.2a Storage and removable media adapter → [tasks/113-t-15.2a-storage-and-removable-media-adapter.md](tasks/113-t-15.2a-storage-and-removable-media-adapter.md)
- [ ] T114 — T-15.2b Storage and removable media pane and tile → [tasks/114-t-15.2b-storage-and-removable-media-pane-and-tile.md](tasks/114-t-15.2b-storage-and-removable-media-pane-and-tile.md)
- [ ] T115 — T-15.3a Sound and routing adapter → [tasks/115-t-15.3a-sound-and-routing-adapter.md](tasks/115-t-15.3a-sound-and-routing-adapter.md)
- [ ] T116 — T-15.3b Sound and routing pane and tile → [tasks/116-t-15.3b-sound-and-routing-pane-and-tile.md](tasks/116-t-15.3b-sound-and-routing-pane-and-tile.md)
- [ ] T117 — T-15.4a Keyboard, Mouse, and Trackpad adapter → [tasks/117-t-15.4a-keyboard-mouse-and-trackpad-adapter.md](tasks/117-t-15.4a-keyboard-mouse-and-trackpad-adapter.md)
- [ ] T118 — T-15.4b Keyboard, Mouse, and Trackpad pane and tile → [tasks/118-t-15.4b-keyboard-mouse-and-trackpad-pane-and-tile.md](tasks/118-t-15.4b-keyboard-mouse-and-trackpad-pane-and-tile.md)
- [ ] T119 — T-15.5a Mission Control and hot corners adapter → [tasks/119-t-15.5a-mission-control-and-hot-corners-adapter.md](tasks/119-t-15.5a-mission-control-and-hot-corners-adapter.md)
- [ ] T120 — T-15.5b Mission Control and hot corners pane and tile → [tasks/120-t-15.5b-mission-control-and-hot-corners-pane-and-tile.md](tasks/120-t-15.5b-mission-control-and-hot-corners-pane-and-tile.md)
- [ ] T121 — T-15.6a Battery and power profiles adapter → [tasks/121-t-15.6a-battery-and-power-profiles-adapter.md](tasks/121-t-15.6a-battery-and-power-profiles-adapter.md)
- [ ] T122 — T-15.6b Battery and power profiles pane and tile → [tasks/122-t-15.6b-battery-and-power-profiles-pane-and-tile.md](tasks/122-t-15.6b-battery-and-power-profiles-pane-and-tile.md)
- [ ] T123 — T-15.7a Notifications and Focus adapter → [tasks/123-t-15.7a-notifications-and-focus-adapter.md](tasks/123-t-15.7a-notifications-and-focus-adapter.md)
- [ ] T124 — T-15.7b Notifications and Focus pane and tile → [tasks/124-t-15.7b-notifications-and-focus-pane-and-tile.md](tasks/124-t-15.7b-notifications-and-focus-pane-and-tile.md)
- [ ] T125 — T-15.8a Lock Screen policy adapter → [tasks/125-t-15.8a-lock-screen-policy-adapter.md](tasks/125-t-15.8a-lock-screen-policy-adapter.md)
- [ ] T126 — T-15.8b Lock Screen policy pane and tile → [tasks/126-t-15.8b-lock-screen-policy-pane-and-tile.md](tasks/126-t-15.8b-lock-screen-policy-pane-and-tile.md)
- [ ] T127 — T-15.9a Menu Bar configuration adapter → [tasks/127-t-15.9a-menu-bar-configuration-adapter.md](tasks/127-t-15.9a-menu-bar-configuration-adapter.md)
- [ ] T128 — T-15.9b Menu Bar configuration pane and tile → [tasks/128-t-15.9b-menu-bar-configuration-pane-and-tile.md](tasks/128-t-15.9b-menu-bar-configuration-pane-and-tile.md)
- [ ] T129 — T-15.10a General, About, and Updates adapter → [tasks/129-t-15.10a-general-about-and-updates-adapter.md](tasks/129-t-15.10a-general-about-and-updates-adapter.md)
- [ ] T130 — T-15.10b General, About, and Updates pane and tile → [tasks/130-t-15.10b-general-about-and-updates-pane-and-tile.md](tasks/130-t-15.10b-general-about-and-updates-pane-and-tile.md)
- [ ] T131 — T-15.11a Users and Groups adapter → [tasks/131-t-15.11a-users-and-groups-adapter.md](tasks/131-t-15.11a-users-and-groups-adapter.md)
- [ ] T132 — T-15.11b Users and Groups pane and tile → [tasks/132-t-15.11b-users-and-groups-pane-and-tile.md](tasks/132-t-15.11b-users-and-groups-pane-and-tile.md)
- [ ] T133 — T-15.12a Printers and Scanners adapter → [tasks/133-t-15.12a-printers-and-scanners-adapter.md](tasks/133-t-15.12a-printers-and-scanners-adapter.md)
- [ ] T134 — T-15.12b Printers and Scanners pane and tile → [tasks/134-t-15.12b-printers-and-scanners-pane-and-tile.md](tasks/134-t-15.12b-printers-and-scanners-pane-and-tile.md)
- [ ] T135 — T-15.13a Privacy and Security adapter → [tasks/135-t-15.13a-privacy-and-security-adapter.md](tasks/135-t-15.13a-privacy-and-security-adapter.md)
- [ ] T136 — T-15.13b Privacy and Security pane and tile → [tasks/136-t-15.13b-privacy-and-security-pane-and-tile.md](tasks/136-t-15.13b-privacy-and-security-pane-and-tile.md)
- [ ] T137 — T-15.14a Accessibility adapter → [tasks/137-t-15.14a-accessibility-adapter.md](tasks/137-t-15.14a-accessibility-adapter.md)
- [ ] T138 — T-15.14b Accessibility pane and tile → [tasks/138-t-15.14b-accessibility-pane-and-tile.md](tasks/138-t-15.14b-accessibility-pane-and-tile.md)
- [ ] T139 — T-15.15a Network advanced (VPN) adapter → [tasks/139-t-15.15a-network-advanced-vpn-adapter.md](tasks/139-t-15.15a-network-advanced-vpn-adapter.md)
- [ ] T140 — T-15.15b Network advanced (VPN) pane and tile → [tasks/140-t-15.15b-network-advanced-vpn-pane-and-tile.md](tasks/140-t-15.15b-network-advanced-vpn-pane-and-tile.md)
- [ ] T141 — T-15.16 Absent-daemon matrix and breadth capture → [tasks/141-t-15.16-absent-daemon-matrix-and-breadth-capture.md](tasks/141-t-15.16-absent-daemon-matrix-and-breadth-capture.md)

## Phase 16 — T-16 Platform polish + packaging

- [ ] T142 — T-16.1a Per-output chrome sizing and reserved zones → [tasks/142-t-16.1a-per-output-chrome-sizing.md](tasks/142-t-16.1a-per-output-chrome-sizing.md)
- [ ] T143 — T-16.1b Per-output window placement → [tasks/143-t-16.1b-per-output-window-placement.md](tasks/143-t-16.1b-per-output-window-placement.md)
- [ ] T144 — T-16.2 Hotplug under load and lockstep → [tasks/144-t-16.2-hotplug-under-load-and-lockstep.md](tasks/144-t-16.2-hotplug-under-load-and-lockstep.md)
- [ ] T145 — T-16.3a Integer-scaled Xwayland → [tasks/145-t-16.3a-integer-scaled-xwayland.md](tasks/145-t-16.3a-integer-scaled-xwayland.md)
- [ ] T146 — T-16.3b Viewport downscale and chrome sizing → [tasks/146-t-16.3b-viewport-downscale-and-chrome.md](tasks/146-t-16.3b-viewport-downscale-and-chrome.md)
- [ ] T147 — T-16.6a AT-SPI and keyboard-only audit → [tasks/147-t-16.6a-atspi-and-keyboard-audit.md](tasks/147-t-16.6a-atspi-and-keyboard-audit.md)
- [ ] T148 — T-16.6b Magnifier and reduced-motion sweep → [tasks/148-t-16.6b-magnifier-and-reduced-motion-sweep.md](tasks/148-t-16.6b-magnifier-and-reduced-motion-sweep.md)
- [ ] T149 — T-16.7 Localization and i18n → [tasks/149-t-16.7-localization-and-i18n.md](tasks/149-t-16.7-localization-and-i18n.md)
- [ ] T150 — T-16.8a Crash/kill matrix → [tasks/150-t-16.8a-crash-kill-matrix.md](tasks/150-t-16.8a-crash-kill-matrix.md)
- [ ] T151 — T-16.8b Compositor-death behavior and restart-policy docs → [tasks/151-t-16.8b-compositor-death-and-restart-policy.md](tasks/151-t-16.8b-compositor-death-and-restart-policy.md)

## Phase 17 — T-17 The premium experience gate

- [ ] T152 — T-17.1a Nested window loop verification → [tasks/152-t-17.1a-nested-window-loop-verification.md](tasks/152-t-17.1a-nested-window-loop-verification.md)
- [ ] T153 — T-17.1b Workspace, Mission Control, and app-switch verification → [tasks/153-t-17.1b-workspace-overview-switcher-verification.md](tasks/153-t-17.1b-workspace-overview-switcher-verification.md)
- [ ] T154 — T-17.1c Flatpak/browser end-to-end verification → [tasks/154-t-17.1c-flatpak-browser-verification.md](tasks/154-t-17.1c-flatpak-browser-verification.md)
- [ ] T155 — T-17.3 Visual floor and reduced-motion sign-off → [tasks/155-t-17.3-visual-floor-and-reduced-motion-sign-off.md](tasks/155-t-17.3-visual-floor-and-reduced-motion-sign-off.md)
- [ ] T156 — T-17.5a Absent-daemon and crash matrix verification → [tasks/156-t-17.5a-absent-daemon-and-crash-matrix.md](tasks/156-t-17.5a-absent-daemon-and-crash-matrix.md)
- [ ] T157 — T-17.5b Leak and lock enforcement verification → [tasks/157-t-17.5b-leak-and-lock-enforcement-verification.md](tasks/157-t-17.5b-leak-and-lock-enforcement-verification.md)
- [ ] T158 — T-17.6 Unfamiliar-user test and sign-off report → [tasks/158-t-17.6-unfamiliar-user-test-and-sign-off-report.md](tasks/158-t-17.6-unfamiliar-user-test-and-sign-off-report.md)

## Phase 18 — Hardware rail (seat / spare GPU / clean VM)

- [ ] T159 — T-03.2 DRM first bring-up → [tasks/159-t-03.2-drm-first-bring-up.md](tasks/159-t-03.2-drm-first-bring-up.md)
- [ ] T160 — T-03.3 Hardware input validation → [tasks/160-t-03.3-hardware-input-validation.md](tasks/160-t-03.3-hardware-input-validation.md)
- [ ] T161 — T-03.4 DRM soak, teardown, runbook → [tasks/161-t-03.4-drm-soak-teardown-runbook.md](tasks/161-t-03.4-drm-soak-teardown-runbook.md)
- [ ] T162 — T-16.4 Suspend/resume soak → [tasks/162-t-16.4-suspend-resume-soak.md](tasks/162-t-16.4-suspend-resume-soak.md)
- [ ] T163 — T-16.5 Graphics driver matrix → [tasks/163-t-16.5-graphics-driver-matrix.md](tasks/163-t-16.5-graphics-driver-matrix.md)
- [ ] T164 — T-16.9 Fedora packaging and CI → [tasks/164-t-16.9-fedora-packaging-and-ci.md](tasks/164-t-16.9-fedora-packaging-and-ci.md)
- [ ] T165 — T-16.10 Debian packaging and CI → [tasks/165-t-16.10-debian-packaging-and-ci.md](tasks/165-t-16.10-debian-packaging-and-ci.md)
- [ ] T166 — T-16.11 Packaged-build performance re-measure → [tasks/166-t-16.11-packaged-build-performance-re-measure.md](tasks/166-t-16.11-packaged-build-performance-re-measure.md)
- [ ] T167 — T-17.2 DRM full-loop verification → [tasks/167-t-17.2-drm-full-loop-verification.md](tasks/167-t-17.2-drm-full-loop-verification.md)
- [ ] T168 — T-17.4 Performance budget verification → [tasks/168-t-17.4-performance-budget-verification.md](tasks/168-t-17.4-performance-budget-verification.md)

## Delivery order (unchanged in intent)

1. **Make the loop real** — T-01, T-02.
2. **Prove it on hardware early** — T-03 (hardware rail; never blocks the
   nested path).
3. **Make it look right** — T-04.
4. **Finish the loop** — T-05, T-06.
5. **Make the chrome live** — T-07 … T-11.
6. **Make it a desktop** — T-12 … T-16.
7. **Gate the premium experience** — T-17.

## Why this order

- **Vertical before horizontal.** Each unit crosses compositor → shell → app
  where it needs to, so integration bugs surface in days, not at the end.
- **Affordances before breadth.** A titlebar and a minimize animation are worth
  more than another settings pane.
- **Producers before consumers.** settingsd (T-08) lands before Settings (T-09)
  and Files (T-10); the adapters (T-07) land before Control Center (T-11).
- **Safety before exposure.** T-12.3 (lock enforcement) is a hard gate before
  any real session.
- **No hardware on the nested critical path.** The old plan had T-12 depend on
  T-03 and therefore T-13…T-17 all sit behind a seat that this host does not
  have. T-12 now develops nested-first; only the DRM capture is on the rail.

## Cross-cutting rules (carried from the legacy plan)

- **No duplicated state machines.** The compositor owns windows, Spaces, and
  outputs; settingsd owns settings; the shell observes and renders
  ([01-architecture.md](design/01-architecture.md)).
- **Everything but the compositor is restartable.** Compositor death ends the
  session by design.
- **Gesture-driven transitions are progress-based and interruptible.** A
  discrete "instant" code path is a bug ([10-design-system.md](design/10-design-system.md)).
- **Reduced-motion variant required** for every animation.
- **Degrade gracefully when a host daemon is absent** — absence is a normal
  state ([07-system-integration.md](design/07-system-integration.md)).
- **Reuse the plumbing.** Never write an SMB client, indexer, thumbnail daemon,
  credential store, or privilege-escalation scheme.
- **Original assets only.** Reproduce the interaction model, never Apple's
  bitmap output ([14-risks.md](design/14-risks.md)).
- **Design system is the single source of visual truth.** New UI consumes its
  components and tokens; `make check-design-tokens` and the gallery goldens
  enforce it.

## Inherited foundation (built by the legacy plan; not re-ticketed)

The new slices build on this; they do not rebuild it.

- **Compositor core** — fd-driven calloop loop, nested/DRM/headless backends,
  the pinned standard protocol surface, outputs and hotplug, damage-driven
  rendering, render counters (`docs/tasks/legacy/02-compositor-core.md`).
- **Input** — Cmd→Super / Option→Alt keymap, global shortcut engine, gesture
  pipeline, hot corners, pointer constraints, synthetic-input harness
  (`docs/tasks/legacy/03-input-keymaps-shortcuts.md`).
- **Window model** — floating/minimized/zoomed/fullscreen with restore
  geometry, focus, placement, interactive move/resize, popups
  (`docs/tasks/legacy/04-window-model.md`).
- **Spaces** — per-output lockstep Spaces, fullscreen Spaces, assignment,
  hotplug migration, wallpaper data (`docs/tasks/legacy/05-spaces-model.md`).
- **Xwayland** — eager start, `DISPLAY` hand-off, crash respawn, X11 windows in
  the same model (`docs/tasks/legacy/06-xwayland.md`).
- **Private shell protocols** — `df_core` trust/token handshake, chrome
  surfaces + reserved zones, window/workspace/output control
  (`docs/tasks/legacy/07-private-shell-protocols.md`).
- **Design system** — tokens + all 20 components + gallery goldens
  (`docs/tasks/legacy/08-design-system.md`).
- **Menu bar** — render/interaction core, overlay dropdown, hotplug re-anchor,
  fixed system + application menus (`docs/tasks/legacy/09-menu-bar.md`).
- **Dock** — presentation/interaction core, launch/pins, magnification,
  bounce, context menus, window chooser, drag rearrangement, auto-hide, Trash,
  Downloads stack, external drops, keyboard navigation, scene-graph commit path
  (`docs/tasks/legacy/10-dock.md`).
- **Overview state machine** — one machine for workspace switch / Mission
  Control / Desktop Reveal, commit rules, hit-test transfer, reduced motion,
  shell chrome (workspace strip, minimized strip, window grid), frame-time
  instrumentation (`docs/tasks/legacy/11-mission-control-workspace-ux.md`).
- **Dev workflow** — `make dev`, `make e2e`, `make soak`, nested/headless
  sessions, conformance suites (`docs/tasks/legacy/01-repo-scaffolding-ci-licensing.md`).

Known foundation gaps are absorbed by new slices: DRM bring-up and perf (T-03),
image wallpaper and live transforms (T-04/T-05), XDnD (T-14), per-output
chrome sizing (T-16).

## Explicitly deferred (post-premium-gate backlog)

Named so they cannot silently creep into the slices:

- Desktop icons (legacy T-19).
- Search/Spotlight-equivalent pane and app-index search.
- Internet Accounts (GOA).
- NVIDIA-specific validation and HDR/color-management staging.
- Printer/scanner breadth beyond the MVP pane.
- Screen Time / AI features (not planned).

## Performance budgets (apply to every slice)

Targets on baseline Intel/AMD hardware, measured inside the dev loop:

| Path | Budget | First measured |
|---|---|---|
| Workspace switch / Mission Control | 60 Hz, no dropped frames for the full gesture | T-03, T-05 |
| Input-to-photon latency | Under one frame, nested and DRM | T-03 |
| Idle desktop | Zero damage, zero client wakeups from our shell | T-03 |
| Animation system | One frame of compositor work per frame of animation | T-02, T-03 |
| Folder open (warm, 1k items) | < 50 ms to first frame, streaming listing | T-10 |
| List-view scroll (100k items) | 60 Hz windowed rendering, flat memory | T-10 |
| Rename/trash/new folder | Visible within one frame (optimistic) | T-10 |

## How to review a unit and a track

**Per unit (agent):**

1. Confirm the unit's headless tests are in `make e2e` and green, and
   `make check` is unchanged.
2. Check its `docs/captures/` artifact and `PROGRESS.md` note against the unit's
   acceptance list.
3. Tick the unit's acceptance boxes only then.

**Per track (human, batched):**

1. Check out the track's branch and run `make dev` (or `make demo` once T-01.6
   adds it).
2. Perform the track's demo checklist in the nested session.
3. Open the track's captures and compare them against the design docs / gallery
   goldens.
4. Tick the track's acceptance boxes. A track whose demo has not been watched is
   not done — but it no longer blocks the next unit.

## Legacy mapping

| New slice | Absorbs legacy tickets |
|---|---|
| T-01 | T-13 (functional SSD half), T-04 (window-menu primitives), T-10 (launch/activate/restore/close), T-09 (app name/Quit) |
| T-02 | T-35, T-10 (tile geometry hand-off) |
| T-03 | T-02 (DRM bring-up, FR-2/3/4/5 budgets), T-01 (soak), T-03 (hardware input validation) |
| T-04 | T-33, T-08 (material tokens), T-02 (render passes) |
| T-05 | T-11 (B-1…B-9), T-05 (image wallpaper + slide), T-14 (Desktop Reveal) |
| T-06 | T-12 |
| T-07 | T-20 (MVP slice A1/A3/A4), T-09 (status-item content) |
| T-08 | T-15 (MVP slice), T-10 (`dock.*` adoption), T-03 (input settings) |
| T-09 | T-16 (Wave 1) |
| T-10 | T-17, T-18 (MVP), T-10 (Trash/Show-in-Files integration) |
| T-11 | T-21, T-25 |
| T-12 | T-24, T-26 |
| T-13 | T-27, T-28, T-29 |
| T-14 | T-22, T-23, T-30, T-06 (XDnD, app matrix) |
| T-15 | T-20 (remaining adapters), T-16 (Waves 2–3) |
| T-16 | T-31, T-32, T-14 (hot-corner/desktop config) |
| T-17 | T-34, the Phase-2/3 exit criteria |

Legacy tickets not listed above are either part of the inherited foundation or
in the post-gate backlog above.

**Numbering note:** references to `T-xx` in the design docs, `PROGRESS.md`,
the legacy ticket files, and source comments use the **legacy** numbering.
This plan restarts at T-01, so a legacy "T-13" and a new "T-13" are different
tickets; use this table to translate.

<!-- symphony:status -->
**Pipeline status** — updated 2026-09-25T01:07:57Z · 34/168 done

- Completed: T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34
- Blocked: none
- Failed: none
- Remaining: T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55, T56, T57, T58, T59, T60, T61, T62, T63, T64, T65, T66, T67, T68, T69, T70, T71, T72, T73, T74, T75, T76, T77, T78, T79, T80, T81, T82, T83, T84, T85, T86, T87, T88, T89, T90, T91, T92, T93, T94, T95, T96, T97, T98, T99, T100, T101, T102, T103, T104, T105, T106, T107, T108, T109, T110, T111, T112, T113, T114, T115, T116, T117, T118, T119, T120, T121, T122, T123, T124, T125, T126, T127, T128, T129, T130, T131, T132, T133, T134, T135, T136, T137, T138, T139, T140, T141, T142, T143, T144, T145, T146, T147, T148, T149, T150, T151, T152, T153, T154, T155, T156, T157, T158, T159, T160, T161, T162, T163, T164, T165, T166, T167, T168
- Last finished: T34 — done · NetworkManager read-path adapter landed in new crate services/networkmanager with fixture tests, absence handling, ADR 0026, docs, Makefile gate, and nested capture; all tests and make e2e green.
<!-- /symphony:status -->
