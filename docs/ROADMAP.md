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
| [T-01](tasks/01-loop-v0-window-controls.md) | Loop v0 — window controls | 6 | launch → titled window → drag/zoom/minimize/restore/close |
| [T-02](tasks/02-loop-v1-lifecycle-motion.md) | Loop v1 — lifecycle motion | 4 | the loop with appear/minimize/restore/zoom/close motion |
| [T-03](tasks/03-real-session-bringup-perf.md) | Real session — bring-up & perf | 4 | the loop on DRM/logind, budgets measured [hw] |
| [T-04](tasks/04-loop-v2-materials.md) | Loop v2 — materials | 4 | blur, shadows, rounded corners |
| [T-05](tasks/05-loop-v3-mission-control-live.md) | Loop v3 — Mission Control live | 6 | live-surface overview, wallpaper slide, Desktop Reveal |
| [T-06](tasks/06-loop-v4-app-switcher.md) | Loop v4 — app switcher | 2 | Cmd+Tab with live previews |
| [T-07](tasks/07-menu-bar-live-status.md) | Menu bar goes live | 6 | real Wi-Fi, volume, battery |
| [T-08](tasks/08-settingsd-live-settings.md) | settingsd — one owner | 3 | one owner for settings, live signals |
| [T-09](tasks/09-settings-app-wave-1.md) | Settings Wave 1 | 6 | Appearance, Wallpaper, Desktop & Dock, Displays-basic |
| [T-10](tasks/10-files-mvp.md) | Files MVP | 7 | browse, open, rename, trash; Dock↔Files Trash |
| [T-11](tasks/11-control-center-notifications.md) | Control Center + notifications | 4 | panel, banners, OSD |
| [T-12](tasks/12-session-lock-idle.md) | Session + lock + idle | 5 | real login session, lock, idle, suspend [hw] |
| [T-13](tasks/13-portals-capture-clipboard.md) | Portals + capture + clipboard | 7 | Flatpak browser walkthrough |
| [T-14](tasks/14-global-menu-app-index-compat.md) | Global menu + app index + compat | 7 | real menus, tray, DBusMenu, XDnD, zoo |
| [T-15](tasks/15-system-services-breadth.md) | System services + Settings Waves 2–3 | 16 | Bluetooth, storage, printers, users, … |
| [T-16](tasks/16-platform-polish-packaging.md) | Platform polish + packaging | 11 | multi-monitor, scaling, soak, a11y, i18n, packages [hw] |
| [T-17](tasks/17-premium-gate.md) | The premium experience gate | 6 | the full loop on nested + DRM, at the visual floor |

## Work units (strict order)

1–104 below are the executable sequence. Each is one focused session. A unit's
"Depends on" is mandatory except for the hardware rail below. `[hw]` units
cannot run on this host (no free logind seat / VM); they are ordered where they
belong but may be marked **open** and skipped, then swept on the hardware rail
before T-17. Everything else is nested/headless and must run.

| # | Unit | [hw] | Depends on |
|---|---|---|---|
| 001 | [T-01.1 — Titlebar render element](tasks/units/001-t-01.1-titlebar-render-element.md) | — | inherited foundation |
| 002 | [T-01.2 — Traffic-light actions](tasks/units/002-t-01.2-traffic-light-actions.md) | — | T-01.1 |
| 003 | [T-01.3 — Titlebar drag, double-click, fullscreen reveal](tasks/units/003-t-01.3-titlebar-drag-double-click-fullscreen-reveal.md) | — | T-01.2 |
| 004 | [T-01.4 — Window menu](tasks/units/004-t-01.4-window-menu.md) | — | T-01.3 |
| 005 | [T-01.5 — Decoration tier policy and X11 correctness](tasks/units/005-t-01.5-decoration-tier-policy-and-x11-correctness.md) | — | T-01.4 |
| 006 | [T-01.6 — make demo harness and loop integration](tasks/units/006-t-01.6-make-demo-harness-and-loop-integration.md) | — | T-01.5 |
| 007 | [T-02.1 — Animation clock and window appear](tasks/units/007-t-02.1-animation-clock-and-window-appear.md) | — | T-01.6 |
| 008 | [T-02.2 — Minimize and restore motion](tasks/units/008-t-02.2-minimize-and-restore-motion.md) | — | T-02.1 |
| 009 | [T-02.3 — Zoom and fullscreen transitions](tasks/units/009-t-02.3-zoom-and-fullscreen-transitions.md) | — | T-02.2 |
| 010 | [T-02.4 — Close ghost, interruptibility, capture](tasks/units/010-t-02.4-close-ghost-interruptibility-capture.md) | — | T-02.3 |
| 011 | [T-03.1 — Nested performance budgets (host rail)](tasks/units/011-t-03.1-nested-performance-budgets-host-rail.md) | — | T-02.2 |
| 012 | [T-03.2 — DRM first bring-up](tasks/units/012-t-03.2-drm-first-bring-up.md) | yes | T-01.6 |
| 013 | [T-03.3 — Hardware input validation](tasks/units/013-t-03.3-hardware-input-validation.md) | yes | T-03.2 |
| 014 | [T-03.4 — DRM soak, teardown, runbook](tasks/units/014-t-03.4-drm-soak-teardown-runbook.md) | yes | T-03.3 |
| 015 | [T-04.1 — Real shadows and rounded-corner clipping](tasks/units/015-t-04.1-real-shadows-and-rounded-corner-clipping.md) | — | T-02.4 |
| 016 | [T-04.2 — Backdrop blur pass](tasks/units/016-t-04.2-backdrop-blur-pass.md) | — | T-04.1 |
| 017 | [T-04.3 — Reusable scene-transform pass](tasks/units/017-t-04.3-reusable-scene-transform-pass.md) | — | T-04.2 |
| 018 | [T-04.4 — Material degrade tiers, schemes, sign-off package](tasks/units/018-t-04.4-material-degrade-tiers-schemes-sign-off-package.md) | — | T-04.3 |
| 019 | [T-05.1 — Live-surface transform into the overview grid](tasks/units/019-t-05.1-live-surface-transform-into-the-overview-grid.md) | — | T-04.4 |
| 020 | [T-05.2 — Hit-testing and selection on live representations](tasks/units/020-t-05.2-hit-testing-and-selection-on-live-representations.md) | — | T-05.1 |
| 021 | [T-05.3 — Drag a live representation between Spaces](tasks/units/021-t-05.3-drag-a-live-representation-between-spaces.md) | — | T-05.2 |
| 022 | [T-05.4 — Image wallpaper and per-Space slide](tasks/units/022-t-05.4-image-wallpaper-and-per-space-slide.md) | — | T-05.3 |
| 023 | [T-05.5 — Desktop Reveal](tasks/units/023-t-05.5-desktop-reveal.md) | — | T-05.4 |
| 024 | [T-05.6 — Overview frame budget and capture](tasks/units/024-t-05.6-overview-frame-budget-and-capture.md) | — | T-05.5 |
| 025 | [T-06.1 — App-switcher state machine](tasks/units/025-t-06.1-app-switcher-state-machine.md) | — | T-05.6 |
| 026 | [T-06.2 — Switcher overlay with live previews](tasks/units/026-t-06.2-switcher-overlay-with-live-previews.md) | — | T-06.1 |
| 027 | [T-07.1 — Adapter framework and contract](tasks/units/027-t-07.1-adapter-framework-and-contract.md) | — | T-01.6 |
| 028 | [T-07.2 — Networking adapter (NetworkManager)](tasks/units/028-t-07.2-networking-adapter-networkmanager.md) | — | T-07.1 |
| 029 | [T-07.3 — Audio adapter (PipeWire/WirePlumber)](tasks/units/029-t-07.3-audio-adapter-pipewire-wireplumber.md) | — | T-07.2 |
| 030 | [T-07.4 — Power adapter (UPower)](tasks/units/030-t-07.4-power-adapter-upower.md) | — | T-07.3 |
| 031 | [T-07.5 — Status-item menus and placeholder removal](tasks/units/031-t-07.5-status-item-menus-and-placeholder-removal.md) | — | T-07.4 |
| 032 | [T-07.6 — Absent-daemon matrix and capture](tasks/units/032-t-07.6-absent-daemon-matrix-and-capture.md) | — | T-07.5 |
| 033 | [T-08.1 — settingsd daemon core and D-Bus API](tasks/units/033-t-08.1-settingsd-daemon-core-and-d-bus-api.md) | — | T-01.6 |
| 034 | [T-08.2 — Consumer migration to settingsd](tasks/units/034-t-08.2-consumer-migration-to-settingsd.md) | — | T-08.1 |
| 035 | [T-08.3 — Restart, resync, and key-schema documentation](tasks/units/035-t-08.3-restart-resync-and-key-schema-documentation.md) | — | T-08.2 |
| 036 | [T-09.1 — Settings app shell](tasks/units/036-t-09.1-settings-app-shell.md) | — | T-08.3 |
| 037 | [T-09.2 — Appearance pane](tasks/units/037-t-09.2-appearance-pane.md) | — | T-09.1 |
| 038 | [T-09.3 — Wallpaper pane](tasks/units/038-t-09.3-wallpaper-pane.md) | — | T-09.2 |
| 039 | [T-09.4 — Desktop & Dock pane](tasks/units/039-t-09.4-desktop-dock-pane.md) | — | T-09.3 |
| 040 | [T-09.5 — Displays-basic pane](tasks/units/040-t-09.5-displays-basic-pane.md) | — | T-09.4 |
| 041 | [T-09.6 — Menu model, absence matrix, wave sign-off](tasks/units/041-t-09.6-menu-model-absence-matrix-wave-sign-off.md) | — | T-09.5 |
| 042 | [T-10.1 — files-core streaming listing and model](tasks/units/042-t-10.1-files-core-streaming-listing-and-model.md) | — | T-08.3 |
| 043 | [T-10.2 — files-core operations and optimistic semantics](tasks/units/043-t-10.2-files-core-operations-and-optimistic-semantics.md) | — | T-10.1 |
| 044 | [T-10.3 — files-core trash and folder watcher](tasks/units/044-t-10.3-files-core-trash-and-folder-watcher.md) | — | T-10.2 |
| 045 | [T-10.4 — Files UI shell and views](tasks/units/045-t-10.4-files-ui-shell-and-views.md) | — | T-10.3 |
| 046 | [T-10.5 — Files performance budgets](tasks/units/046-t-10.5-files-performance-budgets.md) | — | T-10.4 |
| 047 | [T-10.6 — Dock integration: one Trash source](tasks/units/047-t-10.6-dock-integration-one-trash-source.md) | — | T-10.5 |
| 048 | [T-10.7 — Files capture and acceptance walkthrough](tasks/units/048-t-10.7-files-capture-and-acceptance-walkthrough.md) | — | T-10.6 |
| 049 | [T-11.1 — Notification service](tasks/units/049-t-11.1-notification-service.md) | — | T-07.6 |
| 050 | [T-11.2 — DND/Focus policy and menu-bar reflection](tasks/units/050-t-11.2-dnd-focus-policy-and-menu-bar-reflection.md) | — | T-11.1 |
| 051 | [T-11.3 — Control Center panel](tasks/units/051-t-11.3-control-center-panel.md) | — | T-11.2 |
| 052 | [T-11.4 — OSD and ambient capture](tasks/units/052-t-11.4-osd-and-ambient-capture.md) | — | T-11.3 |
| 053 | [T-12.1 — Session manager and services](tasks/units/053-t-12.1-session-manager-and-services.md) | — | T-08.3 |
| 054 | [T-12.2 — Display-manager entry and logout teardown](tasks/units/054-t-12.2-display-manager-entry-and-logout-teardown.md) | — | T-12.1 |
| 055 | [T-12.3 — Lock screen and enforcement](tasks/units/055-t-12.3-lock-screen-and-enforcement.md) | — | T-12.2 |
| 056 | [T-12.4 — Idle timers and inhibitors](tasks/units/056-t-12.4-idle-timers-and-inhibitors.md) | — | T-12.3 |
| 057 | [T-12.5 — Suspend/resume, policy keys, kill matrix, capture](tasks/units/057-t-12.5-suspend-resume-policy-keys-kill-matrix-capture.md) | — | T-12.4 |
| 058 | [T-13.1 — Portal backend skeleton and session service](tasks/units/058-t-13.1-portal-backend-skeleton-and-session-service.md) | — | T-12.5 |
| 059 | [T-13.2 — FileChooser portal](tasks/units/059-t-13.2-filechooser-portal.md) | — | T-13.1 |
| 060 | [T-13.3 — Screenshot portal and capture UI](tasks/units/060-t-13.3-screenshot-portal-and-capture-ui.md) | — | T-13.2 |
| 061 | [T-13.4 — ScreenCast portal and source picker](tasks/units/061-t-13.4-screencast-portal-and-source-picker.md) | — | T-13.3 |
| 062 | [T-13.5 — Clipboard round-trips](tasks/units/062-t-13.5-clipboard-round-trips.md) | — | T-13.4 |
| 063 | [T-13.6 — polkit authentication agent](tasks/units/063-t-13.6-polkit-authentication-agent.md) | — | T-13.5 |
| 064 | [T-13.7 — Flatpak validation and capture](tasks/units/064-t-13.7-flatpak-validation-and-capture.md) | — | T-13.6 |
| 065 | [T-14.1 — app-index service](tasks/units/065-t-14.1-app-index-service.md) | — | T-09.6 |
| 066 | [T-14.2 — menu-broker](tasks/units/066-t-14.2-menu-broker.md) | — | T-14.1 |
| 067 | [T-14.3 — StatusNotifier/AppIndicator tray](tasks/units/067-t-14.3-statusnotifier-appindicator-tray.md) | — | T-14.2 |
| 068 | [T-14.4 — DBusMenu bridge](tasks/units/068-t-14.4-dbusmenu-bridge.md) | — | T-14.3 |
| 069 | [T-14.5 — XDnD bridge](tasks/units/069-t-14.5-xdnd-bridge.md) | — | T-14.4 |
| 070 | [T-14.6 — Strange-app zoo](tasks/units/070-t-14.6-strange-app-zoo.md) | — | T-14.5 |
| 071 | [T-14.7 — Retire interim paths](tasks/units/071-t-14.7-retire-interim-paths.md) | — | T-14.6 |
| 072 | [T-15.1 — Bluetooth](tasks/units/072-t-15.1-bluetooth.md) | — | T-14.7 |
| 073 | [T-15.2 — Storage and removable media](tasks/units/073-t-15.2-storage-and-removable-media.md) | — | T-15.1 |
| 074 | [T-15.3 — Sound and routing](tasks/units/074-t-15.3-sound-and-routing.md) | — | T-15.2 |
| 075 | [T-15.4 — Keyboard, Mouse, and Trackpad](tasks/units/075-t-15.4-keyboard-mouse-and-trackpad.md) | — | T-15.3 |
| 076 | [T-15.5 — Mission Control and hot corners](tasks/units/076-t-15.5-mission-control-and-hot-corners.md) | — | T-15.4 |
| 077 | [T-15.6 — Battery and power profiles](tasks/units/077-t-15.6-battery-and-power-profiles.md) | — | T-15.5 |
| 078 | [T-15.7 — Notifications and Focus](tasks/units/078-t-15.7-notifications-and-focus.md) | — | T-15.6 |
| 079 | [T-15.8 — Lock Screen policy](tasks/units/079-t-15.8-lock-screen-policy.md) | — | T-15.7 |
| 080 | [T-15.9 — Menu Bar configuration](tasks/units/080-t-15.9-menu-bar-configuration.md) | — | T-15.8 |
| 081 | [T-15.10 — General, About, and Updates](tasks/units/081-t-15.10-general-about-and-updates.md) | — | T-15.9 |
| 082 | [T-15.11 — Users and Groups](tasks/units/082-t-15.11-users-and-groups.md) | — | T-15.10 |
| 083 | [T-15.12 — Printers and Scanners](tasks/units/083-t-15.12-printers-and-scanners.md) | — | T-15.11 |
| 084 | [T-15.13 — Privacy and Security](tasks/units/084-t-15.13-privacy-and-security.md) | — | T-15.12 |
| 085 | [T-15.14 — Accessibility](tasks/units/085-t-15.14-accessibility.md) | — | T-15.13 |
| 086 | [T-15.15 — Network advanced (VPN)](tasks/units/086-t-15.15-network-advanced-vpn.md) | — | T-15.14 |
| 087 | [T-15.16 — Absent-daemon matrix and breadth capture](tasks/units/087-t-15.16-absent-daemon-matrix-and-breadth-capture.md) | — | T-15.15 |
| 088 | [T-16.1 — Per-output chrome sizing and placement](tasks/units/088-t-16.1-per-output-chrome-sizing-and-placement.md) | — | T-15.16 |
| 089 | [T-16.2 — Hotplug under load and lockstep](tasks/units/089-t-16.2-hotplug-under-load-and-lockstep.md) | — | T-16.1 |
| 090 | [T-16.3 — Fractional scaling](tasks/units/090-t-16.3-fractional-scaling.md) | — | T-16.2 |
| 091 | [T-16.4 — Suspend/resume soak](tasks/units/091-t-16.4-suspend-resume-soak.md) | yes | T-16.3 |
| 092 | [T-16.5 — Graphics driver matrix](tasks/units/092-t-16.5-graphics-driver-matrix.md) | yes | T-16.4 |
| 093 | [T-16.6 — Accessibility audit](tasks/units/093-t-16.6-accessibility-audit.md) | — | T-16.5 |
| 094 | [T-16.7 — Localization and i18n](tasks/units/094-t-16.7-localization-and-i18n.md) | — | T-16.6 |
| 095 | [T-16.8 — Crash recovery kill matrix](tasks/units/095-t-16.8-crash-recovery-kill-matrix.md) | — | T-16.7 |
| 096 | [T-16.9 — Fedora packaging and CI](tasks/units/096-t-16.9-fedora-packaging-and-ci.md) | yes | T-16.8 |
| 097 | [T-16.10 — Debian packaging and CI](tasks/units/097-t-16.10-debian-packaging-and-ci.md) | yes | T-16.9 |
| 098 | [T-16.11 — Packaged-build performance re-measure](tasks/units/098-t-16.11-packaged-build-performance-re-measure.md) | yes | T-16.10 |
| 099 | [T-17.1 — Nested full-loop verification](tasks/units/099-t-17.1-nested-full-loop-verification.md) | — | T-16.11 |
| 100 | [T-17.2 — DRM full-loop verification](tasks/units/100-t-17.2-drm-full-loop-verification.md) | yes | T-17.1 |
| 101 | [T-17.3 — Visual floor and reduced-motion sign-off](tasks/units/101-t-17.3-visual-floor-and-reduced-motion-sign-off.md) | — | T-17.2 |
| 102 | [T-17.4 — Performance budget verification](tasks/units/102-t-17.4-performance-budget-verification.md) | yes | T-17.3 |
| 103 | [T-17.5 — Robustness verification](tasks/units/103-t-17.5-robustness-verification.md) | — | T-17.4 |
| 104 | [T-17.6 — Unfamiliar-user test and sign-off report](tasks/units/104-t-17.6-unfamiliar-user-test-and-sign-off-report.md) | — | T-17.5 |

### Hardware rail

These units need a seat, spare GPU, or clean VM. When no hardware is available,
mark them open — do not skip them silently — and continue the sequence.

- [T-03.2 — DRM first bring-up](tasks/units/012-t-03.2-drm-first-bring-up.md) — yes (seat / spare GPU / VM)
- [T-03.3 — Hardware input validation](tasks/units/013-t-03.3-hardware-input-validation.md) — yes
- [T-03.4 — DRM soak, teardown, runbook](tasks/units/014-t-03.4-drm-soak-teardown-runbook.md) — yes
- [T-16.4 — Suspend/resume soak](tasks/units/091-t-16.4-suspend-resume-soak.md) — yes
- [T-16.5 — Graphics driver matrix](tasks/units/092-t-16.5-graphics-driver-matrix.md) — yes
- [T-16.9 — Fedora packaging and CI](tasks/units/096-t-16.9-fedora-packaging-and-ci.md) — yes (clean VM)
- [T-16.10 — Debian packaging and CI](tasks/units/097-t-16.10-debian-packaging-and-ci.md) — yes (clean VM)
- [T-16.11 — Packaged-build performance re-measure](tasks/units/098-t-16.11-packaged-build-performance-re-measure.md) — yes (packaged VM)
- [T-17.2 — DRM full-loop verification](tasks/units/100-t-17.2-drm-full-loop-verification.md) — yes
- [T-17.4 — Performance budget verification](tasks/units/102-t-17.4-performance-budget-verification.md) — yes

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
