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
| P3 shell transport | **not started** | Shell is software-rendered offscreen QPA; `grabWindow` + `QImage` + `wl_shm` per commit. |
| P4 refresh-aware scheduling | **not started** | Global 16 ms animation/degrade budget; output is 4K @ 144 Hz. |
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

**Work**

- Run a controlled shell restart with `QSG_INFO=1` and capture the backend,
  graphics API, and swapchain details.
- Record whether the offscreen QPA is software, GL, or RHI-backed.

**Acceptance**

- A committed diagnostic log states the active backend and API.
- No transport rewrite starts before this evidence exists.

### R2.2 HiDPI chrome

**Work**

- Render the shell at physical resolution with the correct buffer scale, or
  use a Qt-supported fractional-scale path.
- Apply the same treatment to the menu bar, Dock, popovers, and overlays.

**Acceptance**

- Dock/menu-bar text, icons, and rim are native-resolution at 1.5x and 2x.
- No layout or hit-testing regression at 1.0x, 1.5x, or 2x.

### R2.3 Remove full-surface copies

**Work**

- Avoid `grabWindow()` full-window re-render where a cached scene-graph frame
  can be reused.
- Add a small `wl_shm` buffer pool instead of `memfd`/`mmap`/`memcpy` per
  commit.
- Damage only the changed region; do not damage the whole buffer.
- Consider a GPU-backed `QQuickRenderControl` or Qt Wayland path only after
  R2.1 shows the shell is the bottleneck.

**Acceptance**

- One animated Dock frame performs at most one full-surface CPU copy.
- Copy count and bytes per frame are logged and measurably lower.
- The private layer protocol and shell restartability are unchanged.

**Do not** build a custom QPA plugin or a second UI renderer without the R2.1
evidence.

---

## R3 — Refresh-aware animation and scheduling (P4)

The compositor still uses a global 16 ms animation/degrade budget. The live
output is 4K @ 143.999 Hz, where the deadline is 6.94 ms.

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

**Acceptance**

- A 60 Hz animation on a 144 Hz output is not reported as healthy when it
  misses 144 Hz deadlines.
- A settled desktop produces zero presentation samples and zero damage.

### R3.3 Mixed-refresh and VRR

**Work**

- Schedule per output, not from a single global clock.
- Account for VRR state where the backend exposes it.
- Test a 60 Hz + 144 Hz mixed setup if hardware is available.

**Acceptance**

- A slow output does not throttle a fast output's animation cadence.
- No double-damage or clock drift when one output is idle.

---

## R4 — Material quality floor and degrade policy

The current controller still degrades the Dock and menu bar like transient
overlays.

### R4.1 Persistent-chrome floor

**Work**

- Keep persistent chrome at `Reduced` (blur on) at worst on the GPU path.
- Allow transient overlays to reach `Minimal`.
- If the GPU cannot meet the deadline even at `Reduced`, use the P1 milky
  fallback rather than a flat translucent plate.

**Acceptance**

- The Dock/menu bar never silently lose their material under load.
- A forced `Minimal` still reads as a deliberate milky plate.

### R4.2 Hysteresis and oscillation

**Work**

- Validate the 12-frame downgrade / 180-frame recovery policy at 60, 120, and
  144 Hz.
- Prevent Full↔Reduced oscillation during a steady pointer sweep.
- Ensure expensive glass is disabled or reduced quickly enough under pressure.

**Acceptance**

- No repeated tier flapping in a 60-second Dock animation.
- Tier transitions are visible in `degrade stats` and bounded.

### R4.3 Per-role/per-output material state

**Work**

- Extend `material_state` beyond the current global snapshot if persistent
  chrome and transient overlays need different effective tiers.
- Keep the event change-only and additive.

**Acceptance**

- The shell can choose the fallback per role without duplicating the
  compositor's degrade policy.

### R4.4 Fallback coverage

**Work**

- Apply the milky fallback to popovers, Control Center, notifications, OSD,
  and the Applications drawer where they currently assume blur.
- Keep the Dock/menu-bar fallback tokens as the source of truth.

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
- scheduling: keep the 16 ms path until the per-output path is proven

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
4. **Shell transport.** GPU-backed path only if `QSG_INFO=1` and copy/frame
   measurements show it is the bottleneck.
5. **Cross-GPU.** Copy/import vs explicit safe fallback; do not introduce CPU
   readback without a measured requirement.
