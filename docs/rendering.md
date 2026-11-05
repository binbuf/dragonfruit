# Rendering — remaining work

| | |
|---|---|
| **Status** | active follow-up plan |
| **Date** | 2026-11-03 |
| **Scope** | native DRM material, shell transport, frame scheduling, multi-GPU, native verification |
| **Related** | [`design/architecture-audit-native-rendering.md`](design/architecture-audit-native-rendering.md), [`design/proposals/native-drm-dock-glass.md`](design/proposals/native-drm-dock-glass.md), [`design/02-compositor.md`](design/02-compositor.md), [`runbook-drm-session.md`](runbook-drm-session.md) |

This file is the work queue after the audit. The audit explains **why**; this
file records **what remains**, with acceptance criteria and rollback points.

The native two-pass material is now **default-on**:

- `DRAGONFRUIT_DRM_MATERIAL=0` disables the scene texture + Kawase blur and
  returns to the feather stack + milky shell plate.
- `DRAGONFRUIT_DRM_GLASS=0` keeps the Kawase blur but disables the
  liquid-glass compose (refraction, specular, adaptive tint).

Only explicit values are persisted by `scripts/native-session.sh`; an unset
variable keeps the default.

## v0.1.1 task mapping

The executable v0.1.1 task set is in
[`docs/tasks/0.1.1/`](tasks/0.1.1/README.md) under
[Track 21](design/tracks/21-presentation-foundation.md). It maps to this queue:

| Queue item | Task |
|---|---|
| R2.1/R2.2/R2.3 shell backend, HiDPI, copies | T183, T184, T185, T186 |
| R3.1/R3.2/R3.3 refresh deadline, presentation pacing, mixed refresh | T187, T188 |
| R4.1–R4.4 persistent-chrome floor, hysteresis, fallback coverage | T189 |
| R1.1–R1.4 panel region, stable IDs, allocations, direct scanout | T190 |
| R0/R6 native material acceptance and evidence | T191 |
| R7/R8 benchmark, soak, observability, docs | T192, T193 |

---

## Current state

| Area | State | Notes |
|---|---|---|
| P0.1 windowed frame stats | **done** | `frame timing` now reports windowed `avg_us`, `p50_us`, `p95_us`, `p99_us`, `window_max_us`, `samples_total`. |
| P0.2 presentation stats | **done (instrumentation)** | DRM `frame_finish` records actual vblank intervals per output; `presentation stats` reports `missed` and `budget_us`. Native baseline not yet captured. |
| P1.1 effective material state | **done** | `df_output.material_state` (v3) reports tier, `blur_active`, and fallback reason. |
| P1.2 milky fallback | **done** | Dock/menu bar use token-driven light/dark fallback when `blur_active=0`. |
| P2A DRM offscreen scene | **done, default-on** | Per-output scene texture; custom `SceneTextureElement`; feather fallback on failure. |
| P2B DRM Kawase blur | **done, default-on** | Per-panel chain reuses `BLUR_SHADER`; `blur_draws` counts built elements. |
| P2C glass compose | **done, default-on** | Reuses `GLASS_SHADER` and `GlassSpec`; `DRAGONFRUIT_DRM_GLASS=0` disables. |
| P3 shell transport | **baseline + HiDPI done (T-21.1a/b)** | Shell is software-rendered offscreen QPA; `grabWindow` + `QImage` + fresh `wl_shm` per commit. HiDPI chrome now commits physical buffers with `buffer_scale`/`wp_viewport` ([ADR 0192](design/adr/0192-chrome-scale-physical-buffer-and-viewport.md)); pooling, frame callbacks, and GPU transport remain T185/T186. Measured in [`captures/t21-chrome-baseline.txt`](captures/t21-chrome-baseline.txt); transport decision [ADR 0191](design/adr/0191-chrome-transport-render-control-and-shm-fallback.md). |
| P4 refresh-aware scheduling | **wired (T-21.2b)** | The animation clock and degrade budget use the fastest active output's period, with `DRAGONFRUIT_REFRESH_HZ` test overrides and `refresh_hz`/`source` stats ([ADR 0199](design/adr/0199-refresh-clock-wiring.md)). Per-output urgent/animation requests and earliest-deadline arming are wired ([ADR 0201](design/adr/0201-per-output-frame-requests-and-earliest-deadline.md)); VRR adaptation, chrome callbacks, and synthetic 60+144 evidence are wired ([ADR 0202](design/adr/0202-vrr-adaptation-and-mixed-refresh-evidence.md)). Native mixed-refresh remains an explicit `OPEN` artifact until a free two-output DRM seat runs it. |
| P5 multi-GPU | **single-GPU only** | Material path disables itself on a multi-GPU seat; shell falls back. |
| P6 native visual verification | **not implemented** | No trusted native capture path. |

**Important:** the default-on native path has not yet been accepted on hardware.
The first task below is the gate.

---

## R0 — Accept the default-on native material on hardware (blocker)

**Why.** The code is default-on, but only nested rendering has been exercised
end-to-end. A native DRM run must prove the scene texture, plane composition,
blur sampling, and fallback behavior on the real seat.

**Steps**

1. Rebuild and log in without any material env flags:
   ```bash
   make native-rebuild
   ```
2. Confirm the binaries are release and the installed profile is release.
3. Capture diagnostics:
   ```bash
   make native-capture
   ```
4. Read `material stats` from the latest snapshot:
   ```text
   path=gpu blur_passes>0 blur_panels>=1 blur_draws>0
   ```
5. Confirm `degrade stats` is `full` or `reduced`, not `minimal`.
6. Visually inspect the Dock and menu bar over a light and a dark wallpaper.
7. Exercise a pointer sweep, app launch, popover, Applications drawer,
   Mission Control with 10+ windows, fullscreen video, and lock/unlock.

**Acceptance**

- Native `blur_draws > 0` and `blur_panels >= 1`.
- The blur samples the live wallpaper/window content behind the Dock, not a
  static or stale image.
- No black frame, no compositor crash, no cursor loss, no input lag spike.
- `make native-recover` remains the rollback if the session cannot start.

**Rollback**

```bash
DRAGONFRUIT_DRM_MATERIAL=0 make native-rebuild
```

If the compositor is already down, use `make native-recover` from a TTY first.

---

## R1 — Native material performance and resource bounds

The current implementation is intentionally the simplest correct shape:

- a full-output scene texture per output;
- a full-output Kawase chain per panel;
- one scene render + one output composite per frame;
- no panel-region scissoring.

At 4K this is the main performance risk.

### R1.1 Panel-region scene rendering

**Why.** A 3840x2160 scene texture is 33 MB per output; rendering and blurring
the full output for a 2560x125 Dock panel wastes bandwidth and GPU time.

**Work**

- Render only the union of the material panel rects plus blur-radius padding
  into a smaller scene texture, or update a cached full scene only in damaged
  regions.
- Keep the scene texture in output coordinates so panel `src` regions still
  map correctly.
- Reallocate on panel-union size changes, not every frame.

**Acceptance**

- Steady-state Dock magnification does not allocate a new texture per frame.
- GPU work for a stationary Dock is materially below the full-output path.
- Visual output is pixel-equivalent to the full-output path at the panel.

### R1.2 Stable element IDs and cache reuse

**Why.** `SceneTextureElement` and `BackdropBlurElement` currently create a new
`Id` each frame. The scene changes every frame, but the panel elements and
texture handles should not force full-output damage when only a small region
changed.

**Work**

- Give the scene and each panel a stable `Id` per output/panel slot.
- Keep the element instances or reuse IDs across frames.
- Measure `DrmCompositor` damage and `direct_scanouts` before/after.

**Acceptance**

- A stationary Dock with an unchanged wallpaper does not damage the full
  output every frame.
- No stale pixels at panel edges after a damage-only update.

### R1.3 Allocation and synchronization audit

**Work**

- Confirm no `create_buffer` calls after warm-up at a fixed mode/scale.
- Check `MultiFrame` synchronization and `GlesTexture` lifetimes under repeated
  panel add/remove.
- Add a debug assertion or counter for texture allocations per frame.

**Acceptance**

- Zero steady-state allocations in a 5-minute Dock animation run.
- No GPU memory growth across mode changes, output hotplug, or 100 cycles.

### R1.4 Direct-scanout policy

**Why.** A full-output scene texture forces composition and loses direct
scanout for fullscreen clients.

**Work**

- Keep the feather/no-material path as the direct-scanout path.
- Detect a fullscreen client with no overlapping chrome material and skip the
  material pass for that output-frame.
- Report direct scanouts separately from material frames.

**Acceptance**

- A fullscreen video/game direct-scans out when no material overlaps it.
- The Dock/menu bar still frost when they are visible.
- No tearing or frame-callback stalls at the transition.

---

## R2 — Shell transport and HiDPI (P3)

The shell is a Qt Quick process on the offscreen QPA. The live process maps no
GLES/GL/DRI driver and runs the software scene graph. Each Dock frame currently:

1. renders the QML scene;
2. `grabWindow()` renders it a second time;
3. copies the bar/popover out of the image;
4. `memcpy`s into a fresh `wl_shm` buffer;
5. commits it; the compositor imports/uploads it.

The shell also commits a logical-resolution buffer without
`wl_surface.set_buffer_scale`, so a 1.5x output upscales the chrome.

### R2.1 Confirm the scene-graph backend

**Status: done (T-21.1a).** The committed baseline
[`captures/t21-chrome-baseline.txt`](captures/t21-chrome-baseline.txt) records
`QSG_INFO=1` → `Loading backend software`; the live shell maps the EGL/GLX
loader libraries but no DRI driver. The measured path and the transport
decision are in [ADR 0191](design/adr/0191-chrome-transport-render-control-and-shm-fallback.md).

**Work**

- Run a controlled shell restart with `QSG_INFO=1` and capture the backend,
  graphics API, and swapchain details.
- Record whether the offscreen QPA is software, GL, or RHI-backed.

**Acceptance**

- A committed diagnostic log states the active backend and API.
- No transport rewrite starts before this evidence exists.

### R2.2 HiDPI chrome

**Status (T-21.1b): done.** The shell binds `wp_fractional_scale_manager_v1`
and `wp_viewporter`, keeps the QML scene in logical coordinates, and commits
physical buffers: exact integers use `wl_surface.set_buffer_scale`; fractional
scales commit `ceil(logical * scale)` and map it with one `wp_viewport` per
surface. The pure `ChromeScale` helper owns the rounding and pointer mapping.
The menu bar, Dock, Dock/menu popovers, Control Center, notifications, OSD,
overview/switcher, Applications drawer, and lock/portal overlays all use the
same scene setup. `chrome stats` reports the resulting `logical_size`,
`buffer_size`, `buffer_scale`, `scale`, and `viewport` per surface.

**Work**

- Render the shell at physical resolution with the correct buffer scale, or
  use a Qt-supported fractional-scale path.
- Apply the same treatment to the menu bar, Dock, popovers, and overlays.
- Recompute on `preferred_scale`, output-scale changes, configure, and hotplug;
  keep reserved zones, panel rects, and input regions logical.

**Acceptance**

- Dock/menu-bar text, icons, and rim are native-resolution at 1.5x and 2x.
- No layout or hit-testing regression at 1.0x, 1.5x, or 2x.
- Nested stills: `docs/captures/t21-hidpi-{1x,1_5x,2x}.png`, plus the 1.5x Dock
  popover and Control Center stills. Design contract:
  [ADR 0192](design/adr/0192-chrome-scale-physical-buffer-and-viewport.md).

### R2.3 Remove full-surface copies

**Landed (T-21.1c).** Every shell surface now has one `ChromeFrameProducer`:
a `QQuickRenderControl` scene rendered once into a ping-pong `QImage` pair,
never `QQuickWindow::grabWindow()`. Each `wl_surface` owns a three-buffer
`wl_shm` pool; `wl_buffer.release` returns a buffer, a resize retires the old
generation until the compositor releases it, and steady-state frames allocate
nothing. `ShellProtocol::commitTo()` copies the source rectangle exactly once,
compares it with the previous frame to damage only the changed pixel bounding
box, and skips the commit entirely when nothing changed. `wl_surface.frame`
callbacks answered by the compositor in `post_repaint()` and the no-damage path
pace the next QML render; a 100 ms safety tick is counted as
`fallback_renders` if the compositor never answers. The private layer protocol
and surface roles are unchanged, and the shell remains independently
restartable.

**Acceptance**

- One animated Dock frame performs at most one full-surface CPU copy.
- Copy count and bytes per frame are logged and measurably lower.
- The private layer protocol and shell restartability are unchanged.

Before/after evidence:
[`captures/t21-chrome-baseline.txt`](captures/t21-chrome-baseline.txt) and
[`captures/t21-chrome-transport-after.txt`](captures/t21-chrome-transport-after.txt).
The GPU/dmabuf transport remains T186; the pooled shm path is its fallback.

**Spike (T186a).** The GPU path now has a capability probe and a
`QQuickRenderControl` render-control skeleton. `GpuChromeProbe` binds
`zwp_linux_dmabuf_v1` read-only on the shell's existing Wayland connection,
logs its feedback formats/modifiers and the EGL export/import extensions, and
records a fallback reason when any prerequisite is missing. The pure
`GpuChromeCapabilities` helper selects `ABGR8888`/`ARGB8888` with a
deterministic modifier preference and is covered by `tst_gpucapabilities`.
`GpuChromeFrameProducer` subclasses the T185 producer with a `QOpenGLContext`,
`QOffscreenSurface`, hidden `QQuickWindow`, reusable FBO and
`QQuickRenderTarget` DPR handling; one QML scene renders once at 320x180 from
a 160x90 logical scene at DPR 2. The flag stays off, no `wl_buffer` is
committed, and the offscreen shell rail records `no-gl-context` instead of a
black frame. Evidence:
[`captures/t21-gpu-probe.txt`](captures/t21-gpu-probe.txt) and
[`captures/t21-gpu-smoke.png`](captures/t21-gpu-smoke.png); contract
[ADR 0194](design/adr/0194-gpu-chrome-probe-and-render-control-spike.md).

**Dock dmabuf transport (T186b).** With `DRAGONFRUIT_CHROME_GPU=1` and a host
`WAYLAND_DISPLAY`, the shell uses the Wayland QPA for the Qt GL context only
and renders every chrome scene through `GpuChromeFrameProducer`. The Dock
exports its FBO texture as an EGLImage, queries/exports it with the Mesa
dmabuf entry points, validates the exact FourCC/modifier against the
compositor feedback, and creates one reusable `zwp_linux_buffer_params_v1`
buffer (with `y_invert` for the GL texture origin). `ShellProtocol::
commitDmabufTo()` attaches that buffer through the Dock's viewporter source
rect; `glFinish()` completes the GL work before each commit. The other chrome
surfaces still read back and commit through the T185 shm pool. Any export,
format/modifier, viewporter, or import failure falls back to the same shm path
and is reported as `gpu=shm gpu_reason=<token>`; `chrome stats` also carries
`dmabuf_commits/bytes/deferred/fallbacks`. Evidence:
[`captures/t21-dmabuf-dock.txt`](captures/t21-dmabuf-dock.txt) (GPU run
`path=render-control+dmabuf`, `dmabuf_commits=13`, `grab_renders=0`, Dock
`copy_bytes=0`, compositor `chrome_dmabuf_imports=13`; fallback run
`gpu=off`, zero dmabuf imports) and the two host stills
[`t21-dmabuf-dock.png`](captures/t21-dmabuf-dock.png) /
[`t21-dmabuf-dock-fallback.png`](captures/t21-dmabuf-dock-fallback.png).
Contract: [ADR 0195](design/adr/0195-dmabuf-export-and-single-buffer-dock-commit.md).

**Dock dmabuf ring (T186c).** The Dock now uses a bounded three-slot
per-surface `ChromeDmabufRing`. Each slot owns an FBO/export and one reusable
`wl_buffer`; `wl_buffer.release` returns the slot to the ring, and a stale
serial never frees a newer commit. A size or export-format change retires the
old generation: free slots are destroyed immediately, busy ones stay alive
until release. The producer waits on a GL fence (`glFenceSync` /
`glClientWaitSync`, 100 ms bound) before handing the buffer over; if fences are
unavailable it falls back to `glFinish`. The compositor exposes only implicit
dma-buf sync (no `linux-drm-syncobj-v1`), so the exported buffer carries the
driver's implicit dma-fence and the GL fence closes the producer-side race; that
implicit-only limitation is recorded in `04-shell.md`. Ring exhaustion commits
the last complete frame through the T185 shm pool for that frame and retries on
release; export/import/format failures abandon the rail for the session. Both
are counted in `dmabuf_fallbacks` with a `gpu_reason`, and no path goes black.
`chrome stats` now carries `dmabuf_buffers`, `dmabuf_allocs`, `dmabuf_reuses`,
`dmabuf_sync`, `dmabuf_sync_waits`, and `dmabuf_sync_timeouts`. Evidence:
[`captures/t21-dmabuf-ring.txt`](captures/t21-dmabuf-ring.txt) (warm/steady
`dmabuf_buffers=3`, flat `dmabuf_allocs`, advancing commits/reuses; fallback
run `gpu=off`, zero dmabuf imports) and the two host stills
[`t21-dmabuf-ring.png`](captures/t21-dmabuf-ring.png) /
[`t21-dmabuf-ring-fallback.png`](captures/t21-dmabuf-ring-fallback.png).
Contract: [ADR 0196](design/adr/0196-dmabuf-ring-release-reuse-and-sync.md).

**Chrome rollout (T186d).** Every chrome scene now commits through the same
dmabuf path: the menu bar and its dropdown, the Dock and its popover, Mission
Control, the app switcher, notification banner, Control Center, OSD, chooser,
screenshot selection, ScreenCast picker, polkit dialog, Applications drawer,
and the session-lock surfaces. `ShellProtocol::commitDmabufFor()` addresses a
surface by its stable `chrome stats` label; `commitLockDmabuf()` addresses a
lock surface by its configure id. A shared scene (bar + popup, Dock + popover)
renders once and commits the same exported frame to both surfaces; the producer
ring slot is held until the last `wl_buffer.release`, and if any surface cannot
express the commit the whole scene takes the T185 shm path. No successful
dmabuf frame performs a CPU readback. Each `chrome surface` stats line now
reports `path=dmabuf|shm`, `gpu_reason`, the live `dmabuf_buffers` count,
`dmabuf_allocs`/`dmabuf_reuses`, and `copy_bytes`; a GPU frame has
`copy_bytes=0`, while a fallback frame is counted with its reason. Evidence:
[`captures/t21-gpu-rollout.txt`](captures/t21-gpu-rollout.txt) (Dock, menu bar,
both popovers, and lock on dmabuf; `DRAGONFRUIT_CHROME_GPU=0` on shm with zero
dmabuf imports) and the five stills
[`t21-gpu-rollout-dock.png`](captures/t21-gpu-rollout-dock.png),
[`-menubar.png`](captures/t21-gpu-rollout-menubar.png),
[`-popover.png`](captures/t21-gpu-rollout-popover.png),
[`-lock.png`](captures/t21-gpu-rollout-lock.png), and
[`-fallback.png`](captures/t21-gpu-rollout-fallback.png). Contract:
[ADR 0197](design/adr/0197-gpu-chrome-rollout-and-per-surface-stats.md). The
default remains off until T192/T193 accept the rail; the implicit-sync
limitation still applies to a shared scene under load.

**Close-out (T185a).** The final-tree rerun was host-throttled: the nested
window was not presented at 60 Hz, so the shell's 100 ms safety tick is visible
as `fallback_renders` and the Dock sweep commits fewer frames than the accepted
run. The transport invariants still held on that tree: `grab_renders=0`, a
bounded pool, and exactly one 960,000 B Dock copy per committed frame. A second
final-tree run on a KWin virtual host (1920x1200 nested output) reproduced the
accepted shape: Dock sweep `commits=136`, `grab_renders=0`, `shm_allocs=3`,
`copy_bytes=123,177,360`, `frame_callbacks=78`, `fallback_renders=0`. The
18:57 accepted run's numbers remain the comparison in the committed report,
with the host-throttled sample and its caveat beside it. Live close-out stills
are `captures/t21-chrome-motion.png` (a frame during Dock magnification),
`-menubar-open.png`, `-dock-popover.png`, `-dock-rest.png`, and `-lock.png`
(the lock surface with typed password dots). The lock smoke test used the
system menu because the synthetic harness on the close-out host did not set the
Meta modifier for the `Cmd+Ctrl+Q` chord. `FrameCommitGate` is retired: no
runtime path used it after T-21.1c, so its header and unit tests were removed.

**Do not** build a custom QPA plugin or a second UI renderer without the R2.1
evidence.

---

## R3 — Refresh-aware animation and scheduling (P4)

The animation clock and degrade budget now derive from the fastest active
output's refresh period (T187b); per-output presentation deadlines, mixed
refresh, and VRR are still T188. The live output is 4K @ 143.999 Hz, where the
deadline is 6.94 ms.

### R3.1 Per-output deadline

**Work**

- Replace the global `FRAME_INTERVAL` budget with a per-output refresh period.
- Keep animation progress on monotonic elapsed time; use the deadline only for
  scheduling and degrade decisions.
- Do not replace 16 ms with a global 8 ms.

**Acceptance**

| Refresh | Deadline | Expected |
|---|---|---|
| 60 Hz | 16.67 ms | baseline |
| 120 Hz | 8.33 ms | no global-16 ms downgrade |
| 144 Hz | 6.94 ms | animation pacing measured |
| 240 Hz | 4.17 ms | no unnecessary damage when settled |

### R3.2 Presentation feedback and missed deadlines

**Work**

- Use the `presentation stats` ring per output.
- Count missed deadlines at 1.5x the refresh period.
- Distinguish CPU render duration, GPU execution, and presentation interval.
- Add p50/p95/p99 and missed counts to the native acceptance report.
- The pure per-output presentation clock (`last_presented`, `next_deadline`,
  `is_due`, `time_until_due`) is in place (T-21.2b, ADR 0200); DRM records the
  vblank timestamp and nested records the successful `submit()` completion.
  Earliest-deadline timer arming, urgent-vs-animation requests, and the DRM
  in-flight marker are wired in T188b (ADR 0201).
- A settled output has no animation request, so it produces no frame and no
  presentation sample even while another output animates.

**Acceptance**

- A 60 Hz animation on a 144 Hz output is not reported as healthy when it
  misses 144 Hz deadlines.
- A settled desktop produces zero presentation samples and zero damage.

### R3.3 Mixed-refresh and VRR

**Work**

- Schedule per output, not from a single global clock. Done in T188b: the
  scheduler arms one timer from the earliest due request and presents each
  output only at its own deadline (ADR 0201).
- Account for VRR state where the backend exposes it. Done in T188c (ADR 0202):
  a `VrrState::On` output with a backend range adopts an in-range presentation
  interval; every other output keeps the fixed period and reports
  `vrr=unsupported`/`off`.
- Test a 60 Hz + 144 Hz mixed setup. The pure scheduler test and the headless
  synthetic 60/144 trace are committed; the native two-output run is an explicit
  `OPEN` artifact (`docs/captures/t21-vrr-mixed-refresh-native.txt`) until a
  free DRM seat with both outputs is available.

**Acceptance**

- A slow output does not throttle a fast output's animation cadence.
- No double-damage or clock drift when one output is idle.
- Chrome frame callbacks follow every presented output frame and the no-damage
  callback path; the shell is not left waiting for a presentation.

---

## R4 — Material quality floor and degrade policy

T-21.2c classifies `MaterialRole` as persistent (`Chrome`, `Dock`) or
transient (`Popup`, `Drawer`) and computes an effective material per role. The
compositor render path and the v4 protocol carry that state; the shell applies
it through one policy, and the degrade hysteresis is refresh-aware with a flap
latch (T189d).

### R4.1 Persistent-chrome floor

**Work**

- Keep persistent chrome at `Reduced` (blur on) at worst on the GPU path.
- Allow transient overlays to reach `Minimal`.
- If the GPU cannot meet the deadline even at `Reduced`, use the P1 milky
  fallback rather than a flat translucent plate.

**Status (T-21.2c)**

- `MaterialRole::is_persistent()` is the classification. The render path
  resolves each chrome panel through `effective_material_for_role`, so a
  global `Minimal` selection no longer drops the Dock/menu bar to the feather
  stack.
- A persistent role at a selected `Minimal` reports `tier=reduced`,
  `blur_active=0`, `fallback=4` (`floor-missed`): the explicit milky fallback,
  not a flat translucent `Minimal` slab.
- Transient roles follow the ladder to `Minimal` unchanged; software/headless
  stays `Minimal` with the `software` reason for every role.

**Acceptance**

- The Dock/menu bar never silently lose their material under load.
- A forced `Minimal` still reads as a deliberate milky plate.

### R4.2 Hysteresis and oscillation

**Work**

- Validate the 12-frame downgrade / 180-frame recovery policy at 60, 120, and
  144 Hz.
- Prevent Full↔Reduced oscillation during a steady pointer sweep.
- Ensure expensive glass is disabled or reduced quickly enough under pressure.

**Status (T-21.2c)**

- The 12/180 frame counts are now wall-clock windows accumulated from rendered
  frame durations: **200 ms** down and **3 s** up. At 60 Hz that is the same
  12/180 reference; at 120/144 Hz the frame count scales with the refresh
  period instead of shrinking the window.
- A downgrade within 10 s of an upgrade across the same ladder boundary
  latches that boundary. Recovery across a latched boundary needs the EMA below
  **0.4 × budget** for **10 s** (instead of 0.75 × budget for 3 s), so a steady
  sweep that keeps returning to the expensive tier stays at `Reduced`. A
  successful upgrade clears the latch; a forced tier resets the selection
  memory.
- `degrade stats` / `query degrade` add `down_window_us=200000`,
  `up_window_us=3000000`, and `latched=0|1`. The synthetic
  `set degrade-frames <count> <microseconds>` hook feeds the real
  `observe_rendered_frame` path for the headless load test.
- The committed load trace is `docs/captures/t21-degrade-load.txt`
  (`make t21-degrade-load`): a 60 s simulated sweep at 60/120/144 Hz plus the
  headless compositor's per-role load sequence. The pure tests assert the
  ~200 ms/~3 s windows at all three rates; the 60 s sweep is bounded to
  `downgrades=2 upgrades=1 latched=1` at every rate.

**Acceptance**

- No repeated tier flapping in a 60-second Dock animation.
- Tier transitions are visible in `degrade stats` and bounded.
- The same wall-clock windows hold at 60, 120, and 144 Hz.

### R4.3 Per-role/per-output material state

**Work**

- Extend `material_state` beyond the current global snapshot if persistent
  chrome and transient overlays need different effective tiers.
- Keep the event change-only and additive.

**Status (T-21.2c)**

- `df_output` is v4. `material_role_state` is the additive event:
  `role` (0=chrome, 1=dock, 2=popup, 3=drawer), `tier`, `blur_active`, and
  `fallback` (0=none, 1=no-sampled-blur, 2=software, 3=multi-gpu,
  4=floor-missed). The legacy v3 `material_state` keeps its global meaning.
- The compositor emits the role event per output only for roles whose
  effective state changed, not per frame. The shell parses and stores the
  role table; `ShellController` applies it through one `MaterialPolicy`
  object. The menu bar consumes `Chrome`, the Dock consumes `Dock`, and the
  legacy v3 snapshot is only the compatibility source for a role that has not
  received a v4 state. Any explicit fallback reason selects the milky
  complement; a persistent `Minimal` with no reason is guarded to the explicit
  `floor-missed` state (ADR [0204](adr/0204-shell-role-material-application-policy.md)).
  The popup/overlay family uses the same policy object (T189c); see R4.4.

**Acceptance**

- The shell can choose the fallback per role without duplicating the
  compositor's degrade policy.

### R4.4 Fallback coverage

**Work**

- Apply the milky fallback to popovers, Control Center, notifications, OSD,
  and the Applications drawer where they currently assume blur.
- Keep the Dock/menu-bar fallback tokens as the source of truth.

**Status (T-21.2c)**

- The Dock and menu bar are covered (T189b): `MaterialPolicy` turns every
  explicit fallback reason, including `floor-missed`, into the existing
  token-driven milky complement.
- The popup family is covered (T189c): menus and Dock popovers, Control
  Center, notification banners, and the OSD read the `Popup` role; the
  Applications drawer reads `Drawer`. `design-system/Material.qml` is the one
  fill/fallback complement, and every surface records its
  `materialFallback` reason. `material.popupFallbackOpacity` is the new
  fallback alpha (0.92 light / 0.90 dark); the fill reuses
  `chromeFallbackFill`, so the Dock/menu-bar fallback tokens remain the single
  neutral source. Forced Full/Reduced/Minimal captures cover each surface in
  light and dark (ADR [0205](design/adr/0205-popup-family-material-fallback.md)).

**Acceptance**

- No chrome surface becomes a flat translucent slab when the sampled pass is
  absent.

---

## R5 — Multi-GPU and compatibility (P5)

The first native slice disables the material path on a multi-GPU seat. That is
the intended safe fallback, not the final behavior.

### R5.1 Same-GPU, multiple outputs

**Work**

- Verify one render GPU driving two outputs.
- Keep one scene texture and panel cache per output.
- Confirm per-output scale/transform handling.

**Acceptance**

- Both outputs blur independently.
- No cross-output texture sharing or stale panel rects.

### R5.2 Hotplug and mode changes

**Work**

- Reallocate scene/blur textures on mode, scale, and transform changes.
- Handle output removal while a material frame is in flight.
- Verify the shell fallback event on a failed output.

**Acceptance**

- No leaked textures or stale `blur_panels` after 10 hotplug cycles.
- A mode change does not leave a black or stretched panel.

### R5.3 Cross-GPU copy or explicit fallback

**Work**

- Do not sample a texture created on one GPU from another.
- If a compatible import/copy exists, implement it with explicit sync.
- Otherwise report `fallback=multi-gpu` and use the P1 milky plate.

**Acceptance**

- Hybrid laptops either show correct local glass or a documented fallback.
- No CPU readback in the production cross-GPU path.

---

## R6 — Native visual verification (P6)

Nested screenshots are not native proof. The project prohibits unrestricted
client screenshot access, so the capture path must stay trusted.

### R6.1 Trusted native capture

**Work**

- Investigate a trusted, explicitly enabled capture path through the existing
  portal architecture, or an isolated development-only KMS readback tool.
- Keep the no-screencopy rule intact.

**Acceptance**

- A native capture exists for light/dark, Full/Reduced/Minimal, and 1080p/4K.
- The capture path is opt-in, trusted, and does not expose arbitrary clients.

### R6.2 External evidence

If native capture is not practical yet:

- Use external photographs/video of the actual panel.
- Use native material/frame diagnostics.
- Use nested captures only as reference evidence.
- Never label native glass verified from a nested screenshot.

**Acceptance**

- The acceptance report states exactly which evidence is native and which is
  nested.

### R6.3 Reproducible visual fixtures

**Work**

- Add a fixture wallpaper/desktop with known luminance and a moving window
  behind the Dock.
- Capture the same fixture in nested and native modes.

**Acceptance**

- Blur sampling, panel edges, and specular rim can be compared across backends.

---

## R7 — Regression matrix

Run after R0 and after each material/scheduling change.

| Area | Checks |
|---|---|
| Rendering | light/dark; Full/Reduced/Minimal; 1080p/4K; single/multiple glass panels; native/nested equivalence; scale/rotation; resource reuse |
| Interactions | rapid Dock magnification; Applications drawer; popovers; Mission Control 10+ windows; multi-window minimize/restore; fullscreen direct scanout; lock/unlock; session restart |
| Performance | 60 Hz reference; 120/144 Hz; iGPU under glass/overview; dGPU 4K; idle CPU/GPU; no persistent memory/GPU growth |
| Compatibility | Qt/GTK; Xwayland; multi-output hotplug; suspend/resume; screenshots/screencast/portal boundaries; conformance/teardown soak |

Do not require every integrated GPU to render full refraction at 144/240 Hz.
Define a minimum acceptable tier per hardware class and measure it.

---

## R8 — Observability and documentation cleanup

### R8.1 Material stats

- Document `blur_draws` alongside `blur_passes` and `blur_panels`.
- Make clear that `path=gpu` is not proof and `blur_passes` only means the
  panels resolved.
- Add a draw-success counter if a future backend can skip a built element.

### R8.2 Presentation stats

- Document `presentation stats` fields and the 1.5x missed-deadline rule.
- Include a native capture in the acceptance report.

### R8.3 Docs

- Keep `docs/design/architecture-audit-native-rendering.md` as the architecture
  record.
- Keep this file as the work queue.
- Update the runbook with the default-on behavior and opt-out flags.
- Update `PROGRESS.md` with the native acceptance result when it lands.

---

## Suggested sequence

1. **R0** — native acceptance of the default-on P2 path.
2. **R1.1/R1.2** — panel-region rendering and stable IDs, with measurements.
3. **R4.1/R4.2** — persistent-chrome floor and hysteresis.
4. **R3.1/R3.2** — per-output deadline and presentation acceptance.
5. **R2.1/R2.2** — shell backend confirmation and HiDPI.
6. **R2.3** — shell copy reduction, only if measured as the bottleneck.
7. **R5** — same-GPU multi-output, then hotplug, then cross-GPU fallback.
8. **R6** — native visual evidence.
9. **R7/R8** — full matrix and documentation close-out.

Each slice should be independently testable and have a rollback:

- material: `DRAGONFRUIT_DRM_MATERIAL=0`
- glass: `DRAGONFRUIT_DRM_GLASS=0`
- shell: revert the transport change without touching the private protocol
- scheduling: `DRAGONFRUIT_REFRESH_HZ` pins a test rate; the 60 Hz fallback is
  the safe default until per-output deadlines are proven

---

## Open decisions

1. **Glass default.** It is default-on. If the 4K iGPU cannot meet the deadline,
   keep `DRAGONFRUIT_DRM_GLASS=0` as the documented fallback and define the
   minimum hardware class for Full.
2. **Persistent-chrome floor.** `Reduced` at worst for Dock/menu bar, or allow
   `Minimal` with a convincing milky plate? The audit recommends the latter as
   a perceptual floor, not a mandatory GPU-blur floor.
3. **Native capture.** Trusted portal path vs isolated development readback vs
   external evidence only.
4. **Shell transport.** Decided in
   [ADR 0191](design/adr/0191-chrome-transport-render-control-and-shm-fallback.md):
   `QQuickRenderControl` + reusable GPU buffers/dmabuf is the primary path;
   pooled `wl_shm` + frame callbacks is the fallback. The T-21.1a baseline
   confirmed the software backend and the per-commit copy cost.
5. **Cross-GPU.** Copy/import vs explicit safe fallback; do not introduce CPU
   readback without a measured requirement.
