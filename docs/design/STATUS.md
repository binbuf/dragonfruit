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
  global-menu DBusMenu clicks, VRR/night light/color management, system-menu
  power actions, Control Center's original five tiles).
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
| T-09 Settings Wave 1 | Implemented | app shell, live apply, Appearance/Wallpaper/Desktop & Dock/Displays, menu model, absence matrix | Displays is scale/rotation only (G-13) |
| T-10 Files MVP | **Partial** | files-core listing/sort/ops/trash/watch, C ABI, sidebar/favorites/volumes, browser/views/selection/context menus, optimistic rename/new-folder/trash, file opening/thumbnails, perf budgets, Dock Trash, reveal, captures | tabs/new window, copy/move/paste, undo, progress/conflicts, Open With, Files menu publication, watcher wiring (G-04, G-14) |
| T-11 Control Center + notifications | Implemented | notification service/policy/actions, Control Center, OSD, a11y | some Control Center links/actions remain follow-ups |
| T-12 Session + lock + idle | **Partial** | session plan/supervisor/units/entry, lock protocol/PAM/kill-resistance, idle engine/inhibitors, suspend model, policy keys | no production idle service, no concrete logind suspend backend, power actions are stubs (G-10) |
| T-13 Portals + capture + clipboard | **Partial** | portal backend, settings/globalshortcuts/filechooser/screenshot/screencast, clipboard history, polkit, Flatpak walkthrough | live PipeWire screencast producer is still the stills fallback; DRM screenshot capture absent (G-03) |
| T-14 Global menu + app index + compat | **Partial** | app-index identity/events/subscriptions, menu-broker native tier, tray, DBusMenu projection, XDnD model, zoo | DBusMenu click routing and shell consumption, hide/hide-others/show-all, XDnD runtime half (G-05, G-07) |
| T-15 System services + Settings 2–3 | Implemented | 22 shipped panes, adapters, Control Center tiles, absence matrix | real-daemon/VM half is not automated |
| T-16 Platform polish + packaging | **Partial** | per-output chrome/placement, hotplug, integer Xwayland, fractional chrome, a11y audit, magnifier, i18n, crash matrix, pointer timestamps | driver matrix, Fedora/Debian packaging, packaged perf, DRM full loop, baseline perf (G-11) |
| T-17 Premium gate | Partial / human-open | nested loop/navigation/Flatpak captures, visual floor, robustness, leak/lock, sign-off report | DRM full loop and baseline perf OPEN; human unfamiliar-user verdict batched |
| T-18 Wallpaper content provider | Implemented | wallpaperd, shipped default, provider cache, pane collections/attribution, absence | — |
| T-19 Desktop affordances | Implemented | Tabler line + Phosphor fill icon split, Settings category tiles, menu-bar/Dock/app icons, Applications drawer, desktop items | desktop process is not provisioned by the systemd session (G-09) |
| T-20 Tahoe liquid-glass material | **Partial** | scene texture, Kawase blur, glass shader, all chrome roles, DRM sampled path, effective material event, software fallback, tolerance captures | native hardware acceptance, multi-GPU/rotation, quality floor, refresh scheduling (G-08, G-12, G-02) |
| T-21 Presentation foundation (v0.1.1) | **Proposed** | task set T183–T193: shell transport/HiDPI, per-output refresh scheduling, material cost/floor, native evidence | all rows; see [tasks/0.1.1](../tasks/0.1.1/README.md) |

## 4. System status by area

| Area | State | Evidence | Notable limits |
|---|---|---|---|
| Compositor core | Implemented | `compositor/src/`, conformance suites, nested/headless/DRM backends | DRM hardware half still OPEN |
| Window model / motion | Implemented | `window/`, `window_conformance`, captures | window menu has no text (G-06) |
| Spaces / Mission Control | Implemented | `workspace/`, `overview/`, T-05 captures | paging, neighbour reveal (G-15) |
| Chrome material | Implemented (nested + single-GPU DRM) | `window/blur.rs`, `backend/{nested,drm}.rs`, `material_state`, T-20 captures | multi-GPU/rotated outputs fall back; no native pixel proof (G-03, G-08) |
| Shell chrome | Implemented | `shell/`, Qt tests, T-17 captures | offscreen QML + `grabWindow` + `wl_shm`, no buffer scale or frame pacing (G-01) |
| Design system | Implemented | `design-system/`, gallery goldens, Tabler/Phosphor gates | system-wide Inter install is packaging-only and not wired |
| Settings | Implemented | 22 panes, settingsd, absence matrix | Displays breadth, VRR/night light/color (G-13) |
| Files | Partial | `files-core`, C ABI, browser/views, captures | feature gap vs the Finder design (G-04) |
| Global menu | Partial | menu-broker native tier, tray, DBusMenu projection | DBusMenu click/shell consumption, hide verbs (G-05) |
| System services | Implemented (read/control paths) | adapters, bridge host, 22 panes, absence matrix | real daemons/VM half is manual |
| Session / lock / idle | Partial | supervisor, units, entry, lock/PAM, idle engine | idle service and logind suspend backend absent (G-10) |
| Portals / capture / clipboard | Partial | portal crate, shell presenter, Flatpak walkthrough | live screencast producer deferred; DRM capture absent (G-03) |
| Xwayland / compat | Partial | X11 windows, decoration, zoo matrix | XDnD runtime half (G-07), `_NET_FRAME_EXTENTS` (G-16) |
| Packaging | Design-only | `docs/design/12-packaging.md`, empty `packaging/*/README` | T164–T166 OPEN (G-11) |

## 5. Gap register

Severity is about **product/architecture risk**, not implementation effort.

### P0 — fix before adding more surface area

| ID | Gap | Evidence | Why it matters | Proposed action |
|---|---|---|---|---|
| G-01 | **Shell chrome transport is software-rendered and copied per frame.** The shell forces `QT_QPA_PLATFORM=offscreen`, calls `grabWindow()` for every chrome surface, deep-copies the image, allocates a fresh `wl_shm` buffer, and commits without a buffer scale or frame callback. | `shell/src/main.cpp`, `shellcontroller.cpp` render/commit paths, `shellprotocol.cpp::commitTo`; audit §5 | This is on the critical path for every Dock sweep, menu, popover, and animation. At 4K/1.5x it is both a frame-budget and a sharpness problem; it directly undermines the "premium" claim. | New unit: render at physical scale (`set_buffer_scale`/fractional scale), reuse a small buffer pool, consume `wl_surface.frame`, and remove the full-window `grabWindow` re-render. Measure before/after. |
| G-02 | **Per-output presentation deadlines and VRR are not wired yet.** The shared clock and degrade budget follow the fastest active output's period (T-21.2a, ADR 0199), and T-21.2b now records each output's last presentation and next deadline in a pure clock (ADR 0200), but each output is still presented on one global tick and VRR is carried only. | `compositor/src/refresh.rs`, `compositor/src/presentation.rs`, `state.rs::sync_refresh_policy`; audit §9.4 | A mixed 60+144 Hz desktop still renders both outputs on the fastest tick; VRR cannot adapt yet. | T188b: per-output frame requests and earliest-deadline arming; T188c: VRR and mixed-refresh evidence; T189: per-role material floors. |
| G-03 | **No native visual verification.** DRM capture explicitly fails (`"the DRM backend does not implement capture yet"`); nested captures are the only visual evidence. | `compositor/src/backend/drm.rs` capture guard; audit §P6; T-17 report | The premium material, HiDPI, and cursor path cannot be signed off on the rail users actually run. Nested screenshots are not native proof. | Trusted, explicitly enabled KMS readback or an external capture procedure; record native evidence for the Dock/menu bar at 1x/1.5x/2x and Full/Reduced. |

### P1 — architecture/feature gaps that block a credible release

| ID | Gap | Evidence | Why it matters | Proposed action |
|---|---|---|---|---|
| G-04 | **Files is a browser subset, not the documented Finder MVP.** No tabs, no New Window (menu row is disabled), no copy/move/paste, no undo, no per-item progress/conflicts, no Open With, no Files menu publication. The Rust ops exist but the UI does not expose them. | `apps/files/FilesShell.qml` `nodeMenu`/`backgroundMenu`; no tab model; `FilesDirectoryModel`; `services/files-core/src/ops.rs` | The Files design promises Finder semantics that are not there. A user will find the first missing operation immediately. | Cut a Files-completeness track: tabs/windows first, then copy/move/paste and undo, then progress/conflicts, then Open With and menu publication. |
| G-05 | **Global-menu compatibility is incomplete.** The DBusMenu projection is stored by app-index/menu-broker, but the shell does not consume it; `dbusmenu:<id>` rows only log. Hide/Hide Others/Show All are enabled in the model but the shell actions are stubs. | `services/app-index/src/menubridge.rs`, `services/menu-broker/src/model.rs`, `shell/src/shellcontroller.cpp` action handler | The advertised "compatible Linux apps" tier is not real for the common DBusMenu case; the fixed app menu's live verbs are cosmetic. | Wire shell focus→registration→menu, route `dbusmenu:<id>` back through `WindowMenuEvent`, and implement the three visibility verbs as compositor window operations. |
| G-06 | **The compositor window menu is label-less.** It draws token-colored rows with no text or icons; `window/menu.rs` says the visual pass is intentionally flat and label-less. T-04 never added the labels the design promised. | `compositor/src/window/menu.rs` header, `render_elements` | Right-clicking a titlebar opens an unlabeled menu. This is a first-day usability defect, not polish. | Add a text renderer or move the menu to a trusted shell surface; do not leave a menu that cannot be read. |
| G-07 | **Cross-boundary XDnD is still a model + codec.** The X11 connection/runtime half is not implemented. | ADR 0099, `compositor/src/xdnd.rs`, T-17 waiver | Dragging a file from an X11 app to a Wayland app (and back) is a common workflow and a visible compatibility hole. | Implement the bridge window/atom/selection half on a dedicated X11 connection; keep the existing conformance codec tests. |
| G-08 | **Sampled material is single-GPU, normal-transform only.** A multi-GPU seat disables the sampled path; a rotated output falls back to feather; the `route=copy` plan is not executed. | `compositor/src/backend/drm.rs` device-count guard and transform guard; `multi_gpu::plan_scene_routes` | Hybrid laptops and rotated displays lose the signature material. | Keep the local route; add an explicit cross-GPU copy or a reported fallback, then transform-aware sampling. Never sample across GPUs. |
| G-09 | **The Files desktop process is not provisioned in a real session.** The dev tool writes the desktop token, but the systemd plan/units do not start `dragonfruit-files --desktop`. | `services/session/src/plan.rs`, `services/session/units/`, `docs/design/09-files.md` T-19.3 note | Desktop icons work in `make dev` but not after a real login. | Add the desktop service/unit and token hand-off to the session plan; assert it in the session tests. |
| G-10 | **Session power and idle are model-level.** System-menu About/Sleep/Restart/Shut Down/Log Out log only; there is no production idle service; `SuspendBackend` has no concrete logind implementation. | `shell/src/shellcontroller.cpp` action handler; `services/session/src/{idle,suspend}.rs`; no idle unit | A desktop that cannot reliably lock/suspend/log out is not a daily driver. | Wire logind power/session actions, ship the idle service on `ext-idle-notify`/inhibitors, and implement the logind suspend backend. |
| G-11 | **Packaging and the hardware rail are open.** Fedora/Debian packaging is still a README; driver matrix, packaged perf, DRM full loop, and baseline perf are OPEN. | `packaging/*/README.md`, ROADMAP T163–T168 | Without packages and a driver matrix there is no installable, verifiable product. | Keep the hardware rail; cut packaging as real work, not a doc placeholder. |

### P2 — deliberate deferrals or polish gaps

| ID | Gap | Evidence | Proposed action |
|---|---|---|---|
| G-12 | No persistent-chrome quality floor: the Dock/menu bar can degrade to `Minimal` (blur off) under load. | `window/degrade.rs`; audit decision #2 | Add a policy that keeps persistent chrome at `Reduced` or better while transient overlays may reach `Minimal`. |
| G-13 | VRR, night light, color management/HDR, tearing-control, and content-type-specific scanout policy are not implemented. The private output protocol carries `set_vrr`/`set_night_light`, but the compositor accepts and acks them without applying anything; the Displays pane exposes only scale/rotation; `wp-content-type-manager-v1` is advertised but not consumed by policy. | `compositor/src/shell/mod.rs` request handler; `apps/settings/DisplaysPane.qml`; `content_type_state` registration only; no `xx-color-management`/tearing code | Either implement or explicitly mark design-only; do not advertise a Displays capability that is not there. |
| G-14 | Files folder watcher is not wired to the Files facade; thumbnails are local-only (images, PDF/video via optional helpers), not the full freedesktop pipeline for archives/remote files. | `services/files-core/src/watch.rs`, `apps/files/FilesDirectoryModel.cpp`, `FilesThumbnailProvider.cpp` | Wire `apply_watch`/delta reconciliation; document helper-dependent thumbnail coverage. |
| G-15 | Mission Control paging at the readable floor and neighbour-Space reveal are deferred. | `compositor/src/overview/grid.rs` comment; T-05 docs | Add paging and neighbour reveal when the grid is used with many windows. |
| G-16 | `_NET_FRAME_EXTENTS` is not published to X11 clients. | no code in `compositor/src/xwayland.rs`; 05 doc | Publish frame extents so X11 clients can see the SSD titlebar strip. |
| G-17 | Spotlight/search, GOA, printer breadth, NVIDIA-specific validation, HDR, and live PipeWire screencast remain explicit backlog. | T-17 post-gate backlog, `docs/design/07-system-integration.md` | Keep them named; do not let them creep into unrelated units. |
| G-18 | Documentation drift: stale DRM/feather claims, desktop icons "later", Files "cxx-qt"/GVfs/thumbnails/file-opening claims, Control Center five-tile status, Displays VRR/color claims, duplicate ADR 0123, broken links. | this audit | Fixed in the same change as this file; see §7. |
| G-19 | **Shell chrome is not exported to AT-SPI.** The offscreen QPA has no accessibility backend, so the menu bar, Dock, Control Center, OSD, and overlays keep per-component `Accessible.*` semantics but do not appear on the accessibility bus. | ADR 0154, `shell/src/main.cpp` | Accepted boundary for now, but a screen-reader user cannot inspect the chrome. Revisit when the shell transport is reworked (a Wayland/AT-SPI bridge may fall out of that change). |
| G-20 | **No shipped input-method/IME process.** The compositor advertises `text-input`/input-method protocols, but no first-party IME UI or integration ships; CJK/complex-input users depend on a host IME. | `compositor/src/state.rs` protocol registration; no IME service | Decide whether to integrate an existing IME (ibus/fcitx) or ship a minimal candidate window; do not advertise IME support as complete until one is present. |

## 6. Decisions made in code, now recorded

These were implementation decisions that had no design record at the audit
point. They are now either captured in an ADR or reconciled in a topic doc.

| Decision | Where it lives now |
|---|---|
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
- `05-window-decorations.md`: the window menu is still label-less; material and
  `_NET_FRAME_EXTENTS` status are explicit.
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
| Menu model ownership → menu-broker | Mostly clean | The native tier is owned; the DBusMenu projection is owned by app-index and stored by the broker. The shell's fixed-menu fallback duplicates the enable rules by design, but the actual hide verbs are not implemented. |
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
5. **Desktop process lifecycle.** The dev tool provisions the desktop token;
   the systemd session does not. Make the process a first-class session
   service (or explicitly launch it from the shell) and test restart/crash
   isolation on the real session.

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
- **Compatibility closure is incomplete.** DBusMenu clicks, hide verbs, XDnD,
  and `_NET_FRAME_EXTENTS` are exactly the kind of gaps users interpret as
  "Linux desktop jank".
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
12. Spotlight/GOA/printer breadth/live PipeWire screencast, as explicit
    post-gate work.

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
