# Design ↔ code status and gap register

| | |
|---|---|
| **Status** | living document |
| **Audited at** | `5928a51` (2026-11-03) |
| **Scope** | every document in `docs/design/`, the task roadmap, and the code paths those documents claim |
| **Method** | read the design docs and ADRs against the current tree; verify each user-visible claim at the named source; distinguish *design target* from *shipped behavior* |

This is the answer to "is the design up to date with the code?" It is not a
second design document. The topic docs remain the source of truth for **what we
intend to build**; this file is the source of truth for **what is actually in
the tree today, what is partial, and what is still only a design**.

The rule for future work:

> A design doc describes the target. This file records the delta. If a change
> makes a sentence in a design doc false, update the design doc **and** the
> corresponding row here in the same change.

## 1. Verdict

The architecture is substantially implemented, not a paper design. The
compositor, shell, services, first-party apps, design system, private protocols,
session model, lock, portals, and the GPU material all exist and have scripted
evidence. The T-01…T-20 track sequence is essentially complete at the
agent-verified level.

The documentation is **mostly current but not uniformly trustworthy**:

- Several docs still describe the pre-T-20 world (DRM feather material, flat
  chrome, desktop icons as "later").
- Several docs describe target behavior as shipped (Files tabs/copy/undo,
  VRR/night light/color management, system-menu power actions, Control
  Center's original five tiles).
- Some recent decisions were made in code without a design record (DRM sampled
  material default and its fallback boundaries; Files thumbnails and
  default-handler opening; window activation restacking).
- One ADR number is duplicated (`0123`), and a few cross-links are broken.

Those are now reconciled in the topic docs, and the remaining gaps are listed
below. The important architectural conclusion is separate from documentation:

> **The largest competitive risk is no longer missing features. It is the
> shell's presentation transport and the fixed 60 Hz scheduling model.** The
> compositor now has a credible live-surface, GPU-material architecture; the
> shell still renders QML offscreen, reads it back with `grabWindow()`, copies
> it into a fresh `wl_shm` buffer, and never sets a buffer scale or consumes
> frame callbacks. That is the gap most likely to make a technically strong
> desktop feel second-rate on real hardware.

The **v0.2.0 total-experience track** adds the other half of that judgement:
the Dock click tree, menu bar and extras, Overview/workspaces, app switcher,
Search, Applications drawer, System Settings, animations, HUDs, and the
cross-distro ownership of every setting. The target is
[22-total-experience.md](22-total-experience.md); the row map is
[settings-business-logic-map.md](settings-business-logic-map.md); the task
sequence is [v0.2.0/ROADMAP.md](../v0.2.0/ROADMAP.md). A desktop that renders
beautifully but has a dead Dock click, a label-less menu, or a fake Settings
toggle is not a premium experience.

## 2. Status legend

| State | Meaning |
|---|---|
| **Implemented** | behavior exists, is exercised by the named tests/captures, and matches the design |
| **Partial** | the core exists but a named part is a stub, deferred, or only model-level |
| **Design-only** | documented, no implementation in the tree |
| **Hardware-open** | implemented and headless-tested; the real-device/seat half is not verified |
| **Human-open** | agent evidence exists; the human sign-off/verdict is still pending |
| **Superseded** | kept as history; a newer ADR/audit replaces the decision |

A track marked **done** below means "agent-done at the track's stated bar",
which is not the same as "hardware-verified" or "human-signed-off". The T-17
report already uses that distinction; this file applies it consistently.

## 3. Track status

| Track | State | What is actually in the tree | Open items |
|---|---|---|---|
| T-01 Loop v0 — window controls | Implemented | SSD titlebar, traffic lights, drag/double-click, window menu, tier policy, demo harness, captures | window menu is still label-less (G-06) |
| T-02 Loop v1 — lifecycle motion | Implemented | one animation clock, appear/minimize/restore/zoom/fullscreen/close, interruptibility, reduced motion | — |
| T-03 Real session — bring-up/perf | Partial / hardware-open | DRM backend, bring-up classification, input matrix, soak/runbook, presentation instrumentation | real DRM bring-up, real-device input, DRM soak, multi-GPU import run are OPEN (T159–T161) |
| T-04 Loop v2 — materials | Implemented | shadows, rounded corners, feather backdrop, scene transform, degrade tiers, light/dark | superseded in part by T-20's sampled material; F3 waiver narrowed |
| T-05 Loop v3 — Mission Control live | Implemented | live-surface grid, video at scale, hit-test/selection, drag between Spaces, wallpaper slide, Desktop Reveal, gesture trace | paging at the readable floor and neighbour-Space reveal are deferred (G-15) |
| T-06 Loop v4 — app switcher | Implemented | compositor machine, live previews, Cmd+` cycling, pointer commit, reduced motion | — |
| T-07 Menu bar goes live | Implemented | adapter contract, NetworkManager/audio/power, status menus, absence matrix, idle trace | real daemons/hardware half is not in CI |
| T-08 settingsd — one owner | Implemented | typed schema, D-Bus, persistence/migrations, shell client, Theme binding, compositor policy | — |
| T-09 Settings Wave 1 | Implemented | app shell, live apply, Appearance/Wallpaper/Desktop & Dock/Displays, menu model, absence matrix | Displays advanced rows are capability-gated with OPEN reasons (G-13) |
| T-10 Files MVP | **Partial** | files-core listing/sort/ops/trash/watch, C ABI, sidebar/favorites/volumes, browser/views/selection/context menus, optimistic rename/new-folder/trash, file opening/thumbnails, perf budgets, Dock Trash, reveal, measured 52/198/30/28 pt chrome, status bar, toolbar-hidden mode, Get Info inspector, captures | tabs/new window, copy/move/paste, undo, progress/conflicts, Open With, Files menu publication, watcher wiring (G-04, G-14) |
| T-11 Control Center + notifications | Implemented | notification service/policy/actions, Control Center, OSD, a11y | some Control Center links/actions remain follow-ups |
| T-12 Session + lock + idle | **Partial** | session plan/supervisor/units/entry, lock protocol/PAM/kill-resistance, idle engine/inhibitors, suspend model, policy keys | no production idle service, no concrete logind suspend backend, power actions are stubs (G-10) |
| T-13 Portals + capture + clipboard | **Partial** | portal backend, settings/globalshortcuts/filechooser/screenshot/screencast, clipboard history, polkit, Flatpak walkthrough | live PipeWire screencast producer is still the stills fallback; DRM screenshot capture absent (G-03) |
| T-14 Global menu + app index + compat | **Implemented** | app-index identity/events/subscriptions, menu-broker native tier, tray, DBusMenu focus/click consumption, hide/hide-others/show-all, dynamic Window menu, Services/Recent Items, XDnD model, zoo | XDnD runtime half (G-07) |
| T-15 System services + Settings 2–3 | Implemented | 26 shipped panes, adapters, Control Center tiles, absence matrix | real-daemon/VM half is not automated |
| T-16 Platform polish + packaging | **Partial** | per-output chrome/placement, hotplug, integer Xwayland, fractional chrome, a11y audit, magnifier, i18n, crash matrix, pointer timestamps | driver matrix, Fedora/Debian packaging, packaged perf, DRM full loop, baseline perf (G-11) |
| T-17 Premium gate | Partial / human-open | nested loop/navigation/Flatpak captures, visual floor, robustness, leak/lock, sign-off report | DRM full loop and baseline perf OPEN; human unfamiliar-user verdict batched |
| T-18 Wallpaper content provider | Implemented | wallpaperd, shipped default, provider cache, pane collections/attribution, absence | — |
| T-19 Desktop affordances | Implemented | Tabler line + Phosphor fill icon split, Settings category tiles, menu-bar/Dock/app icons, Applications drawer, desktop items, systemd-provisioned Files desktop | — |
| T-20 Tahoe liquid-glass material | **Partial** | scene texture, Kawase blur, glass shader, all chrome roles, DRM sampled path, effective material event, software fallback, tolerance captures, native DP-1 diagnostics and visual sweep (T191a/T191b), native baseline re-verification (T208: `captures/t208-native-acceptance.md`), nested legacy/replacement acceptance (T207: `captures/t207-nested-acceptance.md`), replacement default enabled by operator decision (ADR 0234) | native replacement evidence OPEN (T208 gate: native direct scanout candidates>0/direct 0; native lock-exit OPEN), multi-GPU/rotation, quality floor, cursor/overlay-plane evidence, refresh scheduling (G-08, G-12, G-02) |
| T-21 Presentation foundation (v0.1.1) | **Proposed** | task set T183–T193: shell transport/HiDPI, per-output refresh scheduling, material cost/floor, native evidence | all rows; see [tasks/0.1.1](../tasks/0.1.1/README.md) |
| T-22 Total desktop experience (v0.2.0) | **Proposed** | task set T209–T262: Settings map, menu bar/extras, Dock, Overview/workspaces, switcher, Search, drawer, desktop, dialogs, motion/HUDs, cross-distro. T219 landed the experience contract: 88 surfaces / 418 state rows, 169 captured + 71 verified, 178 explicit `OPEN`; `make experience-schema-check` is in `make gates`. T230a extended the contract with the 2026-11-08 structural review: 89 surfaces / 482 state rows, 168 captured + 74 verified, 240 explicit `OPEN`; the new `N-WIN`, `SET-GENERAL`, `Q-AD`, `S-NC`, `S-CC`, and `N-FILES` states carry the measured chrome/About/drawer/notification/Control Center/Files geometry and the 14 review `OPEN` ids. T230b landed the window-chrome style system: `WindowStyle` (0/32/40/48/52/66/88 pt), 16 pt AX-frame/13 pt drawn traffic lights with title-row-derived 8/18 pt insets and 23 pt spacing, content-layer accessory-bar tokens, the compositor SSD style seam, and the 52 pt unified Settings/Files migration; long-title/RTL stay `OPEN`. T230c added the separate 280×503 About utility panel, the canonical 740×820 Settings → General → About page, original vector hero, host-backed explicit absence rows, utility gradient/shadow, system-menu launch, the measured row/action layout, and the live light/dark menu journey; long/localized surfaces are captured at 1× and tested at 1×/1.5×/2×, while controlled material fit over the 21 reference backgrounds and a live localized host remain `OPEN`. T230d landed Files' measured 52/198/30/28 pt window anatomy, the status bar (item/selection/free space), the 64 pt toolbar-hidden mode with the sidebar hidden and a full-width status bar, the 164×52 pt view group with 40×36 pt segments and explicit disabled Column/Gallery reasons, and the 328×1034 pt Get Info auxiliary window with seven single-column sections, real rename editing and multi-selection aggregation; tabs, column/gallery rendering, search results, tags, customization and motion remain `OPEN` with owners/reruns in `09-files.md` and the structural review. T231 landed one status-item registry (`StatusItemRegistry.qml`) with priority order, third-party-last overflow and the overflow menu; migrated Wi-Fi/Bluetooth/Battery/Sound/Search/Notifications/Focus/Accessibility/Date & Time/Quick Settings/Overview and tray items to it; the clock is a registry item whose click opens the Date & Time calendar panel (seconds/24-hour/date formats, month grid, keyboard traversal and AX names), while Notifications opens the Notification Center; schema revision 24 adds `menu.control.clock`/`notifications`/`controlCenter`; the shared `Popup` anchor now tracks live geometry; tests and the `make t231-extras-capture` stills cover clock formats, calendar, item removal and third-party overflow (ADR 0248). T232 completed the six core extras: Wi-Fi current/known/other networks, inline details, join, scanning/error and `Wi-Fi Settings…`; Bluetooth power, paired devices, discovery/nearby pairing and `Bluetooth Settings…`; Battery charge/health/low/full, live power mode and `Battery Settings…`; Sound output routing, volume/mute and `Sound Settings…`; Search activation and `menu.control.search` visibility; and the Quick Settings panel plus its customization link. The shell now opens real `settings://pane/<id>` deep links through a running Settings endpoint (`org.dragonfruit.SettingsApp1`) or a `--deep-link` cold launch, and `make t232-extras-capture` records every extra in light/dark (ADR 0249). T233 landed the measured menu-bar reveal/hide transition (0 ms edge reveal, 350 ms hide grace, 160 ms slide, reduced-motion collapse; `query menubar` reports progress/offset/input-top), the width-measured status budget with notch inset and a never-clip hard fallback into the overflow menu, the active-display app-menu/status policy (`MenuBarDisplayPolicy`), and the complete Menu Bar pane control list plus the overflow explanation; tests cover reserved zones, input regions, fullscreen, hotplug, multi-display and the measured budget, and `make t233-menubar-capture` records the reveal/concealed/overflow stills (ADR 0250). T234 landed the measured Notification Center ramp: 344 pt right-edge layout with a header (Clear All + overflow Clear Notifications), 75 pt active banner-stack rows, contiguous per-app grouping, the optional 164 pt live Calendar widget stack inside the lower material band, the separate centered 1027×552 pt widget gallery (240 pt sidebar, 19 categories, 102×102 thumbnails, add/remove), banner enter/exit motion and a peeking stack count, shell-armed banner timeout, service-side `ClearAll`/`ClearHistory`, and per-display placement through the existing compositor overlay rule; `make t234-notifications-capture` records light/dark banner/Center/gallery stills and `tst_notifications`/`tst_widgetmodel` pin the layout, materials, grouping, actions, widget absence and gallery. Named Focus profiles (Do Not Disturb/Personal/Work/Sleep/custom) and allowed people landed as a presentation over the accepted three-mode policy (ADR 0058/0253): `SetFocusProfile` installs the profile's mode and its own app/person lists, a manual change selects `custom`, and the menu bar, Control Center and Settings read one `FocusPolicy()` view; allowed people use the optional `sender`/`x-dragonfruit-sender` `Notify` hint. Focus-active widget styling, the live Weather provider, inline reply and Center open/close motion remain `OPEN`. T220 landed the normative naming map (`naming-gate.json`), the user-facing/desktop/packaging name gate (`make check-naming`), the asset provenance manifest (`asset-manifest.json`, `make check-assets`), the third-party table and eight decisions (ADR 0229), and the `Recommends`/`Suggests` table (`packaging/optional-services.json`); all are in `make gates`. T221 landed the dependency-free cross-distro adapter contract (`services/distro-adapters`), the eight capability traits, the Fedora/Ubuntu/Debian/Arch stubs, the mock, the required/recommended/optional dependency/absence table, the live probe, and `make distro-contract-check` in `make gates` (ADR 0230). T222 landed the executable Settings map (`SettingsMap.qml`), the 29-entry catalog/22-shipped-pane alignment with owner/deep-link/absence metadata, row-level owners/keys/absence states, the omitted-pane table, the map/debug-view tests, and the in-app Pane & Row Owners debug view (ADR 0231). T223 landed the local Settings search index (ranked pane/section/row results with synonyms and help text), `settings://` deep links that scroll/highlight a row, the unified `SettingsRow` anatomy, the bottom-right `?` help anchor, and the search/deep-link/long-string tests and capture (ADR 0232). T224 landed the one Settings apply path (optimistic write, owner confirm/revert with an inline message, per-key generation guard), schema range/enum/type validation (`settingsSchemaInfo`), polkit `CheckAuthorization` with `AllowUserInteraction` for privileged account actions (the shell agent owns the PAM prompt; no app password field), the five-kind `SettingsAbsence`/`SettingsAbsenceNotice` policy used by the panes, and the apply/auth/UI tests plus the four-state capture (ADR 0233). T225 landed the Wi-Fi, Network, VPN, and firewall panes with the live NetworkManager service/known-network model, radio/join/forget/hidden-join writes, priority-based service order, VPN plugin discovery/add/autoconnect, the firewalld/ufw/nftables adapter (Unavailable for nftables, never fake Off), schema revision 19's prompt-policy keys, adapter/pane/absence tests, and the pane/quick-sheet/absence capture (ADR 0234). T226 landed the General group with real host/distro adapters: the `dragonfruit-general-adapter` (timedatectl, localectl, XDG autostart, systemd user/system units, restic/Pika/Timeshift detection, `df` usage, `findmnt` startup disk), the native/PackageKit update provider on the `Updates` interface, schema revision 20 (`clock.use24Hour`, `updates.auto`/`autoInstall`, `locale.*`, `backup.*`), `Settings1.Reset`, the in-place General sub-panes, and the General/Date & Time/Sharing/absence capture (ADR 0235). T227 landed the Appearance completion (highlight/sidebar/tint/scroll/motion), the Quick Settings customization pane and its live Control Center module list, the Search pane's durable T246 contract with disabled indexing rows, the Screen Saver pane reusing the lock scene, the Desktop & Stage Manager group with `desktop.showItems` consumed by the Files desktop, the capability-gated Displays advanced sheet, the pane/policy/absence tests, and the T227 capture (ADR 0236). T228 landed the macOS-to-portal privacy mapping with per-row Linux differences, the Security group (Flatpak/distro sources, read-only LUKS/hardening/extensions/Advanced), the Biometrics & Password pane (fprintd enrollment/deletion, read-only PAM detection, AccountsService password change), the optional Online Accounts pane (GOA providers/accounts/sync, hidden when GOA is absent), the shared-source bridge wiring, absence tests for portal/fprintd/GOA/LUKS/polkit, and the T228 capture (ADR 0237). T229 landed Sound's Test action, Keyboard input sources/text replacements/dictation, Mouse/Trackpad secondary click/scroll/double-click/look-up/haptics, the optional Game Controllers pane, Users & Groups' admin group and read-only directory projection, Printers discovery/add/options, Battery options/power-settings link, and the v10 pointer/keyboard-layout policy; schema revision 22 and capture (ADR 0238). T229a completed the Wallpaper/Notifications/Focus/Lock Screen panes: wallpaper Screen Saver/Clock deep links, per-app notification preferences over the service's `SetAppPreference` (allow/style/sound/badge), Focus modes/people note/schedule/shortcut with the shell `FocusSchedule`, lock-screen `lock.*` observation and logind power row, schema revision 23, and light/dark/absence capture (ADR 0239). T235 landed the measured Control Center grid (292 pt field = 4×64 + 3×12, 64/140/140×140/292 spans), the 13-module default inventory, the 308 pt expanded sheets that grow the outer panel, the 736×900 Edit Controls catalogue with a 245 pt sidebar and 22 categories, three pages, per-control span resizing, keyboard/AT-SPI menu-bar pinning, and schema revision 25 (`quickSettings.page2`/`page3`/`pageCount`/`activePage`/`perDisplay`/`spans`); `make t235-control-center-capture` records the grid/sheets/catalogue/pages/resize/absence stills and `tst_controlcenter` pins them (ADR 0254). Pointer drag-to-menu-bar, per-display surfaces and the measured open/close/expand/edit motion remain OPEN. T236 completed the Dock lifecycle menu: Hide/Show via the shared tracked app-level hide, dimmed running dot for hidden apps, Quit/Force Quit (shell-launched pid checked for liveness before SIGKILL, close request as the protocol fallback), immediate removal of a quitting temporary entry with an eight-second refusal fallback, the real Desktop & Dock Settings deep link, and click-tree/activation-fallback coverage including a vanished window (ADR 0256) | implementation rows T230–T262; T254–T259 implement the distro backends behind the T221 traits; see [tasks/0.2.0](../tasks/0.2.0/README.md), [settings-business-logic-map.md](settings-business-logic-map.md), [distro-adapter-contract.md](distro-adapter-contract.md), [interface-naming-and-licensing.md](interface-naming-and-licensing.md), [asset-manifest.json](asset-manifest.json), and [experience-schema.md](experience-schema.md) |

## 4. System status by area

| Area | State | Evidence | Notable limits |
|---|---|---|---|
| Compositor core | Implemented | `compositor/src/`, conformance suites, nested/headless/DRM backends | DRM hardware half still OPEN |
| Window model / motion | Implemented | `window/`, `window_conformance`, captures | window menu has no text (G-06) |
| Spaces / Mission Control | Implemented | `workspace/`, `overview/`, T-05 captures | paging, neighbour reveal (G-15) |
| Chrome material | Implemented (nested + single-GPU DRM) | `window/blur.rs`, `backend/{nested,drm}.rs`, `material_state`, T-20 captures, `captures/t21-native-material.txt`, `captures/t21-native-*.png`, `captures/t208-native-acceptance.md`, ADR 0234 replacement default | multi-GPU/rotated outputs fall back (G-08); cursor/overlay-plane evidence and direct scanout remain open (G-03); native replacement evidence remains OPEN (T208) while `DRAGONFRUIT_GLASS_ROLLOUT=legacy` is the rollback |
| Shell chrome | Implemented | `shell/`, Qt tests, T-17 captures | offscreen QML + `grabWindow` + `wl_shm`, no buffer scale or frame pacing (G-01) |
| Design system | Implemented | `design-system/`, gallery goldens, Tabler/Phosphor gates | system-wide Inter install is packaging-only and not wired |
| Settings | Implemented | 28 panes (Online Accounts optional), settingsd, absence matrix, executable map/debug view (T222), local search/deep links/row anatomy (T223), one apply/validate/revert path, polkit admin auth, five absence kinds (T224), Wi-Fi/Network/VPN/firewall panes with the live NetworkManager + firewall adapters (T225), General group with live time/locale/autostart/sharing/backup/storage/startup adapters, native/PackageKit updates, and `Settings1.Reset` (T226), the T227 Appearance completion / Quick Settings customization / Search / Screen Saver / Desktop & Stage / Displays-advanced panes with the live Control Center module list, the T228 Privacy & Security mapping / Security status / Biometrics & Password / optional Online Accounts panes, and the T229 Sound/Keyboard/Mouse/Trackpad/Game Controllers/Users & Groups/Printers & Scanners/Battery breadth with the v10 pointer and keyboard-layout policy, and T229a Wallpaper/Notifications/Focus/Lock Screen completion (per-app notification prefs, Focus schedule, lock-scene observation), plus T230c two-surface About (utility panel and canonical page) | Displays advanced rows (refresh, arrangement, night light, color profile, HDR, VRR) are disabled with OPEN reasons (G-13); per-display Control Center panels are T235; the Search service is T246; Stage Manager/click-wallpaper/widgets are T242/T245b; custom-chrome rows land on their section/pane; the full connection editor and nftables rule editing remain out of scope; backup engines are delegated and the update auto-policy is consumed by the running host, not a separate updater daemon |
| Files | Partial | `files-core`, C ABI, browser/views, captures | feature gap vs the Finder design (G-04) |
| Global menu | Implemented | menu-broker native tier, tray, DBusMenu focus/click consumption, hide verbs, dynamic Window menu, Services/Recent Items | Apps that export no DBusMenu still get fixed menus by design |
| System services | Implemented (read/control paths) | adapters, bridge host, 23 panes, absence matrix | real daemons/VM half is manual |
| Session / lock / idle | Partial | supervisor, units, entry, lock/PAM, idle engine | idle service and logind suspend backend absent (G-10) |
| Portals / capture / clipboard | Partial | portal crate, shell presenter, Flatpak walkthrough | live screencast producer deferred; DRM capture absent (G-03; T208 confirmed the DRM backend returns "does not implement capture yet") |
| Xwayland / compat | Partial | X11 windows, decoration, zoo matrix | XDnD runtime half (G-07), `_NET_FRAME_EXTENTS` (G-16) |
| Packaging | Design + early binary assets | `docs/design/12-packaging.md`, `packaging/README.md`, `packaging/optional-services.json` (T220 marker table), `scripts/stage-install.sh` + `scripts/package.sh` (tar.gz/DEB/RPM/Arch, native-Qt deps, per-package install smoke test), `.github/workflows/release.yml`, `services/distro-adapters` (T221 contract/stubs/mock/probe), `docs/design/distro-adapter-contract.md` | T164–T166 OPEN (G-11): official Fedora/COPR source packaging, Debian policy/source packaging, and packaged perf; T254–T259 implement the distro backends; package names/licenses verified at T259 |
| Experience evidence contract | Implemented | `docs/design/experience-schema.md`, `docs/captures/experience-{schema,manifest,thresholds}.json`, `scripts/t219-experience-schema.py`, `make experience-schema-check` | 178 explicit `OPEN` state rows; implementation lands in T222–T262 |
| Naming / licensing / assets | Implemented (policy and gates) | `docs/design/naming-gate.json`, `docs/design/asset-manifest.json`, `packaging/optional-services.json`, `make check-naming`, `make check-assets`, ADR 0229 | legal review before first public release; distro package names/licenses verified at T259 |

## 5. Gap register

Severity is about **product/architecture risk**, not implementation effort.

### P0 — fix before adding more surface area

| ID | Gap | Evidence | Why it matters | Proposed action |
|---|---|---|---|---|
| G-01 | **Shell chrome transport is software-rendered and copied per frame.** The shell forces `QT_QPA_PLATFORM=offscreen`, calls `grabWindow()` for every chrome surface, deep-copies the image, allocates a fresh `wl_shm` buffer, and commits without a buffer scale or frame callback. | `shell/src/main.cpp`, `shellcontroller.cpp` render/commit paths, `shellprotocol.cpp::commitTo`; audit §5 | This is on the critical path for every Dock sweep, menu, popover, and animation. At 4K/1.5x it is both a frame-budget and a sharpness problem; it directly undermines the "premium" claim. | New unit: render at physical scale (`set_buffer_scale`/fractional scale), reuse a small buffer pool, consume `wl_surface.frame`, and remove the full-window `grabWindow` re-render. Measure before/after. |
| G-02 | **Per-output presentation deadlines, VRR adaptation, and chrome callbacks are wired; native mixed-refresh evidence is OPEN.** T-21.2a derives each output's period, T188a records its last presentation and next deadline in a pure clock (ADR 0200), T188b arms one animation timer from the earliest due request while presenting each output only at its own deadline (ADR 0201), and T188c adapts an in-range interval only when the backend reports `VrrState::On` with a range (ADR 0202). The pure scheduler test and a headless synthetic 60+144 trace are committed; the native two-output run is an explicit `OPEN` artifact because this host has one connected DRM output and cannot restart the live session. | `compositor/src/refresh.rs`, `compositor/src/presentation.rs`, `compositor/src/frame_scheduler.rs`, `compositor/src/backend/drm.rs`, `state.rs::sync_refresh_policy`; `docs/captures/t21-vrr-mixed-refresh*.txt`; audit §9.4 | A 60 Hz output is no longer forced onto a 144 Hz tick; VRR adapts only where the backend exposes it, and chrome callbacks follow presented frames. The native mixed-refresh claim still needs hardware. | T189: per-role material floors; T191/T192: consume the presentation clock. Hardware rail: run `make t21-vrr-mixed-refresh` on a free 60+144 DRM seat. T192b adds the nested presentation benchmark (`docs/captures/t21-presentation-bench.md`, `make t21-presentation-bench`): per-scenario p50/p95/p99, 60/120/144/240 derived deadlines, Full/Reduced/Minimal tiers, and the dmabuf/shm fallback rate. T192c adds the T183-vs-v0.1.1 comparison (`docs/captures/t21-presentation-bench-compare.md`), keeps the dmabuf/shm fallback rate separate, and records the native scenario/mode rows `OPEN` with the seat-probe blocker (`docs/captures/t21-presentation-bench.open.txt`). It also fixes the T192b full-rail Mission Control teardown crash (producer-owned windows; slot targets destroyed before `QQuickRenderControl::invalidate()`); the overview-included verification run completes with zero shm commits and zero copy bytes (`docs/captures/t21-presentation-bench-fullrail.txt`), while the measured run keeps the overview on the counted fallback row so the fallback rate stays visible. Native mixed-refresh evidence remains OPEN. |
| G-03 | **Native visual evidence now exists; cursor/overlay and direct scanout remain open.** T191b read the real DP-1 KMS scanout buffer with a dev-only `pidfd_getfd` + dma-buf readback (not screencopy, not nested) and committed light/dark Dock/menu stills over live content plus menu, drawer, Control Center, OSD, notification, Dock popover, magnification, Mission Control, lock, and fullscreen stills. The Dock/menu bar measurably brighten when a white client is moved behind them (dark menu bar mean 55.0→113.2; Dock 45.4→161.6; light Dock 68.7→183.5). The readback covers the primary plane only, so the hardware cursor and overlay planes are not in the stills; `direct_scanouts=0` (47 candidates) means direct scanout remains T190d/T192. T208 re-verified the native baseline on `seat0`/`card1`/DP-1 (KMS stills, live-backdrop deltas, idle/load p50/p95/p99, resources) and confirmed fullscreen candidates 4019→11261 with `direct_scanouts=0`; the running build is pre-T195/T196, so the replacement native row, native lock-exit and native rollback cycle remain OPEN. | `docs/captures/t21-native-*.png`, `docs/captures/t21-native-material.txt`, `docs/captures/t208-native-acceptance.md`, `docs/captures/t208-native-stats.txt`, `docs/captures/t208-native-*.png`; `scripts/t208-kms-readback.c`, `scripts/t208-shell-fullscreen.c`; `shell/src/shellcontroller.cpp` capture seams; audit §P6 | The premium material, live sampling, and HiDPI chrome are now visually evidenced on the rail users run. Cursor sharpness and overlay planes still cannot be signed off from these stills, and the rollback render still needs a session cycle. | Keep the native stills current; add a cursor/overlay readback or external photograph for the cursor path, and run the T190d direct-scanout trace on a free seat. ADR 0234 enabled the replacement glass default; the free-seat replacement/rollback rows remain OPEN. Keep nested stills labeled reference. |

### P1 — architecture/feature gaps that block a credible release

| ID | Gap | Evidence | Why it matters | Proposed action |
|---|---|---|---|---|
| G-04 | **Files is a browser subset, not the documented Finder MVP.** No tabs, no New Window (menu row is disabled), no copy/move/paste, no undo, no per-item progress/conflicts, no Open With, no Files menu publication. The Rust ops exist but the UI does not expose them. | `apps/files/FilesShell.qml` `nodeMenu`/`backgroundMenu`; no tab model; `FilesDirectoryModel`; `services/files-core/src/ops.rs` | The Files design promises Finder semantics that are not there. A user will find the first missing operation immediately. | Cut a Files-completeness track: tabs/windows first, then copy/move/paste and undo, then progress/conflicts, then Open With and menu publication. |
| G-05 | **Resolved by T230.** The shell now maps focus to the native/DBusMenu registration, reads a fresh `WindowMenu`, routes `dbusmenu:<id>` clicks through `WindowMenuEvent`, and executes Hide/Hide Others/Show All as tracked compositor minimize/restore. | `shell/src/shellcontroller.cpp`, `shell/src/appindexclient.cpp`, `services/app-index/src/dbus.rs`, `services/menu-broker/src/model.rs`; ADR [0240](adr/0240-menu-bar-focus-dbusmenu-and-hide-verbs.md) | The "compatible Linux apps" tier is real for apps that register a DBusMenu. | None. A toolkit that exports no DBusMenu still gets the fixed menus by design. |
| G-06 | **The compositor window menu is label-less.** It draws token-colored rows with no text or icons; `window/menu.rs` says the visual pass is intentionally flat and label-less. T-04 never added the labels the design promised. | `compositor/src/window/menu.rs` header, `render_elements` | Right-clicking a titlebar opens an unlabeled menu. This is a first-day usability defect, not polish. | Add a text renderer or move the menu to a trusted shell surface; do not leave a menu that cannot be read. |
| G-07 | **Cross-boundary XDnD is still a model + codec.** The X11 connection/runtime half is not implemented. | ADR 0099, `compositor/src/xdnd.rs`, T-17 waiver | Dragging a file from an X11 app to a Wayland app (and back) is a common workflow and a visible compatibility hole. | Implement the bridge window/atom/selection half on a dedicated X11 connection; keep the existing conformance codec tests. |
| G-08 | **Sampled material is single-GPU, normal-transform only.** A multi-GPU seat disables the sampled path; a rotated output falls back to feather; the `route=copy` plan is not executed. The native single-GPU/normal-transform DP-1 4K/144 path is now confirmed active by T191a (`path=gpu`, `blur_draws=2243`, `backdrop_skipped=0`); the hybrid/rotated cases remain the gap. | `compositor/src/backend/drm.rs` device-count guard and transform guard; `multi_gpu::plan_scene_routes`; `docs/captures/t21-native-material.txt` | Hybrid laptops and rotated displays lose the signature material. | Keep the local route; add an explicit cross-GPU copy or a reported fallback, then transform-aware sampling. Never sample across GPUs. |
| G-09 | **Resolved.** The session plan and systemd units provision the Files desktop: `dragonfruit-session --print-env` mints the role-scoped desktop token, the entry imports it, the compositor writes `<socket>.desktop-launch-token`, and `dragonfruit-files-desktop.service` starts `dragonfruit-files --desktop` after the socket/settings gate. | `services/session/src/{env,plan}.rs`, `services/session/units/dragonfruit-files-desktop.service`, `services/session/tests/{units,environment,kill_matrix}.rs` | Desktop icons now survive a real login, not only `make dev`/`make demo`. | None. The desktop restarts in place like the shell; its token is reissued after a crash. |
| G-10 | **Session power and idle are model-level.** System-menu Sleep/Restart/Shut Down/Log Out remain incomplete; there is no production idle service; `SuspendBackend` has no concrete logind implementation. | `shell/src/shellcontroller.cpp` action handler; `services/session/src/{idle,suspend}.rs`; no idle unit | A desktop that cannot reliably lock/suspend/log out is not a daily driver. | Wire logind power/session actions, ship the idle service on `ext-idle-notify`/inhibitors, and implement the logind suspend backend. |
| G-11 | **Packaging and the hardware rail are open.** Fedora/Debian packaging is still a README; driver matrix, packaged perf, DRM full loop, and baseline perf are OPEN. | `packaging/*/README.md`, ROADMAP T163–T168 | Without packages and a driver matrix there is no installable, verifiable product. | Keep the hardware rail; cut packaging as real work, not a doc placeholder. |

### P2 — deliberate deferrals or polish gaps

| ID | Gap | Evidence | Proposed action |
|---|---|---|---|
| G-12 | **Persistent-chrome floor policy, protocol, shell application, and refresh-aware hysteresis landed; native flicker fix hardened in T-21.5.** `MaterialRole` classifies `Chrome`/`Dock` as persistent, `EffectiveMaterial::resolve_for_role` keeps them at `Reduced` with the sampled pass on even when the global controller selects `Minimal`, and `df_output` v4 carries the role-keyed state (ADR 0203). The render path consumes it, and `ShellController` applies all four roles through one `MaterialPolicy` with the milky fallback (ADR 0204); the popup family uses the same state through `design-system/Material.qml` and records its fallback reason (ADR 0205). Every downgrade now latches its boundary, with a per-tier cost guard, and panel blur is cached by scene/source/spec/scale (ADR 0211). Rounded blur spans now translate element-local damage into span-local scissor coordinates, fixing the hard-edged raw-background patches seen while moving over the Dock (ADR 0212). The 12/180 hysteresis is a 200 ms/3 s wall-clock window with the 10 s latched recovery, tested at 60/120/144 Hz and traced in `docs/captures/t21-degrade-load.txt` (ADR 0206). | `window/degrade.rs`, `window/backdrop.rs`, `window/blur.rs`, `window/material_cache.rs`, `render.rs`, `shell/mod.rs`, `shell/src/materialpolicy.*`, `design-system/Material.qml`; ADR 0203/0204/0205/0206/0211/0212 | Keep the load trace current when the ladder or budget policy changes; region tiling remains the next 4K performance step. |
| G-13 | Night light, color management/HDR, tearing-control, and content-type-specific scanout policy are not implemented. VRR is now consumed from the backend on the DRM rail (T188c, ADR 0202): a connector that can toggle without a modeset is enabled and its state is reported; nested/headless report `unsupported`. There is still no VRR UI or settings policy, the private `set_vrr` request remains an ack, and `wp-content-type-manager-v1` is advertised but not consumed by policy. T227 exposes scale/rotation/brightness as live Displays controls and disables refresh, arrangement, night light, color profile, HDR, and VRR with explicit OPEN reasons; the capability table is `libs/settings-client/displaycapabilities.*` and is asserted by `shell/tests/tst_displayspolicy.cpp`. | `compositor/src/backend/drm.rs` connector VRR probe; `compositor/src/shell/mod.rs` request handler; `apps/settings/DisplaysPane.qml`; `libs/settings-client/displaycapabilities.*`; `content_type_state` registration only; no `xx-color-management`/tearing code | Implement the missing compositor/colord half before promoting any disabled row; do not advertise a Displays capability that is not there. |
| G-14 | Files folder watcher is not wired to the Files facade; thumbnails are local-only (images, PDF/video via optional helpers), not the full freedesktop pipeline for archives/remote files. | `services/files-core/src/watch.rs`, `apps/files/FilesDirectoryModel.cpp`, `FilesThumbnailProvider.cpp` | Wire `apply_watch`/delta reconciliation; document helper-dependent thumbnail coverage. |
| G-15 | Mission Control paging at the readable floor and neighbour-Space reveal are deferred. | `compositor/src/overview/grid.rs` comment; T-05 docs | Add paging and neighbour reveal when the grid is used with many windows. |
| G-16 | `_NET_FRAME_EXTENTS` is not published to X11 clients. | no code in `compositor/src/xwayland.rs`; 05 doc | Publish frame extents so X11 clients can see the SSD titlebar strip. |
| G-17 | Spotlight/search, print job management/driver installation, controller remapping, NVIDIA-specific validation, HDR, and live PipeWire screencast remain explicit backlog. The optional GOA adapter landed in T228; provider OAuth and the GOA sign-in UI stay delegated to GOA. | T-17 post-gate backlog, `docs/design/07-system-integration.md`, ADR 0237, ADR 0238 | Keep them named; do not let them creep into unrelated units. |
| G-18 | Documentation drift: stale DRM/feather claims, desktop icons "later", Files "cxx-qt"/GVfs/thumbnails/file-opening claims, Control Center five-tile status, Displays VRR/color claims, duplicate ADR 0123, broken links. | this audit | Fixed in the same change as this file; see §7. |
| G-19 | **Shell chrome is not exported to AT-SPI.** The offscreen QPA has no accessibility backend, so the menu bar, Dock, Control Center, OSD, and overlays keep per-component `Accessible.*` semantics but do not appear on the accessibility bus. | ADR 0154, `shell/src/main.cpp` | Accepted boundary for now, but a screen-reader user cannot inspect the chrome. Revisit when the shell transport is reworked (a Wayland/AT-SPI bridge may fall out of that change). |
| G-20 | **No shipped input-method/IME process.** The compositor advertises `text-input`/input-method protocols, but no first-party IME UI or integration ships; CJK/complex-input users depend on a host IME. | `compositor/src/state.rs` protocol registration; no IME service | Decide whether to integrate an existing IME (ibus/fcitx5) or ship a minimal candidate window; do not advertise IME support as complete until one is present. |
| G-21 | **The total desktop experience is not yet specified as one contract.** Settings rows are now mapped to owners in the executable map (T222) and the shipped rows are searchable/deep-linkable (T223), but the panes that implement the new rows are not complete; menu-bar extras are complete for the six core extras (T232), while visibility/overflow, Notification Center, and Control Center customization remain, Dock behaviour/motion has named gaps, Overview/workspaces paging and neighbour reveal are deferred, the app switcher is app-mode-focused, Search/Stage/tiling/Preview are design-only, HUDs/sound/cursor are partial, and cross-distro adapters/packaging are not complete. | This audit; T-22 inventory; [22-total-experience.md](22-total-experience.md); [settings-business-logic-map.md](settings-business-logic-map.md); [distro-compatibility.md](distro-compatibility.md) | Execute T224–T262 in order. Every Settings row has an owner/absence path in `SettingsMap.qml`; every surface gets a capture and a keyboard/AT-SPI/reduced-motion path; every distro gets a build/install/smoke row. Do not add new Settings rows or surface chrome outside the map without a scope update. |

## 6. Decisions made in code, now recorded

These were implementation decisions that had no design record at the audit
point. They are now either captured in an ADR or reconciled in a topic doc.

| Decision | Where it lives now |
|---|---|
| The Dock perimeter is one uniform specular rim plus inner glow on every edge and corner; the corner lens is inside-rim and gated by `4·nx²·ny²`; the QML fallback uses the same full perimeter with three glow strokes and no outer shadow. | ADR [0255](adr/0255-dock-uniform-perimeter-and-corner-gate.md), [04-shell.md](04-shell.md), [10-design-system.md](10-design-system.md) |
| Native DRM sampled material is default on single-GPU, normal-transform outputs; multi-GPU/rotation fall back to feather; env opt-outs exist; `material_state` is the shell's source and a failed material frame disables the path for the session. | [ADR 0189](adr/0189-drm-sampled-material-default-and-fallbacks.md), [02-compositor.md](02-compositor.md) |
| Files thumbnails use the freedesktop cache and optional local helpers; default-handler opening resolves the MIME default, then an installed handler, then `xdg-open`/`gio`; the offscreen desktop's child drops `QT_QPA_PLATFORM`. | [ADR 0190](adr/0190-files-thumbnails-and-default-handler-opening.md), [09-files.md](09-files.md) |
| Tabler is the line-glyph library; Phosphor remains the filled Settings/app-tile set; `Icon.qml` is a compatibility facade. | ADR [0185](adr/0185-tabler-line-icons.md), [10-design-system.md](10-design-system.md) |
| Menu rows share one anatomy (`MenuRow`) and one shortcut renderer (`ShortcutHint`). | ADR [0187](adr/0187-menu-row-anatomy-and-shortcut-glyphs.md), [10-design-system.md](10-design-system.md) |
| Dock reference metrics/zoom profile are the measured Tahoe profile, not the original first-pass values. | ADR [0186](adr/0186-dock-reference-retune-profile-fit-and-states.md), [04-shell.md](04-shell.md) |
| Window activation restacks and honors activation requested before the first buffer; Files default-handler opening resolves MIME defaults before `xdg-open`. | [02-compositor.md](02-compositor.md) window-model note; ADR 0190 |
| ADR `0123` was duplicated (Sound pane and thin/full dev session). The dev-session ADR is renumbered to `0188`; the Sound ADR keeps `0123`. | [ADR 0188](adr/0188-thin-demo-and-full-dev-session.md) |

## 7. Documentation drift fixed in this pass

The topic docs were edited so their claims match the tree:

- `02-compositor.md`: DRM sampled material is implemented and default on the
  supported path; multi-GPU/rotation limits are explicit; VRR/night light/color
  management/tearing-control are marked as not implemented; the shell transport
  and fixed-refresh gaps are linked.
- `04-shell.md`: desktop icons are implemented; Control Center is the full
  T-15 tile set (not five); the Trash source is files-core (not a GVfs mount);
  system-menu power/About actions are recorded as stubs.
- `05-window-decorations.md`: the window menu is still label-less; the
  T230b chrome style table/traffic geometry, material, long-title/RTL `OPEN`
  rows, and `_NET_FRAME_EXTENTS` status are explicit.
- `06-global-menu.md`: DBusMenu shell consumption/click routing and the
  hide/show verbs are marked partial.
- `08-settings.md`: Displays is scale/rotation only; VRR/night light/color are
  not claimed as shipped.
- `09-files.md`: the bridge is the hand-written C ABI; GVfs is the target, not
  the shipping backend; thumbnails/file-opening/desktop icons are recorded as
  implemented; tabs/copy/move/undo/Open With/menu publication are recorded as
  open.
- `10-design-system.md`: component inventory matches the module; the
  system-wide Inter install is marked packaging-only.
- `11-session-and-dev-workflow.md`: wallpaperd is in the unit list; lock-screen
  keys are written by the pane; the missing idle service/logind suspend
  backend is explicit.
- `12-packaging.md`: packaging is marked design-only until T164–T166.
- `14-risks.md`: the current engineering risks (transport, scheduling, native
  verification, Files, compatibility) are added.
- `advisor-briefing.md`: marked as a historical briefing; its DRM-gap section
  points at the implementation record.
- `architecture-audit-native-rendering.md`: marked P0/P1/P2 implemented;
  P3/P4/P5/P6 remain open.
- `proposals/native-drm-dock-glass.md`: marked superseded by the audit and the
  implementation.
- `proposals/dock-liquid-glass-perimeter.md`: proposed replacement for the
  Dock's directional perimeter; the measured flat band and multi-scale blur
  stay unchanged.
- `tracks/19`, `tracks/20`: icon split and DRM material close-out corrected.
- Broken links in ADRs 0090/0120/0176 and `02-compositor.md` fixed.
- ADR `0123-thin-demo-and-full-dev-session.md` renumbered to `0188`.

## 8. Architecture assessment

### Boundary review

The ownership boundary is consistently followed, with a few exceptions worth
watching:

| Boundary | Verdict | Notes |
|---|---|---|
| Windows/focus/stacking/Spaces/outputs → compositor | Clean | The shell projects, never owns; activation requests go through one compositor authority. |
| App identity → app-index | Clean | The compositor publishes raw `app_id`/`WM_CLASS`; the shell is the forwarder; no `.desktop` scan in the compositor. |
| Desktop settings → settingsd | Clean | Shell/apps observe `org.dragonfruit.Settings1`; compositor policy arrives through the shell bridge. |
| Menu model ownership → menu-broker | Clean | The native tier is owned; the DBusMenu projection is owned by app-index and stored by the broker; the shell consumes the resolved model and executes the hide verbs through compositor window operations. The shell's fixed-menu fallback mirrors the same rules for a broker-absent session by design. |
| Notification queue/Focus → notification service | Clean | Control Center and menu bar share one policy source. |
| Hardware state → host daemons | Clean | Adapters observe/request; absence is a first-class state. |
| Desktop items → Files process | Clean on the layer boundary | The compositor grants a role-scoped background layer; Files owns the content. The session does not start the process yet. |
| Lock → compositor | Clean and fail-secure | PAM is a helper; no crash path unlocks. |
| Chrome presentation → shell | **Coupling risk** | The shell owns layout/interaction, but its offscreen QML → `wl_shm` transport is the performance/quality bottleneck. This is a transport problem, not an ownership problem. |

### Decisions worth revisiting

1. **Compositor-owned window menu with no text renderer.** The model and
   command routing are good, but a label-less menu cannot ship. Either add a
   text renderer or move presentation to the trusted shell while keeping the
   model compositor-owned. Do not build a second menu model.
2. **Offscreen QML transport.** The current path exists because the shell owns
   its own protocol connection. The audit's recommendation stands: measure
   first, then either use the Qt Wayland platform plugin, `QQuickRenderControl`
   with a GPU buffer path, or keep the offscreen path with buffer reuse and
   physical-scale rendering. Do not add a second UI renderer.
3. **Control Center density.** The T-15 breadth wave reached 18 tiles plus
   clipboard. The no-scroll rule is holding, but the panel is now a product
   surface that deserves a deliberate hierarchy rather than another tile.
4. **Per-output policy duplication.** `display.scale`/`display.rotation` are
   settingsd keys forwarded by the shell; VRR/night light/color are protocol
   fields with no implementation. Decide whether the Displays pane is a
   two-control MVP or a real output-management surface before adding keys.
5. **Desktop process lifecycle — resolved.** The desktop is now a first-class
   session service (`dragonfruit-files-desktop.service`) with its own
   role-scoped token, restart policy, and crash-isolation tests.

### Strengths

- **One owner per state class.** Windows/Spaces/outputs in the compositor,
  identity in app-index, settings in settingsd, menu ownership in menu-broker,
  notifications in the notification service. This is the right shape for a
  multi-process desktop and it is consistently followed.
- **Live surfaces, not screenshots.** Mission Control, the app switcher, and
  lifecycle motion transform the real buffers. That is a genuine differentiator
  over overview tools that scrape or thumbnail.
- **Cross-process visual parity.** One token source generates QML and Rust;
  compositor-drawn SSD chrome and QML chrome read the same values. This is
  unusually strong discipline for a desktop project.
- **Honest degradation.** Full/Reduced/Minimal, absence-as-a-state, fail-secure
  lock, restartable services, and explicit OPEN rows in the gate report. The
  project does not pretend a missing daemon or missing hardware is fine.
- **No capture shortcuts.** Effects are render passes over live buffers; the
  portal is the only capture path. That is both an architectural and an IP
  safeguard.

### Weaknesses

- **The shell is the weak link in the presentation story.** The compositor has
  a modern GPU material path; the shell still hands it software-rendered
  `wl_shm` frames with no scale or pacing. This is the single most important
  thing to fix for perceived quality.
- **Scheduling is 60 Hz-shaped.** The animation clock, degrade budget, and
  timers all assume 16 ms. High-refresh and VRR hardware will expose it.
- **Native evidence is missing.** The team can prove nested behavior but not
  the DRM pixels users will see.
- **Files is behind its own design.** The browsing core is good; the user-facing
  Finder semantics are not there yet.
- **Compatibility closure is incomplete.** XDnD and `_NET_FRAME_EXTENTS` are
  exactly the kind of gaps users interpret as "Linux desktop jank"; the global
  menu's DBusMenu focus/click and hide verbs are now closed (T230).
- **A few stubs are still presented as features.** System-menu power actions,
  About, and the label-less compositor window menu are the most visible.
- **Accessibility and IME are incomplete at the desktop level.** Apps are on
  AT-SPI, but shell chrome is not; the compositor advertises input-method
  protocols without a shipped IME. These are P2 gaps (G-19/G-20), not reasons
  to claim parity with mature desktops.

### Competitive read

GNOME and KDE win on application breadth, settings breadth, and hardware
coverage; we will not beat them there in the near term. The credible
differentiator is **coherence of the live-surface desktop**: one compositor
that owns Spaces, Mission Control, the app switcher, the Dock hand-off, and the
material, with first-party apps that share one design system. That story is
already real in the tree. It becomes competitive only when the shell transport
and scheduling are fixed, native hardware is verified, and Files/global-menu
compatibility stop producing obvious dead ends. Adding more settings panes
before those are fixed would widen the surface without improving the product.

## 9. Recommended next sequence

This is a proposal, ordered by architectural leverage. The v0.1.1 task files
are now cut from it.

> **v0.1.1 task set:** the executable form of P0-A/P0-B/P0-C plus the material
> cost/floor follow-ups is in
> [`docs/tasks/0.1.1/`](../tasks/0.1.1/README.md) (T183–T193), under
> [Track 21](tracks/21-presentation-foundation.md).

### P0 — presentation foundation (next)

1. **P0-A Shell transport + HiDPI.** Render chrome at the output's physical
   scale, set the buffer scale/fractional scale, reuse buffers, consume frame
   callbacks, and stop re-rendering the whole offscreen window for every
   commit. Acceptance: before/after copy-count and frame-time report; crisp
   text/icons at 1.5x; no behavior change to the private layer protocol.
2. **P0-B Refresh-aware scheduling.** Per-output period/deadline, animation
   clock and degrade budget use it, VRR state included. Acceptance: 60/120/144/
   240 Hz table; p95/p99 and missed-deadline counts; idle stays zero-wakeup.
3. **P0-C Native verification.** Trusted capture/readback for DRM; commit
   native stills for Dock/menu bar/window at 1x/1.5x/2x, Full/Reduced. No
   nested screenshot may be labeled native proof.
4. **P0-D Total experience (v0.2.0).** Execute T209–T262: capture the reference,
   map every Settings row to an owner, then complete the menu bar/extras, Dock,
   Overview/workspaces, switcher, Search, drawer, motion/HUDs, and cross-distro
   integration. This is the product-closure track; it does not replace the
   presentation foundation above. Executable sequence:
   [v0.2.0/ROADMAP.md](../v0.2.0/ROADMAP.md) · task files:
   [tasks/0.2.0/](../tasks/0.2.0/README.md).

### P1 — close the visible dead ends

4. **P1-A Files completeness.** Tabs/New Window, copy/move/paste, undo,
   progress/conflicts, Open With, Files menu publication, watcher wiring.
5. **P1-B Compatibility closure.** DBusMenu focus/click routing, hide/show
   verbs, XDnD runtime, `_NET_FRAME_EXTENTS`.
6. **P1-C Session completion.** Desktop-process unit/token, production idle
   service, logind suspend backend, power/About actions.
7. **P1-D Window-menu text.** A readable compositor menu or a trusted shell
   menu surface.

### P2 — breadth and hardening

8. Multi-GPU/rotated material and a persistent-chrome quality floor.
9. Packaging (Fedora/Debian), driver matrix, packaged perf, DRM full loop.
10. VRR/night light/color management/tearing-control if the Displays design is
    kept; otherwise remove them from the design's shipped claims.
11. Shell AT-SPI export and a shipped input-method/IME path.
12. Spotlight/printer breadth/live PipeWire screencast, as explicit post-gate
    work; the optional GOA adapter landed in T228.

## 10. Keeping this current

- Every ADR that changes a shipped behavior updates the topic doc and the
  relevant row here.
- Every "done" task that leaves a hardware/human/automation half says so in
  the same row; do not use a bare checkbox as evidence.
- Every design doc claim about a protocol, setting, or visual path links to the
  code or test that proves it, or is marked design-only.
- Re-run the link/ADR-number check in `scripts/` (or the commands in
  `docs/design/README.md`) after doc edits; the audit found one duplicate ADR
  and six broken local links.
