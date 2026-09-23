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
[units/](tasks/units/). Every unit ends with its headless tests green and a capture
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
Every track is decomposed into one-session **work units** in [units/](tasks/units/),
listed in strict execution order below. A unit ends with its headless test green
in `make e2e`, `make check`/`make soak` unchanged, and a capture artifact
committed. Human sign-off (watch the demo, compare to the design docs and the
gallery goldens) is **batched at the track boundary**, not part of a unit — see
[SLICING-REVIEW.md](tasks/SLICING-REVIEW.md) for why the old definition of done made
every task un-completable.

| Track | Design reference | Units | Demo you can watch |
|---|---|---|---|
| [T-01](tasks/01-loop-v0-window-controls.md) | Loop v0 — window controls | 7 | launch → titled window → drag/zoom/minimize/restore/close |
| [T-02](tasks/02-loop-v1-lifecycle-motion.md) | Loop v1 — lifecycle motion | 6 | the loop with appear/minimize/restore/zoom/close motion |
| [T-03](tasks/03-real-session-bringup-perf.md) | Real session — bring-up & perf | 5 | the loop on DRM/logind, budgets measured [hw] |
| [T-04](tasks/04-loop-v2-materials.md) | Loop v2 — materials | 6 | blur, shadows, rounded corners |
| [T-05](tasks/05-loop-v3-mission-control-live.md) | Loop v3 — Mission Control live | 7 | live-surface overview, wallpaper slide, Desktop Reveal |
| [T-06](tasks/06-loop-v4-app-switcher.md) | Loop v4 — app switcher | 3 | Cmd+Tab with live previews |
| [T-07](tasks/07-menu-bar-live-status.md) | Menu bar goes live | 10 | real Wi-Fi, volume, battery |
| [T-08](tasks/08-settingsd-live-settings.md) | settingsd — one owner | 6 | one owner for settings, live signals |
| [T-09](tasks/09-settings-app-wave-1.md) | Settings Wave 1 | 8 | Appearance, Wallpaper, Desktop & Dock, Displays-basic |
| [T-10](tasks/10-files-mvp.md) | Files MVP | 14 | browse, open, rename, trash; Dock↔Files Trash |
| [T-11](tasks/11-control-center-notifications.md) | Control Center + notifications | 8 | panel, banners, OSD |
| [T-12](tasks/12-session-lock-idle.md) | Session + lock + idle | 10 | real login session, lock, idle, suspend [hw] |
| [T-13](tasks/13-portals-capture-clipboard.md) | Portals + capture + clipboard | 12 | Flatpak browser walkthrough |
| [T-14](tasks/14-global-menu-app-index-compat.md) | Global menu + app index + compat | 11 | real menus, tray, DBusMenu, XDnD, zoo |
| [T-15](tasks/15-system-services-breadth.md) | System services + Settings Waves 2–3 | 31 | Bluetooth, storage, printers, users, … |
| [T-16](tasks/16-platform-polish-packaging.md) | Platform polish + packaging | 15 | multi-monitor, scaling, soak, a11y, i18n, packages [hw] |
| [T-17](tasks/17-premium-gate.md) | The premium experience gate | 9 | the full loop on nested + DRM, at the visual floor |

## Work units (strict order)

168 below are the executable sequence. Each is one focused session. A unit's
"Depends on" is mandatory except for the hardware rail below. `[hw]` units
cannot run on this host (no free logind seat / VM); they are ordered where they
belong but may be marked **open** and skipped, then swept on the hardware rail
before T-17. Everything else is nested/headless and must run.

| # | Unit | [hw] | Depends on |
|---|---|---|---|
| 1 | [T-01.1 — Titlebar render element](tasks/units/001-t-01.1-titlebar-render-element.md) | — | inherited foundation |
| 2 | [T-01.2 — Traffic-light actions](tasks/units/002-t-01.2-traffic-light-actions.md) | — | T-01.1 |
| 3 | [T-01.3 — Titlebar drag, double-click, fullscreen reveal](tasks/units/003-t-01.3-titlebar-drag-double-click-fullscreen-reveal.md) | — | T-01.2 |
| 4 | [T-01.4 — Window menu](tasks/units/004-t-01.4-window-menu.md) | — | T-01.3 |
| 5 | [T-01.5 — Decoration tier policy and X11 correctness](tasks/units/005-t-01.5-decoration-tier-policy-and-x11-correctness.md) | — | T-01.4 |
| 6 | [T-01.6a — `make demo` harness](tasks/units/006-t-01.6a-make-demo-harness.md) | — | T-01.5 |
| 7 | [T-01.6b — Loop integration walkthrough and capture](tasks/units/007-t-01.6b-loop-integration-capture.md) | — | T-01.6a |
| 8 | [T-02.1a — Animation clock and frame discipline](tasks/units/008-t-02.1a-animation-clock.md) | — | T-01.6b |
| 9 | [T-02.1b — Window appear transition](tasks/units/009-t-02.1b-window-appear.md) | — | T-02.1a |
| 10 | [T-02.2 — Minimize and restore motion](tasks/units/010-t-02.2-minimize-and-restore-motion.md) | — | T-02.1b |
| 11 | [T-02.3 — Zoom and fullscreen transitions](tasks/units/011-t-02.3-zoom-and-fullscreen-transitions.md) | — | T-02.2 |
| 12 | [T-02.4a — Close ghost](tasks/units/012-t-02.4a-close-ghost.md) | — | T-02.3 |
| 13 | [T-02.4b — Close interruptibility and idle trace](tasks/units/013-t-02.4b-close-interruptibility-and-idle-trace.md) | — | T-02.4a |
| 14 | [T-03.1a — Nested idle trace and animation frame budget](tasks/units/014-t-03.1a-nested-idle-and-animation-budget.md) | — | T-02.2 |
| 15 | [T-03.1b — Latency instrument and direct-scanout template](tasks/units/015-t-03.1b-latency-and-scanout-instruments.md) | — | T-03.1a |
| 16 | [T-03.2 — DRM first bring-up](tasks/units/016-t-03.2-drm-first-bring-up.md) | yes | T-01.6b |
| 17 | [T-03.3 — Hardware input validation](tasks/units/017-t-03.3-hardware-input-validation.md) | yes | T-03.2 |
| 18 | [T-03.4 — DRM soak, teardown, runbook](tasks/units/018-t-03.4-drm-soak-teardown-runbook.md) | yes | T-03.3 |
| 19 | [T-04.1a — Real shadows](tasks/units/019-t-04.1a-shadows.md) | — | T-02.4b |
| 20 | [T-04.1b — Rounded-corner clipping](tasks/units/020-t-04.1b-rounded-corner-clipping.md) | — | T-04.1a |
| 21 | [T-04.2 — Backdrop blur pass](tasks/units/021-t-04.2-backdrop-blur-pass.md) | — | T-04.1b |
| 22 | [T-04.3 — Reusable scene-transform pass](tasks/units/022-t-04.3-reusable-scene-transform-pass.md) | — | T-04.2 |
| 23 | [T-04.4a — Material degrade tiers and instrumentation](tasks/units/023-t-04.4a-material-degrade-tiers.md) | — | T-04.3 |
| 24 | [T-04.4b — Light/dark, reduced motion, and sign-off package](tasks/units/024-t-04.4b-material-schemes-and-sign-off-package.md) | — | T-04.4a |
| 25 | [T-05.1a — Live-surface transform into the grid](tasks/units/025-t-05.1a-live-surface-transform.md) | — | T-04.4b |
| 26 | [T-05.1b — Live video at scale and degrade](tasks/units/026-t-05.1b-live-video-and-degrade.md) | — | T-05.1a |
| 27 | [T-05.2 — Hit-testing and selection on live representations](tasks/units/027-t-05.2-hit-testing-and-selection-on-live-representations.md) | — | T-05.1b |
| 28 | [T-05.3 — Drag a live representation between Spaces](tasks/units/028-t-05.3-drag-a-live-representation-between-spaces.md) | — | T-05.2 |
| 29 | [T-05.4 — Image wallpaper and per-Space slide](tasks/units/029-t-05.4-image-wallpaper-and-per-space-slide.md) | — | T-05.3 |
| 30 | [T-05.5 — Desktop Reveal](tasks/units/030-t-05.5-desktop-reveal.md) | — | T-05.4 |
| 31 | [T-05.6 — Overview frame budget and capture](tasks/units/031-t-05.6-overview-frame-budget-and-capture.md) | — | T-05.5 |
| 32 | [T-06.1 — App-switcher state machine](tasks/units/032-t-06.1-app-switcher-state-machine.md) | — | T-05.6 |
| 33 | [T-06.2a — Switcher overlay and live previews](tasks/units/033-t-06.2a-switcher-overlay-and-previews.md) | — | T-06.1 |
| 34 | [T-06.2b — Switcher commit, Cmd+` cycling, interruptibility](tasks/units/034-t-06.2b-switcher-commit-and-cycling.md) | — | T-06.2a |
| 35 | [T-07.1a — Adapter contract, states, and mock](tasks/units/035-t-07.1a-adapter-contract-and-mock.md) | — | T-01.6b |
| 36 | [T-07.1b — Event subscription, restart re-subscribe, absence](tasks/units/036-t-07.1b-subscription-restart-and-absence.md) | — | T-07.1a |
| 37 | [T-07.2a — NetworkManager read path](tasks/units/037-t-07.2a-networkmanager-read-path.md) | — | T-07.1b |
| 38 | [T-07.2b — NetworkManager join and polkit degradation](tasks/units/038-t-07.2b-networkmanager-join-and-polkit.md) | — | T-07.2a |
| 39 | [T-07.3 — Audio adapter (PipeWire/WirePlumber)](tasks/units/039-t-07.3-audio-adapter-pipewire-wireplumber.md) | — | T-07.2b |
| 40 | [T-07.4 — Power adapter (UPower)](tasks/units/040-t-07.4-power-adapter-upower.md) | — | T-07.3 |
| 41 | [T-07.5a — Wi-Fi and volume status menus](tasks/units/041-t-07.5a-wifi-and-volume-menus.md) | — | T-07.4 |
| 42 | [T-07.5b — Battery menu, placeholder removal, keyboard a11y](tasks/units/042-t-07.5b-battery-menu-and-placeholder-removal.md) | — | T-07.5a |
| 43 | [T-07.6a — Absent-daemon masking matrix](tasks/units/043-t-07.6a-absent-daemon-matrix.md) | — | T-07.5b |
| 44 | [T-07.6b — Menu-bar idle trace and capture](tasks/units/044-t-07.6b-idle-trace-and-capture.md) | — | T-07.6a |
| 45 | [T-08.1a — settingsd config model and D-Bus API](tasks/units/045-t-08.1a-settingsd-model-and-dbus-api.md) | — | T-01.6b |
| 46 | [T-08.1b — settingsd persistence and migrations](tasks/units/046-t-08.1b-settingsd-persistence-and-migrations.md) | — | T-08.1a |
| 47 | [T-08.2a — Shell migration to settingsd](tasks/units/047-t-08.2a-shell-migration-to-settingsd.md) | — | T-08.1b |
| 48 | [T-08.2b — Design-system Theme binding](tasks/units/048-t-08.2b-design-system-theme-binding.md) | — | T-08.2a |
| 49 | [T-08.2c — Compositor motion/input policy migration](tasks/units/049-t-08.2c-compositor-policy-migration.md) | — | T-08.2b |
| 50 | [T-08.3 — Restart, resync, and key-schema documentation](tasks/units/050-t-08.3-restart-resync-and-key-schema-documentation.md) | — | T-08.2c |
| 51 | [T-09.1a — Settings app shell](tasks/units/051-t-09.1a-settings-app-shell.md) | — | T-08.3 |
| 52 | [T-09.1b — Settings live-apply plumbing](tasks/units/052-t-09.1b-settings-live-apply.md) | — | T-09.1a |
| 53 | [T-09.2 — Appearance pane](tasks/units/053-t-09.2-appearance-pane.md) | — | T-09.1b |
| 54 | [T-09.3 — Wallpaper pane](tasks/units/054-t-09.3-wallpaper-pane.md) | — | T-09.2 |
| 55 | [T-09.4 — Desktop & Dock pane](tasks/units/055-t-09.4-desktop-dock-pane.md) | — | T-09.3 |
| 56 | [T-09.5 — Displays-basic pane](tasks/units/056-t-09.5-displays-basic-pane.md) | — | T-09.4 |
| 57 | [T-09.6a — Settings menu-model publication](tasks/units/057-t-09.6a-menu-model-publication.md) | — | T-09.5 |
| 58 | [T-09.6b — Settings absence matrix and wave captures](tasks/units/058-t-09.6b-absence-matrix-and-wave-captures.md) | — | T-09.6a |
| 59 | [T-10.1a — files-core streaming listing and model](tasks/units/059-t-10.1a-files-core-streaming-listing.md) | — | T-08.3 |
| 60 | [T-10.1b — files-core sorting and platform fallback](tasks/units/060-t-10.1b-files-core-sorting-and-platform.md) | — | T-10.1a |
| 61 | [T-10.2a — files-core operations](tasks/units/061-t-10.2a-files-core-operations.md) | — | T-10.1b |
| 62 | [T-10.2b — Optimistic semantics and state preservation](tasks/units/062-t-10.2b-optimistic-semantics.md) | — | T-10.2a |
| 63 | [T-10.3a — files-core trash](tasks/units/063-t-10.3a-files-core-trash.md) | — | T-10.2b |
| 64 | [T-10.3b — files-core folder watcher](tasks/units/064-t-10.3b-files-core-folder-watcher.md) | — | T-10.3a |
| 65 | [T-10.4a — Files window, toolbar, and sidebar](tasks/units/065-t-10.4a-files-window-toolbar-sidebar.md) | — | T-10.3b |
| 66 | [T-10.4b — Files list and icon views](tasks/units/066-t-10.4b-files-list-and-icon-views.md) | — | T-10.4a |
| 67 | [T-10.4c — Files context menus, multi-select, optimistic UI](tasks/units/067-t-10.4c-files-context-menus-multiselect.md) | — | T-10.4b |
| 68 | [T-10.5 — Files performance budgets](tasks/units/068-t-10.5-files-performance-budgets.md) | — | T-10.4c |
| 69 | [T-10.6a — Dock trash source](tasks/units/069-t-10.6a-dock-trash-source.md) | — | T-10.5 |
| 70 | [T-10.6b — Drop-to-trash, Empty Trash, trash://](tasks/units/070-t-10.6b-dock-drop-to-trash-and-empty.md) | — | T-10.6a |
| 71 | [T-10.6c — Show in Files, Downloads, and .desktop identity](tasks/units/071-t-10.6c-files-navigation-and-desktop-identity.md) | — | T-10.6b |
| 72 | [T-10.7 — Files capture and acceptance walkthrough](tasks/units/072-t-10.7-files-capture-and-acceptance-walkthrough.md) | — | T-10.6c |
| 73 | [T-11.1a — Notification service core](tasks/units/073-t-11.1a-notification-service-core.md) | — | T-07.6b |
| 74 | [T-11.1b — Notification actions and Dock badge replacement](tasks/units/074-t-11.1b-notification-actions-and-dock-badge.md) | — | T-11.1a |
| 75 | [T-11.2a — DND/Focus policy](tasks/units/075-t-11.2a-dnd-focus-policy.md) | — | T-11.1b |
| 76 | [T-11.2b — DND/Focus menu-bar reflection and Dock failure path](tasks/units/076-t-11.2b-dnd-reflection-and-dock-failure.md) | — | T-11.2a |
| 77 | [T-11.3a — Control Center panel and core tiles](tasks/units/077-t-11.3a-control-center-panel-and-tiles.md) | — | T-11.2b |
| 78 | [T-11.3b — Focus/DND, dark mode, and Control Center a11y](tasks/units/078-t-11.3b-control-center-focus-dark-a11y.md) | — | T-11.3a |
| 79 | [T-11.4a — OSD overlay](tasks/units/079-t-11.4a-osd-overlay.md) | — | T-11.3b |
| 80 | [T-11.4b — OSD keyboard/a11y and captures](tasks/units/080-t-11.4b-osd-a11y-and-captures.md) | — | T-11.4a |
| 81 | [T-12.1a — Session manager and restart policy](tasks/units/081-t-12.1a-session-manager-and-restart-policy.md) | — | T-08.3 |
| 82 | [T-12.1b — Session environment, systemd units, second-VT](tasks/units/082-t-12.1b-session-environment-and-units.md) | — | T-12.1a |
| 83 | [T-12.2 — Display-manager entry and logout teardown](tasks/units/083-t-12.2-display-manager-entry-and-logout-teardown.md) | — | T-12.1b |
| 84 | [T-12.3a — Lock protocol and lock UI](tasks/units/084-t-12.3a-lock-protocol-and-ui.md) | — | T-12.2 |
| 85 | [T-12.3b — Lock PAM authentication](tasks/units/085-t-12.3b-lock-pam-authentication.md) | — | T-12.3a |
| 86 | [T-12.3c — Lock input capture and kill-resistance](tasks/units/086-t-12.3c-lock-input-capture-and-kill-resistance.md) | — | T-12.3b |
| 87 | [T-12.4a — Idle timers](tasks/units/087-t-12.4a-idle-timers.md) | — | T-12.3c |
| 88 | [T-12.4b — Idle inhibitors and wake restore](tasks/units/088-t-12.4b-idle-inhibitors-and-wake.md) | — | T-12.4a |
| 89 | [T-12.5a — Suspend/resume cycle](tasks/units/089-t-12.5a-suspend-resume-cycle.md) | — | T-12.4b |
| 90 | [T-12.5b — Session policy keys, kill matrix, capture](tasks/units/090-t-12.5b-session-policy-keys-and-kill-matrix.md) | — | T-12.5a |
| 91 | [T-13.1a — Portal backend and session service](tasks/units/091-t-13.1a-portal-backend-and-session-service.md) | — | T-12.5b |
| 92 | [T-13.1b — Settings and GlobalShortcuts portals](tasks/units/092-t-13.1b-settings-and-globalshortcuts-portals.md) | — | T-13.1a |
| 93 | [T-13.2a — FileChooser portal](tasks/units/093-t-13.2a-filechooser-portal.md) | — | T-13.1b |
| 94 | [T-13.2b — FileChooser picker UI](tasks/units/094-t-13.2b-filechooser-picker-ui.md) | — | T-13.2a |
| 95 | [T-13.3a — Screenshot portal and selection UI](tasks/units/095-t-13.3a-screenshot-portal-and-selection.md) | — | T-13.2b |
| 96 | [T-13.3b — Screenshot save/copy and portal-only gate](tasks/units/096-t-13.3b-screenshot-save-copy-and-gate.md) | — | T-13.3a |
| 97 | [T-13.4a — ScreenCast portal and source picker](tasks/units/097-t-13.4a-screencast-portal-and-picker.md) | — | T-13.3b |
| 98 | [T-13.4b — ScreenCast stream and stills fallback](tasks/units/098-t-13.4b-screencast-stream-and-fallback.md) | — | T-13.4a |
| 99 | [T-13.5a — Clipboard text/image/uri-list round-trips](tasks/units/099-t-13.5a-clipboard-round-trips.md) | — | T-13.4b |
| 100 | [T-13.5b — Clipboard history (if specified)](tasks/units/100-t-13.5b-clipboard-history.md) | — | T-13.5a |
| 101 | [T-13.6 — polkit authentication agent](tasks/units/101-t-13.6-polkit-authentication-agent.md) | — | T-13.5b |
| 102 | [T-13.7 — Flatpak validation and capture](tasks/units/102-t-13.7-flatpak-validation-and-capture.md) | — | T-13.6 |
| 103 | [T-14.1a — app-index identity resolution and icons](tasks/units/103-t-14.1a-app-index-identity-and-icons.md) | — | T-09.6b |
| 104 | [T-14.1b — app-index events, launch registry, recency](tasks/units/104-t-14.1b-app-index-events-and-recency.md) | — | T-14.1a |
| 105 | [T-14.1c — app-index subscription API](tasks/units/105-t-14.1c-app-index-subscription.md) | — | T-14.1b |
| 106 | [T-14.2a — menu-broker export model and fixed menu](tasks/units/106-t-14.2a-menu-broker-export-and-fixed-menu.md) | — | T-14.1c |
| 107 | [T-14.2b — menu-broker accelerators and toggle](tasks/units/107-t-14.2b-menu-broker-accelerators-and-toggle.md) | — | T-14.2a |
| 108 | [T-14.3 — StatusNotifier/AppIndicator tray](tasks/units/108-t-14.3-statusnotifier-appindicator-tray.md) | — | T-14.2b |
| 109 | [T-14.4 — DBusMenu bridge](tasks/units/109-t-14.4-dbusmenu-bridge.md) | — | T-14.3 |
| 110 | [T-14.5 — XDnD bridge](tasks/units/110-t-14.5-xdnd-bridge.md) | — | T-14.4 |
| 111 | [T-14.6a — Strange-app zoo run and matrix](tasks/units/111-t-14.6a-strange-app-zoo-run.md) | — | T-14.5 |
| 112 | [T-14.6b — Strange-app zoo fixes](tasks/units/112-t-14.6b-strange-app-zoo-fixes.md) | — | T-14.6a |
| 113 | [T-14.7 — Retire interim paths](tasks/units/113-t-14.7-retire-interim-paths.md) | — | T-14.6b |
| 114 | [T-15.1a — Bluetooth adapter](tasks/units/114-t-15.1a-bluetooth-adapter.md) | — | T-14.7 |
| 115 | [T-15.1b — Bluetooth pane and tile](tasks/units/115-t-15.1b-bluetooth-pane-and-tile.md) | — | T-15.1a |
| 116 | [T-15.2a — Storage and removable media adapter](tasks/units/116-t-15.2a-storage-and-removable-media-adapter.md) | — | T-15.1b |
| 117 | [T-15.2b — Storage and removable media pane and tile](tasks/units/117-t-15.2b-storage-and-removable-media-pane-and-tile.md) | — | T-15.2a |
| 118 | [T-15.3a — Sound and routing adapter](tasks/units/118-t-15.3a-sound-and-routing-adapter.md) | — | T-15.2b |
| 119 | [T-15.3b — Sound and routing pane and tile](tasks/units/119-t-15.3b-sound-and-routing-pane-and-tile.md) | — | T-15.3a |
| 120 | [T-15.4a — Keyboard, Mouse, and Trackpad adapter](tasks/units/120-t-15.4a-keyboard-mouse-and-trackpad-adapter.md) | — | T-15.3b |
| 121 | [T-15.4b — Keyboard, Mouse, and Trackpad pane and tile](tasks/units/121-t-15.4b-keyboard-mouse-and-trackpad-pane-and-tile.md) | — | T-15.4a |
| 122 | [T-15.5a — Mission Control and hot corners adapter](tasks/units/122-t-15.5a-mission-control-and-hot-corners-adapter.md) | — | T-15.4b |
| 123 | [T-15.5b — Mission Control and hot corners pane and tile](tasks/units/123-t-15.5b-mission-control-and-hot-corners-pane-and-tile.md) | — | T-15.5a |
| 124 | [T-15.6a — Battery and power profiles adapter](tasks/units/124-t-15.6a-battery-and-power-profiles-adapter.md) | — | T-15.5b |
| 125 | [T-15.6b — Battery and power profiles pane and tile](tasks/units/125-t-15.6b-battery-and-power-profiles-pane-and-tile.md) | — | T-15.6a |
| 126 | [T-15.7a — Notifications and Focus adapter](tasks/units/126-t-15.7a-notifications-and-focus-adapter.md) | — | T-15.6b |
| 127 | [T-15.7b — Notifications and Focus pane and tile](tasks/units/127-t-15.7b-notifications-and-focus-pane-and-tile.md) | — | T-15.7a |
| 128 | [T-15.8a — Lock Screen policy adapter](tasks/units/128-t-15.8a-lock-screen-policy-adapter.md) | — | T-15.7b |
| 129 | [T-15.8b — Lock Screen policy pane and tile](tasks/units/129-t-15.8b-lock-screen-policy-pane-and-tile.md) | — | T-15.8a |
| 130 | [T-15.9a — Menu Bar configuration adapter](tasks/units/130-t-15.9a-menu-bar-configuration-adapter.md) | — | T-15.8b |
| 131 | [T-15.9b — Menu Bar configuration pane and tile](tasks/units/131-t-15.9b-menu-bar-configuration-pane-and-tile.md) | — | T-15.9a |
| 132 | [T-15.10a — General, About, and Updates adapter](tasks/units/132-t-15.10a-general-about-and-updates-adapter.md) | — | T-15.9b |
| 133 | [T-15.10b — General, About, and Updates pane and tile](tasks/units/133-t-15.10b-general-about-and-updates-pane-and-tile.md) | — | T-15.10a |
| 134 | [T-15.11a — Users and Groups adapter](tasks/units/134-t-15.11a-users-and-groups-adapter.md) | — | T-15.10b |
| 135 | [T-15.11b — Users and Groups pane and tile](tasks/units/135-t-15.11b-users-and-groups-pane-and-tile.md) | — | T-15.11a |
| 136 | [T-15.12a — Printers and Scanners adapter](tasks/units/136-t-15.12a-printers-and-scanners-adapter.md) | — | T-15.11b |
| 137 | [T-15.12b — Printers and Scanners pane and tile](tasks/units/137-t-15.12b-printers-and-scanners-pane-and-tile.md) | — | T-15.12a |
| 138 | [T-15.13a — Privacy and Security adapter](tasks/units/138-t-15.13a-privacy-and-security-adapter.md) | — | T-15.12b |
| 139 | [T-15.13b — Privacy and Security pane and tile](tasks/units/139-t-15.13b-privacy-and-security-pane-and-tile.md) | — | T-15.13a |
| 140 | [T-15.14a — Accessibility adapter](tasks/units/140-t-15.14a-accessibility-adapter.md) | — | T-15.13b |
| 141 | [T-15.14b — Accessibility pane and tile](tasks/units/141-t-15.14b-accessibility-pane-and-tile.md) | — | T-15.14a |
| 142 | [T-15.15a — Network advanced (VPN) adapter](tasks/units/142-t-15.15a-network-advanced-vpn-adapter.md) | — | T-15.14b |
| 143 | [T-15.15b — Network advanced (VPN) pane and tile](tasks/units/143-t-15.15b-network-advanced-vpn-pane-and-tile.md) | — | T-15.15a |
| 144 | [T-15.16 — Absent-daemon matrix and breadth capture](tasks/units/144-t-15.16-absent-daemon-matrix-and-breadth-capture.md) | — | T-15.15b |
| 145 | [T-16.1a — Per-output chrome sizing and reserved zones](tasks/units/145-t-16.1a-per-output-chrome-sizing.md) | — | T-15.16 |
| 146 | [T-16.1b — Per-output window placement](tasks/units/146-t-16.1b-per-output-window-placement.md) | — | T-16.1a |
| 147 | [T-16.2 — Hotplug under load and lockstep](tasks/units/147-t-16.2-hotplug-under-load-and-lockstep.md) | — | T-16.1b |
| 148 | [T-16.3a — Integer-scaled Xwayland](tasks/units/148-t-16.3a-integer-scaled-xwayland.md) | — | T-16.2 |
| 149 | [T-16.3b — Viewport downscale and chrome sizing](tasks/units/149-t-16.3b-viewport-downscale-and-chrome.md) | — | T-16.3a |
| 150 | [T-16.4 — Suspend/resume soak](tasks/units/150-t-16.4-suspend-resume-soak.md) | yes | T-16.3b |
| 151 | [T-16.5 — Graphics driver matrix](tasks/units/151-t-16.5-graphics-driver-matrix.md) | yes | T-16.4 |
| 152 | [T-16.6a — AT-SPI and keyboard-only audit](tasks/units/152-t-16.6a-atspi-and-keyboard-audit.md) | — | T-16.3b |
| 153 | [T-16.6b — Magnifier and reduced-motion sweep](tasks/units/153-t-16.6b-magnifier-and-reduced-motion-sweep.md) | — | T-16.6a |
| 154 | [T-16.7 — Localization and i18n](tasks/units/154-t-16.7-localization-and-i18n.md) | — | T-16.6b |
| 155 | [T-16.8a — Crash/kill matrix](tasks/units/155-t-16.8a-crash-kill-matrix.md) | — | T-16.7 |
| 156 | [T-16.8b — Compositor-death behavior and restart-policy docs](tasks/units/156-t-16.8b-compositor-death-and-restart-policy.md) | — | T-16.8a |
| 157 | [T-16.9 — Fedora packaging and CI](tasks/units/157-t-16.9-fedora-packaging-and-ci.md) | yes | T-16.8b |
| 158 | [T-16.10 — Debian packaging and CI](tasks/units/158-t-16.10-debian-packaging-and-ci.md) | yes | T-16.9 |
| 159 | [T-16.11 — Packaged-build performance re-measure](tasks/units/159-t-16.11-packaged-build-performance-re-measure.md) | yes | T-16.10 |
| 160 | [T-17.1a — Nested window loop verification](tasks/units/160-t-17.1a-nested-window-loop-verification.md) | — | T-16.8b |
| 161 | [T-17.1b — Workspace, Mission Control, and app-switch verification](tasks/units/161-t-17.1b-workspace-overview-switcher-verification.md) | — | T-17.1a |
| 162 | [T-17.1c — Flatpak/browser end-to-end verification](tasks/units/162-t-17.1c-flatpak-browser-verification.md) | — | T-17.1b |
| 163 | [T-17.2 — DRM full-loop verification](tasks/units/163-t-17.2-drm-full-loop-verification.md) | yes | T-17.1c |
| 164 | [T-17.3 — Visual floor and reduced-motion sign-off](tasks/units/164-t-17.3-visual-floor-and-reduced-motion-sign-off.md) | — | T-17.1c |
| 165 | [T-17.4 — Performance budget verification](tasks/units/165-t-17.4-performance-budget-verification.md) | yes | T-17.3 |
| 166 | [T-17.5a — Absent-daemon and crash matrix verification](tasks/units/166-t-17.5a-absent-daemon-and-crash-matrix.md) | — | T-17.3 |
| 167 | [T-17.5b — Leak and lock enforcement verification](tasks/units/167-t-17.5b-leak-and-lock-enforcement-verification.md) | — | T-17.5a |
| 168 | [T-17.6 — Unfamiliar-user test and sign-off report](tasks/units/168-t-17.6-unfamiliar-user-test-and-sign-off-report.md) | — | T-17.5b |

### Hardware rail

These units need a seat, spare GPU, or clean VM. When no hardware is available,
mark them open — do not skip them silently — and continue the sequence.

- [T-03.2 — DRM first bring-up](tasks/units/016-t-03.2-drm-first-bring-up.md) — yes
- [T-03.3 — Hardware input validation](tasks/units/017-t-03.3-hardware-input-validation.md) — yes
- [T-03.4 — DRM soak, teardown, runbook](tasks/units/018-t-03.4-drm-soak-teardown-runbook.md) — yes
- [T-16.4 — Suspend/resume soak](tasks/units/150-t-16.4-suspend-resume-soak.md) — yes
- [T-16.5 — Graphics driver matrix](tasks/units/151-t-16.5-graphics-driver-matrix.md) — yes
- [T-16.9 — Fedora packaging and CI](tasks/units/157-t-16.9-fedora-packaging-and-ci.md) — yes
- [T-16.10 — Debian packaging and CI](tasks/units/158-t-16.10-debian-packaging-and-ci.md) — yes
- [T-16.11 — Packaged-build performance re-measure](tasks/units/159-t-16.11-packaged-build-performance-re-measure.md) — yes
- [T-17.2 — DRM full-loop verification](tasks/units/163-t-17.2-drm-full-loop-verification.md) — yes
- [T-17.4 — Performance budget verification](tasks/units/165-t-17.4-performance-budget-verification.md) — yes

### Delivery order (unchanged in intent)

1. **Make the loop real** — T-01, T-02.
2. **Prove it on hardware early** — T-03 (hardware rail; never blocks the
   nested path).
3. **Make it look right** — T-04.
4. **Finish the loop** — T-05, T-06.
5. **Make the chrome live** — T-07 … T-11.
6. **Make it a desktop** — T-12 … T-16.
7. **Gate the premium experience** — T-17.

### Why this order

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
