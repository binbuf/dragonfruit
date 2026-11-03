# Dragonfruit — advisor briefing

**Purpose.** This is the context pack for an advisor who has not seen the
repository. It covers the technology stack, the desktop-environment
architecture, the Dock and compositor decisions, the design-token system, the
frosted-glass/liquid-glass material pipeline, the current native-session gap,
and the decisions that need advice.

**One-line thesis.** Dragonfruit is a Linux desktop environment — its own
Rust/Smithay compositor, Qt Quick shell, first-party apps, and small Rust
services — that reproduces the *interaction model and visual language* of
modern macOS (Tahoe-era references; the user also tracks the newer “Golden
Gate” direction) with original code and assets, while reusing Linux plumbing
(NetworkManager, BlueZ, PipeWire, UPower, UDisks, logind, Mesa, Xwayland,
xdg-desktop-portal).

---

## 1. Technology stack

### 1.1 Platform and runtime

| Layer | Choice |
|---|---|
| Reference OS | Fedora 44; Debian/Ubuntu portability designed in |
| Display protocol | Wayland; Xwayland for X11 clients |
| Graphics | DRM/KMS, GBM, EGL/GLES; Mesa; GLES 3.x / Vulkan-capable GPU is the material baseline |
| Seat/session | systemd user units, logind/libseat, display-manager Wayland session entry |
| Input | libinput, xkbcommon |
| IPC | Private versioned Wayland protocols + D-Bus (`org.dragonfruit.*`) |
| System plumbing | NetworkManager, BlueZ, PipeWire/WirePlumber, UPower, UDisks2/GIO/GVfs, polkit, PAM, xdg-desktop-portal |
| Fonts | Inter (SIL OFL 1.1), bundled and registered by every first-party process |
| Icons | Tabler Icons 3.50.0 (MIT) for line glyphs; Phosphor 2.0.8 (MIT) for filled Settings category tiles; original first-party app icons/wallpapers |
| Cursors | Capitaine XCursor theme, bundled |

### 1.2 Languages and frameworks

| Area | Stack |
|---|---|
| Compositor | Rust 1.98.1 (MSRV 1.80.1), Smithay **=0.7.0** (exact pin), calloop 0.14.4, wayland-server 0.31.10, smithay-drm-extras 0.1.0 |
| Shell | C++20 + Qt 6.11 (Core, Gui, Quick, QuickTest, Test, DBus), QML/Qt Quick, Qt Wayland |
| First-party apps | Qt 6.11 / QML; Settings and Files |
| Services | Rust binaries and libraries over D-Bus |
| Build | Cargo workspace + CMake ≥ 3.24 + Ninja; top-level Makefile orchestrates both |
| Tests | Rust unit/conformance, Qt Quick Test via ctest, QML lint, gallery pixel snapshots, Python capture scripts |
| Capture/diagnostics | Spectacle for nested captures, Pillow, ffmpeg; native session capture service writes unit/journal/render-stat snapshots |

### 1.3 Process model

```text
display manager
  └─ dragonfruit-session.target (systemd --user)
       ├─ dragonfruit-compositor.service   # first; owns seat/DRM/input
       ├─ dragonfruit-shell.service        # Qt Quick chrome; restarts in place
       ├─ dragonfruit-settingsd.service    # desktop settings + persistence
       ├─ dragonfruit-menu-broker.service  # global-menu model routing
       ├─ dragonfruit-app-index.service    # app identity / .desktop / icons
       ├─ dragonfruit-notifications.service
       ├─ dragonfruit-wallpaperd.service
       ├─ dragonfruit-portal.service       # xdg-desktop-portal backend
       └─ system-status / adapters         # NetworkManager, BlueZ, UPower, …
```

- **The compositor is the only process that touches the seat, DRM/KMS, or raw
  input.** Its crash ends the session by design; the display manager returns.
- **Everything else is restartable.** The shell is a Wayland client; services
  are D-Bus clients; apps are ordinary Wayland/Xwayland clients.
- **One owner per state class:** compositor owns windows/focus/stacking/Spaces/
  outputs; `app-index` owns app identity; `settingsd` owns desktop settings;
  `menu-broker` owns which menu model belongs to the focused window; the
  notification service owns the queue/Focus/DND state; host daemons own
  hardware state.
- **Locking is fail-secure** in the compositor (`ext-session-lock-v1`
  semantics); no crash path unlocks a locked session.

### 1.4 Repository map

```text
compositor/       Rust + Smithay compositor
protocols/        private Wayland XML + df-ipc contract crate
shell/            Qt Quick shell: menubar, dock, control-center,
                  notifications, osd, overview, lock, screenshot, switcher
services/         settingsd, menu-broker, app-index, notifications,
                  wallpaperd, session, system adapters, portal host
apps/             Settings, Files
portal/           xdg-desktop-portal-dragonfruit
design-system/    QML module `Dragonfruit` + tokens + gallery
libs/             shared C++/QML support (system font, i18n, …)
tools/            `dragonfruit` dev command, soak/demo/native helpers
scripts/          capture, validation, gates
docs/             design docs, ADRs, runbooks, captures
```

---

## 2. Desktop-environment architecture

### 2.1 Ownership boundary

Dragonfruit owns the things users perceive as “the desktop”:

- compositor/window policy, Spaces, Mission Control, app switcher;
- Dock, menu bar, Control Center, notification center, OSD;
- window-decoration policy (traffic lights where the app permits native SSD);
- Settings UI and desktop configuration model;
- Finder-equivalent Files app;
- global-menu broker (consuming app menu protocols where available);
- session startup, portal integration, private protocols.

It does **not** reimplement networking, Bluetooth, audio, power, storage,
graphics drivers, input, X11 compatibility, or sandboxing. Those stay with the
host Linux stack.

### 2.2 IPC and versioning

Two channels:

1. **Private versioned Wayland protocols** for compositor ↔ shell: workspace
   enumeration, window state, Mission Control, output configuration, layer
   surfaces, panel rects, launch origin, activation results, etc.
2. **D-Bus** for services (`org.dragonfruit.Settings1`, app-index, menu-broker,
   notifications, portal) and system daemons.

Rules: additive-only private protocol changes within a stable release; the
compositor/shell/protocols ship as a lockstep set; cross-version mixing is
unsupported and detected at handshake. D-Bus interfaces use a major-version
suffix.

### 2.3 Session and development workflows

- **Nested mode** (`make dev`, `make dev-full`, `make demo`): the compositor
  runs as a window inside an existing Wayland session, with its own socket and
  services. This is the daily loop and the 80–90% path.
- **Native mode** (`make native`, `make native-debug`, `make native-rebuild`,
  `make native-reboot`, `make native-return`, `make native-recover`): installs
  the session entry, user units, app metadata, portal data, links built
  binaries into `/usr`, preselects the display manager, and logs out/reboots.
- **Profiles:** release (`build-release/`, `target/release`) is now the
  default for every native command because the native session is the
  dogfooding/deployment rail. Debug (`build/`, `target/debug`) is opt-in with
  `make native-debug`, `DF_NATIVE_PROFILE=debug`, or `--debug`. A profile
  switch needs `native-rebuild`/`native-reboot`; `native-restart` refuses a
  mismatch because it cannot replace the running compositor. At 4K, the debug
  material/damage path is CPU-bound enough to miss frame budget.
- **Testing ladder:** nested → dedicated-user real session → VM → hardware
  matrix; native capture service records units, journals, DRM state, and
  render stats.

---

## 3. Compositor

### 3.1 Core choices

- **Rust + Smithay 0.7**, pinned exactly; upgrades are deliberate events.
  Smithay handles Wayland, DRM/KMS/GBM, EGL, libinput, libseat, Xwayland;
  Dragonfruit owns desktop policy.
- **Three backends from day one:** DRM/KMS (native), Wayland/nested (daily
  development), headless (CI).
- **One calloop event loop**, per-output vblank rendering, single-threaded
  scene/GPU work until profiling proves otherwise.
- **One shared animation clock** for lifecycle transitions, overview, and scene
  transforms. A settled clock is not armed: zero damage, zero client wakeups.
- **Damage tracking** keeps the idle path cheap; compositor-drawn effects use
  stable element IDs so unchanged effects do not re-damage every rebuild.
- **Hardware cursor planes** where available; named cursor theme; client cursor
  surfaces still win.
- **Xwayland** via Smithay’s X11Wm + xwayland-shell; X11 windows get SSD and
  participate in the same window model.

### 3.2 Window model and motion

- Windows, focus, stacking, transient dialogs, popups, fullscreen/zoom, and
  per-Space placement are compositor state.
- Lifecycle motions: appear, restore, minimize, close, zoom, fullscreen — one
  `WindowMotion` type and one `MotionFrame` render transform.
- Launch origin: the Dock reports the acted-on entry’s tile rect in output
  coordinates; the compositor plays the window’s appear/minimize/restore from
  the real icon. Missing rect → centered fallback.
- Titlebar double-click and SSD titlebar drag are compositor policy; CSD apps
  keep their own titlebars.

### 3.3 Backends and rendering

| Backend | Renderer | Material path |
|---|---|---|
| Nested (Wayland window) | `GlowRenderer` (GLES via glow) | **Two-pass GPU material implemented** |
| DRM/KMS | `UdevRenderer` = `MultiRenderer<GbmGlesBackend<GlesRenderer>, …>` | **Feather stack only; two-pass is the open follow-up** |
| Headless | software | `MaterialPath::Software` → deterministic `Minimal` |

### 3.4 Material/degrade instrumentation

- `material stats` reports `backdrop_passes`, `backdrop_skipped`, `path=gpu|
  software`, `blur_passes`, `blur_panels`, `blur_downsample_max`.
- `degrade stats` reports the selected tier, budget, over-budget samples,
  downgrades/upgrades, and per-tier frame counts.
- `frame timing` reports average/max frame time and interval.
- `SIGUSR1` dumps stats; clean exit flushes them.
- `path=gpu` means a GPU renderer is available; it does **not** prove the
  sampled-material pass ran. `blur_passes=0` means it did not.

---

## 4. Shell

### 4.1 What the shell is

A Qt Quick process (or set of surfaces) that renders always-visible chrome:
menu bar, Dock, Control Center, notification banners, OSD, overview, lock
screen, app switcher, screenshot UI, Applications drawer. It is a Wayland
client of the compositor.

- Chrome windows render on the **offscreen QPA** and are committed as
  compositor layer surfaces through the private shell protocol.
- The compositor re-injects pointer/keyboard events into the shell as Qt
  events. Every injected event carries a monotonic timestamp (`ChromePointer`)
  so `QQuickDragHandler`/`TapHandler` arbitration works.
- The shell has its own scene-graph commit path and pre-sized popover buffer;
  opening a menu/popover does not resize the window or reallocate the render
  target on the open frame.
- Launch environment: the shell’s forced `QT_QPA_PLATFORM=offscreen` is
  replaced with `wayland` for child apps; the socket is exported as
  `WAYLAND_DISPLAY`.

### 4.2 Chrome surfaces and materials

- Menu bar, Dock, Control Center, notifications, OSD, popovers, and the
  Applications drawer are **compositor-material surfaces**, not flat QML
  rectangles.
- The compositor frosts each panel by `MaterialRole` (`Chrome`, `Dock`,
  `Popup`, `Drawer`) before the QML surface composites over it.
- QML draws a **token-driven scrim/complement** (fill alpha, rim, gloss,
  border, shadow), not an opaque card. The fill’s alpha — never the item’s
  `opacity` — is what lets the material show through.
- The Dock declares its live plate rect via `df_layer_surface.set_panel_rect`;
  the compositor uses it as the backdrop panel. The menu bar/popovers use a
  derived panel unless they opt in.

### 4.3 Menu bar / Control Center / notifications / OSD

- Menu bar: system menu (always present), focused app name/menu via
  menu-broker, status items (Wi-Fi, Bluetooth, volume, battery, clock,
  Focus/DND, accessibility), Control Center entry, Mission Control target.
- Control Center and status menus: popover-family glass; system-service
  adapters degrade cleanly when daemons are absent.
- Notifications: service-owned queue; shell banner is interactive with inline
  actions; one banner at a time; notification center/history.
- OSD: transient popup-family surface for volume/brightness/etc.
- Applications drawer: full-output overlay layer, app-index corpus, search,
  category pills, capped grid, own `Drawer` material role.

---

## 5. Design system and tokens

### 5.1 One source of visual truth

`design-system/tokens/tokens.json` is the single source. It generates:

- `design-system/Theme.qml` for the shell/apps/design system;
- `compositor/src/design_tokens.rs` for the compositor.

Tiers:

```text
primitive tokens    color, radius, spacing, type scale, duration, elevation
semantic tokens     light/dark roles: surface, text, accent, material
component tokens    per-component sizes, states, motion curves
```

This is why compositor-drawn SSD titlebars, QML titlebars, Dock plate geometry,
shadows, and material specs cannot drift across the process boundary.

### 5.2 Components and visual language

- Components: Window, TitleBar, TrafficLights, Sidebar, Toolbar, SplitView,
  SettingsRow/Group, Toggle, SegmentedControl, Slider, Select, Popup,
  ContextMenu, MenuBarMenu, SearchField, SourceList, Icon/TablerIcon/
  PhosphorIcon/SettingsCategoryIcon, Dialog, Sheet, Popover, Tooltip,
  ScrollView, Shadow, FocusRing, Skeleton, WindowResizeGrips.
- Typography: Inter, installed from Qt resources so goldens are
  font-independent on any host.
- Icons: Tabler line glyphs, Phosphor filled tiles; icon registries are build
  gates.
- Motion: named tokens, reduced-motion variants for every animation, progress-
  based/interruptible gestures.
- Quality gates: component gallery + pixel snapshots; QML lint; AT-SPI/
  keyboard audit; dark/light/reduced variants; i18n extraction gate.

### 5.3 Design target and IP boundary

- The reference is the macOS Tahoe-era interaction/visual language (the user
  also tracks the newer “Golden Gate” direction).
- The product rule is: **reproduce interaction quality and mental model, not
  Apple’s bitmap output.** No Apple logo, wallpaper, icons, sounds, branding,
  or pixel-copied artwork ships.
- Apple’s design moves; building against “whatever macOS currently looks like”
  would make Apple an involuntary upstream design dependency. Dragonfruit
  keeps its own token system and adapts the language.

---

## 6. Dock

### 6.1 Division of responsibility

- **Compositor:** windows, focus, stacking, Spaces, activation authority,
  launch-origin animation, layer surfaces, panel rect, input region.
- **Shell/Dock QML:** presentation and interaction — entries, layout,
  magnification, hover/press, tooltips, popovers, drag/drop, keyboard
  navigation.
- **`app-index`:** `app_id`/`WM_CLASS` → `.desktop`, name, icon.
- **`settingsd`:** `dock.pinned`, `dock.size`, `dock.magnification`,
  `dock.position`, `dock.autohide`, `dock.animateOpening`,
  `dock.showIndicators`, `dock.showRecentApps`, `dock.chooserOnHover`,
  `dock.minimizeReaction`, etc.
- **Files/files-core:** Trash state and folder listings.

### 6.2 Model and regions

- `buildDockProjection` groups compositor windows by app, most-recent-first,
  focused window first, with per-window rows and minimized state.
- `buildDockEntries` merges pinned apps, temporary running apps, recents, and
  minimized rows. `applyDockOverflow` shrinks icons to fit and folds excess
  running groups into a terminal overflow cell.
- Regions: pinned prefix, temporary/recent tail, minimized group, fixed tail
  (Downloads stack, user folder pins, Trash). Region boundaries draw hairlines;
  only the app | right-region divider carries the drag resize handle.
- Permanent Applications launcher tile toggles the Applications drawer.

### 6.3 Interaction contract

```text
Pinned app
  ├─ not running → launch
  └─ running
       ├─ one window → activate
       └─ multiple windows → window chooser

Running but not pinned → temporary entry
Minimized window → optional per-window row on the right
```

- Click is a request to the compositor’s activation authority
  (`activate_app`); the compositor replies found/not-found; the shell falls
  back to launching the `.desktop` entry or raising a notice.
- Every launch path goes through one `appLaunchEnvironment` helper.
- The Dock reports the entry tile rect for launch/minimize/restore origin.
- Near-stationary clicks activate; 8 px slop distinguishes tap from drag.
- Window chooser: bounded rows, per-window minimize/restore/close, hover-open
  option, stable geometry-proxy anchor, keyboard navigation.
- Folder stacks: Downloads + user folder pins; popover listing; spring-load
  on dwell; double-click opens folder in Files.
- Trash: Files-backed state; context menu; Empty Trash confirmation and
  busy/success/failure phases.
- Drag/drop: pinned reorder, promote temporary → pinned, remove folder pin,
  external app-alias/file drops, duplicate-pin flash, insertion gap.
- Keyboard: arrow navigation, focus ring, Ctrl+Shift+Arrow reorder, Escape
  releases keyboard focus.
- Hover label: one design-system `Tooltip`, 250 ms dwell, follows the entry,
  suppressed by drag/popover/keyboard; pill with pointer tail.
- Badges: status (failure/missing/Trash unavailable) > app badge > window
  count > none. App badge is the red corner pill; window count is the
  accent/attention pill.

### 6.4 Geometry and magnification

- Floating plate: `edgeMargin` off the anchored screen edge; `padding` is the
  artwork ↔ plate cross-axis inset; `paddingAlong` is the end inset;
  `barThickness = iconSize + 2 * padding`.
- Reserved zone = resting plate + edge margin; magnification reserves nothing
  extra, so windows never re-layout during a sweep.
- Plate wraps the magnified row inside a pre-reserved `magnifyBand`; the
  cross-axis height is fixed; the along-axis has two levels (base and one
  zoomed level) blended by hover engagement.
- Live panel rect: the Dock declares the visible plate; the compositor frosts
  exactly that rect. Input region = plate + currently magnified/bouncing entry
  rects, never the whole surface.
- Positions: bottom / left / right; auto-hide translates the plate off the
  anchored edge and leaves a thin edge trigger; reduced motion collapses
  durations.
- Magnification: distance measured from **stable baseline centres**,
  normalized by resting slot pitch (`iconSize + gap`), monotone cubic through
  `1.00 / 0.90 / 0.45 / 0.00` at `0 / 1 / 2 / 3` pitches. `magnifyFit`
  bisects a lower peak when the zoomed union would exceed the output. The
  row is a continuous warp that keeps the pointer anchored; the plate is
  centered and only changes at engagement. Discrete motion uses a
  non-overshooting ease-out; hover engagement uses `motion.dockHover`.
- Running dots: fixed size, bottom-anchored, full-strength for the app that
  owns the focused window, muted for a merely running app.
- Tokens (current): `iconSize` 48 (32–64), `padding` 15, `paddingAlong` 16,
  `gap` 14, `radius` 27, `indicatorSize` 4, `indicatorGap` 8,
  `magnifyPeak` 1.25 / `magnifyPeakMax` 1.5, `magnifyFalloff` 3.0 pitches,
  `divider` 1 px / 0.78 height / 0.7 opacity / 22 gap,
  `plate.fillOpacity` 0.42, rim 1 px at 0.7, gloss 6 px at 0.22,
  border 1 px at 0.55, shadow blur 20 at 0.3.

### 6.5 Why the Dock is QML and not compositor-drawn

- Qt Quick gives first-class animation, layout, text, accessibility, and a
  rich component library for the most interaction-heavy surface.
- The compositor stays policy/state-focused; it does not become a UI toolkit.
- The shell can be restarted independently of windows/session.
- The cost: cross-process geometry/state handoff, pointer re-injection, and
  the need to keep QML/material tokens in lockstep with the compositor.

---

## 7. Frosted glass / liquid glass

### 7.1 The intended pipeline (nested, implemented)

1. Render the scene (wallpaper, desktop, windows, shadows, menus) into an
   output-sized offscreen `GlesTexture`.
2. For each chrome panel, resolve a `BlurPanel`:
   - `MaterialRole` → `BlurSpec` (radius, Kawase iterations, corner radius);
   - `GlassSpec` (refraction, specular, specular width, adaptive tint).
3. Downsample the panel’s scene region through a 4-tap Kawase chain
   (`BLUR_SHADER`), with a built-in bilinear fallback if the custom shader
   fails.
4. Composite the blurred texture back over the panel clipped to token corner
   spans; optionally run `GLASS_SHADER`:
   - SDF rounded-rect edge lens (refraction);
   - bright specular inner rim;
   - adaptive tint from the blurred backdrop’s mean luminance.
5. Composite chrome Wayland surfaces above the blur; the scene texture sits
   below the blur as one full-output element.
6. One material pass per output per frame (`material_pass` guard); no
   double-blur.

### 7.2 Material roles and tokens

| Role | Used by | Light (opacity/blur/refraction/specular/tint) | Dark |
|---|---|---|---|
| `Chrome` | menu bar | 0.82 / 24 / 6 / 0.45 / 0.16 | 0.72 / 28 / 8 / 0.30 / 0.14 |
| `Dock` | Dock | 0.50 / 30 / 8 / 0.50 / 0.22 | 0.42 / 34 / 10 / 0.34 / 0.20 |
| `Popup` | menus, popovers, OSD, notifications, Control Center | 0.62 / 30 / 8 / 0.50 / 0.20 | 0.70 / 32 / 10 / 0.34 / 0.18 |
| `Drawer` | Applications drawer | popup tokens; corner from `primitive.radius.xl` | same |

- Specular width is 2 px for Chrome, 2.5 px for Popup/Dock.
- The material group is semantic per scheme; the compositor reads generated
  Rust constants, QML reads generated `Theme.material`.
- The QML complement uses `dockFill` + `plate.fillOpacity`, rim/gloss/border/
  shadow; it must not paint an opaque fill over the material.

### 7.3 Degrade policy

- `DegradeTier::Full`: blur + refraction + specular + tint.
- `Reduced`: smaller radius/layers; blur stays on; refraction dropped,
  specular/tint kept.
- `Minimal`: blur off entirely; deterministic feather/translucent fill;
  shadows collapse.
- `MaterialPath::Gpu` selects the budget tier; `MaterialPath::Software`
  (headless) always maps to `Minimal` with no readback.
- Controller: EMA frame duration, 16 ms default budget; downgrade after 12
  over-budget frames; recover after 180 clearly-under-budget frames
  (< 0.75 budget); forceable for tests/captures.
- `DRAGONFRUIT_FRAME_BUDGET_US` can override the budget for acceptance runs.

### 7.4 The current native gap (important)

The native session uses the **DRM backend**, and the DRM rail still renders
the **feather stack**, not the two-pass blur/glass:

- `compositor/src/backend/nested.rs` has the two-pass path;
- `compositor/src/backend/drm.rs` still calls
  `chrome_backdrop_render_elements(...)` and maps it to
  `DrmOutputElements::Decoration`;
- `material stats` in the live native session: `path=gpu`, `blur_passes=0`,
  `blur_panels=0`, `backdrop_passes=11`;
- the currently running session predates the new release default and is still
  the **debug** profile (`target/debug`, `build/`); at 4K the frame time is
  ~494 ms average in the startup sample, so the degrade controller drops to
  `reduced` and can reach `minimal` (the previous session spent ~204k frames
  at `minimal`);
- at `Minimal`, even the feather backdrop is removed; the Dock is only the
  QML translucent plate, which reads flat.
- The QML plate is tuned for a blur-backed material; it has no dedicated
  no-blur milky fallback.

This is the subject of
[`docs/design/proposals/native-drm-dock-glass.md`](proposals/native-drm-dock-glass.md):
P0 release profile, P1 QML milky fallback, P2 DRM two-pass glass, P3 quality
floor, P4 profile guardrails.

### 7.5 Material constraints

- Effects are compositor render passes over live buffers — never client
  re-renders, screenshots, or screencopy. This is a hard rule (FR-1 and the
  no-capture-grab gate).
- GPU material is driver-dependent; captures use tolerance comparison
  (mean ≤ 8/255, worst pixel ≤ 64/255), not byte goldens. Headless stays exact.
- The scene texture is owned per GPU; multi-GPU routing plans local/copy/
  software; the material never samples across GPUs.
- `DrmCompositor` currently has no two-pass branch; adding `Blur`/`Scene`
  element variants and a `BlurRenderer` impl for `UdevRenderer` is the main
  implementation unknown.
- DRM screenshot/capture is not implemented yet; native visual verification is
  limited to diagnostics and nested captures.
- The per-frame blur texture allocation is a known frame-budget follow-up.

---

## 8. Testing and quality gates

- **Rust:** unit tests in the compositor binary, conformance suites
  (`milestone_e2e`, `window_conformance`, `xwayland_conformance`,
  `xdnd_conformance`, `shell_protocol_conformance`, `idle_trace`,
  `latency_trace`, `reduced_motion_sweep`, `session_lock_conformance`,
  `suspend_resume_conformance`, `multi_gpu`, etc.).
- **Qt/QML:** `ctest` runs QML lint and Qt Quick Test suites for the shell,
  design system, Settings, Files, Dock, and every chrome surface.
- **Design system:** gallery app renders every component/state/scheme/motion
  variant; pixel snapshots are a gate; strict mode compares committed goldens.
- **Gates:** `make test`, `make e2e`, `make check` (lint + tests + 100-cycle
  teardown soak), `gen-tokens --check`, `check-design-tokens`, `check-i18n`,
  `check-no-capture-grab`, `check-tabler`, `check-phosphor`,
  `check-desktop-names`.
- **Visual evidence:** nested captures for Dock/materials/windows/overview;
  native capture service for units/journals/DRM/render stats; premium-gate
  report with some hardware rows still OPEN.
- **Known open verification rows:** DRM full loop, baseline Intel/AMD frame
  budget, human unfamiliar-user sign-off, true GPU blur/refraction on DRM.

---

## 9. Current high-level decisions, summarized

1. **Own the compositor and shell; reuse Linux plumbing.** Do not build a
   theme or extension; do not reimplement hardware daemons.
2. **Rust/Smithay for the compositor; Qt Quick for shell/apps.** Pin Smithay
   exactly; upgrade deliberately.
3. **Separate processes with one owner per state class.** Compositor is the
   only non-restartable process.
4. **Private versioned Wayland protocols + D-Bus.** Lockstep releases,
   additive-only changes.
5. **Design tokens are the single visual source.** Generated QML and Rust;
   no literals in components; cross-process geometry/material parity.
6. **Dock is QML presentation over compositor authority.** The compositor owns
   windows/focus/activation; the Dock owns layout/interaction; `app-index` owns
   identity; `settingsd` owns persistence.
7. **Floating Dock plate + reserved zone + live panel rect.** The plate grows
   with magnification inside a pre-reserved band; windows never re-layout
   during a sweep; the compositor frosts exactly the declared plate.
8. **Two-pass GPU material is the intended glass.** Scene texture → per-panel
   Kawase blur → liquid-glass shader (refraction/specular/tint) → chrome
   surfaces. Nested has it; DRM still uses the feather fallback.
9. **Degrade honestly.** Full → Reduced → Minimal, with a software/headless
   `Minimal` path; persistent chrome should not silently lose its material
   under load (current gap).
10. **Original assets and interaction-model emulation.** No Apple bitmaps;
    Tahoe-era language, own tokens, Inter, Tabler/Phosphor, original icons.
11. **Nested-first development; native session for real hardware.** Release is
    the native default; debug is opt-in with `make native-debug`/`--debug`.
12. **Fail-secure lock, restartable services, zero-wakeup idle.** Reliability
    is the real product risk, not drawing controls.

---

## 10. Decisions where advice is useful

1. **Priority:** should the next unit be P1 (QML milky fallback) to make the
   current native Dock look intentional, or P2 (DRM two-pass glass) to fix the
   architecture? P1 is days; P2 is the durable fix and has renderer/compositor
   risk.
2. **Persistent-chrome quality floor:** should the Dock/menu bar be allowed to
   fall to `Minimal` under frame pressure, or should they stay at `Reduced`
   (blur on) while transient overlays degrade further?
3. **Profile policy (decided):** release is the default for every native
   command; debug is explicit (`make native-debug`, `--debug`, or
   `DF_NATIVE_PROFILE=debug`). The current running session still needs one
   `native-rebuild`/`native-reboot` to switch from the old debug install.
4. **Tier signal to QML:** add a compositor → shell material-tier event so the
   QML complement can switch between translucent and milky, or keep the QML
   style always-on and conservative?
5. **DRM two-pass scope:** is it worth porting the full liquid-glass shader to
   `UdevRenderer` now, or ship a cheaper DRM-specific blur first and defer
   refraction?
6. **Multi-GPU:** how much of the scene-texture copy/routing path should be in
   the first DRM implementation versus a local-only fallback?
7. **Verification:** with DRM capture unimplemented, what is the minimum
   trustworthy visual evidence for the native rail — diagnostics + nested
   captures, a KMS readback tool, or a temporary screenshot path?
8. **Material fallback design:** solid milky gray, wallpaper-tinted sample, or
   a cheap static blur approximation for the no-blur path? The reference
   `#E5E5E5` is a composited output, not a direct opacity value.

---

## 11. Key files for a deeper review

```text
docs/design/00-overview.md
docs/design/01-architecture.md
docs/design/02-compositor.md
docs/design/04-shell.md
docs/design/10-design-system.md
docs/design/11-session-and-dev-workflow.md
docs/design/14-risks.md
docs/design/adr/0089-dock-plate-geometry-and-live-panel-rect.md
docs/design/adr/0091-dock-tahoe-floating-glass-language.md
docs/design/adr/0102-dock-material-role-and-qml-glass-layers.md
docs/design/adr/0162-dock-hover-zoom-profile-and-label-tail.md
docs/design/adr/0182-tahoe-liquid-glass-material-pass.md
docs/design/adr/0186-dock-reference-retune-profile-fit-and-states.md
docs/design/proposals/native-drm-dock-glass.md

compositor/src/backend/nested.rs      # two-pass blur/glass reference
compositor/src/backend/drm.rs         # feather fallback; missing two-pass
compositor/src/render.rs              # chrome_blur_panels / backdrop elements
compositor/src/window/blur.rs         # BLUR_SHADER / GLASS_SHADER / elements
compositor/src/window/degrade.rs      # Full/Reduced/Minimal + budget controller
compositor/src/window/backdrop.rs     # MaterialRole / BackdropSpec / feather
compositor/src/multi_gpu.rs           # scene-texture routing plan
shell/dock/Dock.qml                   # plate, layout, magnification, popovers
shell/dock/DockEntry.qml              # entry states, badges, indicator
shell/src/dockprojection.cpp          # app grouping / active flag
shell/src/dockmodel.cpp               # pinned merge / overflow
design-system/tokens/tokens.json      # single visual source
design-system/Theme.qml               # generated QML tokens
compositor/src/design_tokens.rs       # generated Rust tokens
scripts/native-session.sh             # native profile/install/reboot
docs/captures/t17-premium-gate.md     # quality gate / open rows
```

---

## 12. Short version for the advisor

Dragonfruit is a pre-alpha Linux DE with a Rust/Smithay compositor, a Qt Quick
shell, and small Rust services. It owns the compositor, Dock, menu bar,
Settings, Files, and design system, and reuses Linux plumbing. The Dock is QML
presentation over compositor state, with a floating plate, live panel rect,
magnification, window chooser, folder stacks, Trash, drag/drop, keyboard, and
tooltips. The design system is token-generated into both QML and Rust, and the
intended glass is a two-pass GPU pipeline: scene texture → Kawase blur →
liquid-glass shader (refraction, specular, adaptive tint), with Full/Reduced/
Minimal degrade tiers. The nested backend implements it; the native DRM backend
still uses the old feather fallback. The currently running native session
predates the new release default and is still debug at 4K, so it degrades to
Reduced/Minimal. The Dock QML is current; the premium look is missing because
the native material path is not implemented and the fallback QML plate is not
tuned for no-blur. The main decision is whether to ship a QML milky fallback
now, port the DRM two-pass glass next, or do both in sequence.
