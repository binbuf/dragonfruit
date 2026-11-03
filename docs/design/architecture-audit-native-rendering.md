# Architecture audit — native DRM presentation, material, and shell transport

| | |
|---|---|
| **Status** | audit / P0 gate before implementation |
| **Date** | 2026-11-02 |
| **Scope** | native DRM presentation path, nested-vs-DRM material path, Qt shell buffer transport, Smithay 0.7 feasibility, resource/synchronization risks |
| **Evidence** | working tree at `6c1acd6`, the running native session, Smithay `=0.7.0` source, and the current generated tokens |
| **Related** | `docs/design/advisor-briefing.md`, `docs/design/proposals/native-drm-dock-glass.md`, ADR 0182, ADR 0186 |

> **Implementation status (2026-11-03).** P0.1 (windowed frame stats), P0.2
> (presentation instrumentation), P1.1/P1.2 (effective material state + milky
> fallback), P2A/P2B (DRM scene texture + Kawase blur), and P2C (glass compose)
> are implemented and the DRM sampled path is default on single-GPU,
> normal-transform outputs. **Still open:** P3 shell transport/HiDPI, P4
> refresh-aware scheduling, P5 multi-GPU/rotated material, and P6 native
> verification. The live-session numbers in §2/§3 are the pre-implementation
> baseline; do not read them as current. See
> [`STATUS.md`](STATUS.md) and
> [ADR 0189](adr/0189-drm-sampled-material-default-and-fallbacks.md).
>
> **Execution (v0.1.1).** The open P3/P4/P5/P6 work is cut into
> [Track 21](tracks/21-presentation-foundation.md) and tasks
> [T183–T193](../tasks/0.1.1/README.md): shell transport/HiDPI, per-output
> refresh scheduling, material cost/floor, and native evidence.

This is the report requested before implementation. It does not change the
renderer. It records what was actually running at the audit point, corrects
two premises in the briefing, and identifies the smallest defensible
implementation sequence.

> **Implementation note (same change).** P0.1 and the DRM presentation-interval
> half of P0.2 are implemented in `compositor/src/state.rs` and
> `compositor/src/backend/drm.rs`. The live session predates the change; it
> needs a rebuild/restart before the new `presentation stats` line can be read.
> The `frame timing` line is append-only: existing fields keep their names, the
> average is now windowed, and `samples_total`, `p50_us`, `p95_us`, `p99_us`,
> and `window_max_us` are appended.
>
> **P1 implementation note.** P1.1/P1.2 also landed: `df_output.material_state`
> (v3) reports the effective material, `EffectiveMaterial` distinguishes the
> sampled pass from the feather path, and the Dock/menu bar switch to
> token-driven milky fallbacks when `blur_active=0`. QML tests cover light/dark
> fallback and the translucent-glass complement. The native session still needs
> a rebuild/restart to receive the new event from the new compositor.
>
> **P2 implementation note.** The DRM scene-texture + Kawase path and the glass
> compose are the default. `DRAGONFRUIT_DRM_MATERIAL=0` and
> `DRAGONFRUIT_DRM_GLASS=0` are opt-outs for diagnostics or rollback. The path
> is single-GPU, normal-output-transform, and local-route only. A failed
> material frame disables the sampled path for the session and falls back to
> the feather stack; `blur_draws` counts elements that were actually built and
> handed to `DrmCompositor`. Native hardware acceptance (visual sampling, frame
> deadline, cursor/fullscreen regressions) is still required.

---

## 1. Verdict

The advisor's executive decision is sound: **retain Rust/Smithay 0.7, Qt 6.11,
and the compositor/shell split.** Nothing in the repository or the live session
supports replacing the compositor with Skia, Impeller, Flutter, or a custom Qt.

The audit also found that the current native baseline is **not yet trustworthy
enough to call healthy or unhealthy**:

1. The installed and running native session is already **release**, not debug.
   The 494 ms sample is historical. The release session is dramatically better
   than the old debug session, but it is still not meeting the panel's
   144 Hz deadline.
2. `frame timing` is **not a windowed statistic**. Its `avg_us` divides a
   lifetime accumulator by the 8192-sample ring length, so it grows without
   bound after the ring fills. It also reports no percentiles, and its
   `max_interval_ms` includes idle gaps.
3. The panel is **3840x2160 at 143.999 Hz**, not a 60 Hz reference. The
   compositor's 16 ms animation budget and degrade budget are therefore the
   wrong deadline for this session.
4. The shell is **software-rendered** on the offscreen QPA and transports each
   frame as a fresh `wl_shm` buffer. That is a material part of the frame-time
   story, not a cosmetic detail.

The root cause of `blur_passes=0` is unambiguous: the DRM render path never
calls `chrome_blur_panels`. It calls only the feather
`chrome_backdrop_render_elements`. The native session is not failing to blur;
it is not asking for the blur pass at all.

The smallest viable path to a correctly blurred native Dock is **P0
instrumentation → P1 fallback → P2A offscreen scene pass → P2B one-panel
Kawase blur**, with the glass details after that. The proposal's P2 sketch is
directionally right, but its renderer abstraction does not compile against
Smithay 0.7 as written; section 7 explains the correction.

---

## 2. What was running at the audit point

> Historical baseline (2026-11-02). The DRM material path and presentation
> counters below were the pre-P2 state; see the status banner above.

Captured from the live session on 2026-11-02:

```text
$ readlink -f /usr/bin/dragonfruit-compositor
/home/user/repos/binbuf/dragonfruit/target/release/dragonfruit-compositor
$ readlink -f /usr/bin/dragonfruit-shell
/home/user/repos/binbuf/dragonfruit/build-release/shell/src/dragonfruit-shell

$ bash scripts/native-session.sh status
native-session: selected profile: release
native-session: installed profile: release
dragonfruit-compositor.service     active
dragonfruit-shell.service          active
```

The running compositor was started at 21:17:27. At 21:32:08 it reported:

```text
render stats:  frames_rendered=83916 frames_skipped_no_damage=176
               direct_scanouts=0 animation_frames_stepped=109
               client_wakeups=248931 latency_us_last=6966
               latency_us_max=246321 latency_samples=46106 latency_dropped=54
material stats: backdrop_passes=84087 backdrop_skipped=0 path=gpu
                blur_passes=0 blur_panels=0 blur_downsample_max=0
degrade stats: tier=full forced=0 budget_us=16000 samples=83914
               over_budget=41 downgrades=2 upgrades=2
               tiers=full:83548,reduced:366,minimal:0
frame timing:  samples=8192 avg_us=45005 max_us=128500 max_interval_ms=59066
```

The output reported by the compositor's Wayland server:

```text
name: DP-1
scale: 2 (wl_output integer; the shell's logical configure is 2560x1440,
         which is 3840x2160 / 1.5 — the compositor uses a fractional scale)
mode: 3840x2160 px, refresh: 143.999 Hz, current preferred
output_transform: normal
```

The shell logs `Dock configured 2560x125` and `menu bar configured 2560x28`.
Those are logical sizes. The shell does **not** set a buffer scale; the
compositor upscales the chrome to the physical output. That is a separate
visual-quality defect discussed in section 9.

The old session (before the release install) is the one the briefing describes:

```text
frame timing (exit): samples=8192 avg_us=348024 max_us=4514006
                     max_interval_ms=13487068
degrade stats (exit): tier=reduced ...
                      tiers=full:32739,reduced:270621,minimal:10069
render stats (exit): frames_rendered=313432
```

That session ran for 5 h 46 min, so its coarse rendered-frame rate was about
15 fps. The current release session rendered 83,916 frames in 882 s, about
95 fps. The comparison is coarse (idle time is in the denominator, and
`frames_rendered` is not a presentation counter), but the direction is clear:
**release fixed the catastrophic debug regression; it did not produce a
144 Hz desktop.**

---

## 3. Native presentation path

The exact chain for one DRM output:

| Stage | File / function | What it does |
|---|---|---|
| 1. Event loop | `compositor/src/session.rs`, `run_session` loop (`dispatch`, then `render`) | Dispatches Wayland/input/timers, then calls the backend render hook. |
| 2. Frame timing seam | `compositor/src/session.rs:171-202` | `frame_start = Instant::now()` before `render`; after `render`, if `frames_rendered` changed, calls `DfState::observe_rendered_frame`. |
| 3. Backend dispatch | `compositor/src/backend/drm.rs`, `render_pending` | Calls `render_surface` for each output whose frame clock fired. |
| 4. Renderer acquisition | `drm.rs:1224`, `GpuManager::single_renderer(&render_node)` | Produces the `UdevRenderer` (`MultiRenderer<GbmGlesBackend<GlesRenderer>, …>`). |
| 5. Material pass guard | `drm.rs:1239`, `state.begin_render_frame()` | Opens one material pass per output-frame. |
| 6. Element construction | `drm.rs:1252-1353` | Lock, cursor, chrome, feather backdrop, window menu, window stack, desktop, wallpaper. |
| 7. DRM composition | `drm.rs:1357-1360`, `surface.drm_output.render_frame(...)` | `DrmCompositor::render_frame` walks the element list front-to-back, assigns planes, direct-scans out when eligible, otherwise composites. |
| 8. Queue | `drm.rs:1399`, `surface.drm_output.queue_frame(Some(feedback))` | Queues the page flip and presentation feedback. |
| 9. Vblank completion | `drm.rs:1029-1190`, `frame_finish` | Reads `DrmEventMetadata`, throttles broken timestamps, calls `frame_submitted()`, delivers `wp_presentation_feedback.presented(...)`. |
| 10. Client wakeups | `drm.rs:1410-1416`, `render::post_repaint` | Sends `wl_surface.frame` callbacks and updates chrome scale. |

The native element stack in `render_surface` is, top to bottom:

```text
lock surface (if locked)
pointer/cursor
chrome surfaces (menu bar, Dock, overlays)
chrome_backdrop_render_elements (feather solid colors)
window menu
window scene (titlebar + client surface + shadow per window)
desktop layer
wallpaper
```

At the audit point there was no `chrome_blur_panels` call, no offscreen scene
texture, no `BackdropBlurElement`, and no `GLASS_SHADER` in this path. The P2
implementation added them; this paragraph records the pre-P2 state.

`DrmOutputElements` (`drm.rs:85-97`) has variants for `Pointer`, `Space`,
`Chrome`, `Decoration`, `Appear`, and `Wallpaper`. It has no `Blur` or `Scene`
variant.

---

## 4. Nested presentation path (the reference)

The nested path is the only implemented two-pass material:

| Stage | File / function | What it does |
|---|---|---|
| 1. Resolve panels | `nested.rs:417`, `render::chrome_blur_panels` | Applies the material pass guard and records blur stats. |
| 2. Scene list | `nested.rs:426-442` | Window menu, window scene, desktop, wallpaper. |
| 3. Scene texture | `nested.rs:443-450`, `render_scene_offscreen` | Renders the scene into a cached output-sized `GlesTexture`. |
| 4. Chrome | `nested.rs:454-460` | Lock + chrome surfaces above the blur. |
| 5. Blur elements | `nested.rs:461-469`, `build_blur_elements` | Per-panel Kawase chain + optional glass program. |
| 6. Scene composite | `nested.rs:472-480` | The scene texture itself is appended below the blur. |
| 7. Output | `nested.rs:535-541` | `OutputDamageTracker::render_output` into the winit framebuffer. |

Key implementation details:

- `MaterialTextureCache` (`nested.rs:303-317`) owns the scene texture and one
  `Vec<GlesTexture>` per panel. Textures are reused across frames and only
  reallocated when the size changes.
- `render_scene_offscreen` (`nested.rs:613-655`) allocates with
  `renderer.create_buffer(Fourcc::Abgr8888, buffer_size)` and binds with
  `renderer.bind(&mut texture)`.
- `blur_panel_texture` (`nested.rs:701-776`) halves the source each pass,
  reuses `slot[pass]`, and calls `GlesFrame::render_texture_from_to` with the
  custom `BLUR_SHADER` uniforms.
- `BackdropBlurElement` (`window/blur.rs:433-554`) draws the blurred texture
  clipped to `rounded_rect_spans`, optionally with `GLASS_SHADER`.
- The nested backend is `GlowRenderer`; its `TextureId` is `GlesTexture`, which
  is why the existing `BlurRenderer: Renderer<TextureId = GlesTexture>` trait
  works there.

This path is a working reference, but it is not directly portable to the DRM
renderer type. Section 7 covers the exact mismatch.

---

## 5. Shell buffer-production and transport path

The shell is a Qt Quick process on the **offscreen QPA**. `main.cpp:30-33`
forces `QT_QPA_PLATFORM=offscreen`. The live shell maps no `libGLESv2`, `libGL`,
or DRI driver and runs six threads, which strongly indicates the software
scene graph; confirm it with `QSG_INFO=1` in a controlled restart before
committing to a GPU transport rewrite. The QML tests explicitly set
`QT_QUICK_BACKEND=software`, but the production shell does not set it, so the
live confirmation matters.

For one Dock frame:

1. `QQuickWindow::afterRendering` fires.
2. `ShellController::onDockAfterRendering` (`shellcontroller.cpp:6999-7008`)
   calls `scheduleDockRender`.
3. `scheduleDockRender` (`:7010-7019`) posts a `QTimer::singleShot(0)` and
   calls `renderDock`.
4. `renderDock` (`:7021-7142`):
   - sets the Dock item geometry;
   - resizes the offscreen window to `m_dockWidth + 2 * 320` by
     `m_dockHeight + 320` (the fixed popover headroom/gutter);
   - calls `m_dockWindow->grabWindow()` (`:7064`). **`grabWindow()` renders the
     scene a second time**; `FrameCommitGate` suppresses the re-entrant
     `afterRendering`.
   - calls `image.copy(leftGutter, headroom, m_dockWidth, m_dockHeight)`
     (`:7098-7099`), a deep copy of the bar;
   - sends the panel rect and input region.
5. `ShellProtocol::commitDockImage` (`shellprotocol.cpp:1369-1377`) calls
   `commitTo`.
6. `ShellProtocol::commitTo` (`shellprotocol.cpp:1574-1635`):
   - `image.convertToFormat(QImage::Format_ARGB32_Premultiplied)`;
   - `createShmFile(size)` — `memfd_create` or `/dev/shm` `O_TMPFILE`;
   - `mmap`, `memcpy(data, frame.constBits(), size)`, `munmap`;
   - `wl_shm_create_pool`, `wl_shm_pool_create_buffer`, destroy pool, close fd;
   - `wl_surface_attach`, `wl_surface_damage_buffer` (whole buffer),
     `wl_surface_commit`, `wl_display_flush`;
   - `onBufferRelease` destroys the `wl_buffer` when the compositor releases
     it.
7. The compositor imports the SHM buffer through Smithay's
   `WaylandSurfaceRenderElement` during `chrome_render_elements`; the import
   uploads/copies the pixels to a GPU texture for that commit.
8. The Dock popover repeats steps 4–7 on a second overlay surface with
   `commitDockPopupImage` (`:1449-1457`). The menu bar repeats the same pattern
   in `ShellController::render` (`:7144-7169`).

Copies and allocations per animated Dock frame, with the observed logical
Dock size `2560x125` and headroom/gutter `320`:

| Copy / allocation | Size (logical pixels) | Approx. bytes |
|---|---:|---:|
| `grabWindow()` offscreen render | 3200 x 445 | 5.70 MB |
| `image.copy(bar)` | 2560 x 125 | 1.28 MB |
| `convertToFormat` | usually a shallow no-op for ARGB32_Premultiplied | 0 |
| `memcpy` to `wl_shm` | 2560 x 125 | 1.28 MB |
| compositor SHM import/upload | 2560 x 125 | 1.28 MB |
| popover (when open) | up to ~1.2 MB | up to ~1.2 MB |
| popover SHM memcpy + import | up to ~1.2 MB each | up to ~2.4 MB |

At the observed ~95 submitted frames/s, the bar alone is roughly 120 MB/s of
`grabWindow` output plus 240 MB/s of copy/upload traffic, before the popover.
The whole Dock offscreen render is 5.7 MB per frame, about 540 MB/s at 95 fps.
This is CPU and memory-bandwidth work on the critical path.

Pacing properties:

- The shell does not wait for `wl_buffer.release` before producing the next
  frame. It creates a fresh buffer per commit; if the compositor is slower than
  the Qt scene graph, buffers can queue until the compositor releases them.
- The shell does not use `wl_surface.frame` callbacks or presentation feedback
  to pace QML animation. Qt advances on its own scene-graph/timer cadence, and
  the Dock bounce uses a 16 ms `QTimer` (`shellcontroller.cpp:1728-1730`).
- The shell does not set `wl_surface.set_buffer_scale`; at a 1.5x output the
  logical-resolution buffer is upscaled by the compositor.

This answers the P3 questions: the shell is software-rendered, the transport is
`QImage` → `wl_shm` with a full-surface copy per commit, there are multiple
copies, and there is no presentation-aligned clock. A GPU-backed path is
justified **if** a controlled `QSG_INFO=1` run confirms the software backend and
the baseline shows the shell on the critical path. Do not build a custom QPA
plugin or a second UI renderer before that measurement.

> **T-21.1a measurement (2026-11-03).** The controlled `QSG_INFO=1` run
> confirms `Loading backend software`; the live shell maps the EGL/GLX loader
> but no DRI driver. On the nested 1920×1200 baseline, the slow Dock sweep
> produced 291 chrome commits / 268,929,360 shm bytes (960,000 bytes for the
> 1920×125 Dock bar alone), with `commit_us` p50/p95/p99 = 1069/1653/1943 µs
> and compositor `frame timing` p50/p95/p99 = 10569/18445/23422 µs. Idle was
> zero-wakeup. The decision is
> [ADR 0191](adr/0191-chrome-transport-render-control-and-shm-fallback.md):
> `QQuickRenderControl` + reusable GPU buffers/dmabuf as the primary path,
> pooled `wl_shm` + frame callbacks as the fallback. The full report is
> [`../captures/t21-chrome-baseline.txt`](../captures/t21-chrome-baseline.txt).

---

## 6. Why `blur_passes=0` at the audit point

`chrome_blur_panels` is the only function that increments `blur_passes`
(`render.rs:321-390`, `record_blur_pass` at `:388`). The DRM path never calls
it. It calls `chrome_backdrop_render_elements` (`render.rs:222-297`), which
resolves `tier.backdrop(...)` and emits `SolidColorRenderElement`s. At
`Minimal`, even that returns empty.

The counters prove the absence, but they also have a semantics problem:

> `chrome_blur_panels` records `blur_passes` **before** the caller builds and
> draws the blur elements. A future DRM implementation could report
> `blur_passes>0` even if the element draw fails or the element is skipped by
> plane assignment. `blur_passes` is currently "panels resolved and the
> material pass was claimed", not "GPU blur executed".

For P2 acceptance, add a draw-success counter (or move the increment to the
successful draw/frame path) and report both. Do not accept `blur_passes>0`
alone as proof.

---

## 7. Smithay 0.7 API feasibility

Checked against the pinned `smithay-0.7.0` source in the cargo registry.

### 7.1 What works

- `MultiRenderer` implements `Offscreen<Target>` and `Bind<Target>` when both
  underlying renderers do (`multigpu/mod.rs:949-1033`). `GlesRenderer`
  implements both for `GlesTexture` (`gles/mod.rs:1511-1597`). Therefore, on a
  single-GPU DRM session, `UdevRenderer::create_buffer(Fourcc::Abgr8888, size)`
  and `UdevRenderer::bind(&mut GlesTexture)` are available.
- `MultiFrame` implements `AsMut<GlesFrame>` (`multigpu/mod.rs:929-947`). A
  custom render element can therefore obtain the underlying `GlesFrame` and
  call `render_texture_from_to` with a `GlesTexture`, the compiled blur/glass
  program, and the uniforms.
- `DrmCompositor::render_frame` is generic over `E: RenderElement<R>`
  (`drm/compositor/mod.rs:1684-1695`). A custom element can be added to the
  `DrmOutputElements` enum; the compositor will composite it rather than
  requiring a `TextureRenderElement`.
- `MultiRenderer` is `Renderer<TextureId = MultiTexture>`
  (`multigpu/mod.rs:1049`). `MultiTexture` is the cross-GPU texture handle; a
  raw `GlesTexture` is local to the render GPU.

### 7.2 What does not work as the proposal describes

The proposal says:

> Both `GlowRenderer` and `UdevRenderer` satisfy these bounds.

That is true for `Offscreen<GlesTexture>` + `Bind<GlesTexture>`, but **not** for
the existing `BlurRenderer` trait:

```rust
pub trait BlurRenderer: Renderer<TextureId = GlesTexture> { ... }
```

`UdevRenderer` has `TextureId = MultiTexture`, so it cannot implement that
trait. Likewise:

- `BackdropBlurElement` implements `RenderElement<R>` only for
  `R: BlurRenderer`; it cannot be drawn by `MultiRenderer` as written.
- `TextureRenderElement<GlesTexture>` implements `RenderElement<R>` for
  renderers whose `TextureId` is `GlesTexture`; it cannot be used as the
  `Scene` variant for `UdevRenderer`.
- `DrmOutputElements` is generated by `render_elements!` with
  `R: ImportAll + ImportMem + ImportMemWl + ImportDmaWl + ImportDma`; adding
  `Blur`/`Scene` variants requires those element types to implement
  `RenderElement<UdevRenderer>`.

The viable correction:

1. Change `BlurRenderer` to a renderer-agnostic trait with a method that takes
   `&GlesTexture` and draws through the renderer's frame. Do not constrain
   `TextureId = GlesTexture`.
2. Implement it for `GlowRenderer` (existing `BorrowMut<GlesFrame>` path) and
   for the concrete `MultiRenderer<GbmGlesBackend<GlesRenderer>, …>` using
   `AsMut<GlesFrame>`.
3. Add a custom `SceneTextureElement` that wraps a `GlesTexture` and draws via
   the same `AsMut<GlesFrame>` path; do not use
   `TextureRenderElement<GlesTexture>`.
4. Add `Blur` and `Scene` variants to `DrmOutputElements`, with the same
   front-to-back ordering as nested.
5. Keep the nested path unchanged; prove the DRM path with a separate branch.

### 7.3 Other API constraints

- `DrmCompositor` may direct-scanout an element only if its underlying storage
  and kind allow it. A custom full-output scene texture will be composited, not
  scanned out. When blur is active, expect to lose direct scanout for that
  output-frame. Keep the no-blur path for fullscreen direct scanout and test it.
- The scene texture is output-local. `DrmCompositor` applies the output
  transform; the custom element geometry must be in output-local physical
  coordinates and the texture sampled with the right orientation. The nested
  path uses `Transform::Normal` for the scene texture and `Flipped180` only for
  the winit back buffer; do not copy the winit flip into DRM.
- `OutputDamageTracker::render_output` can render into a `MultiFramebuffer`
  obtained from `MultiRenderer::bind`; the scene pass and the final output pass
  must remain separate. Do not call `DrmCompositor::queue_frame` for the
  offscreen pass.
- Cross-GPU sampling is not available for a raw `GlesTexture`. The existing
  `multi_gpu::SceneRoute` is a decision/plan, not an implementation. The first
  DRM blur must be `route=local` only; `route=copy` or `route=software` falls
  back to the P1 milky material.

---

## 8. Resource-allocation and GPU-synchronization risks

### 8.1 Allocation

- A full 3840x2160 ABGR8888 scene texture is **33.2 MB**. At 1.5x logical
  scale the compositor still renders physical mode size. Two outputs are
  66 MB+, before blur chains.
- The nested `MaterialTextureCache` is per `NestedData` (one output). DRM needs
  a cache per **output and GPU**, keyed by mode/scale, not a single global
  `Option<GlesTexture>`.
- `blur_panel_texture` reuses per-pass textures, but the DRM implementation
  must not share one panel slot vector across outputs with different sizes.
- `chrome_blur_panels` returns `Vec<BlurPanel>` per frame; that allocation is
  small, but the element list and texture clones are per frame. Keep the
  texture cache alive across frames; do not drop the cache while an element
  still holds a clone.
- The current nested path allocates the scene texture only on size change, but
  it still **renders the full scene every frame** with damage `0`. At 4K this
  is a full-output write plus a full-output composite. Panel-region-only
  rendering (union of blur panels plus blur-radius padding) is a later
  optimization, but the first DRM slice must measure the full-output cost
  before claiming 60/144 Hz.

### 8.2 Synchronization

- `MultiFrame::finish` inserts a sync point when the target/render split
  requires it (`multigpu/mod.rs:1373-1406`). The scene texture must be finished
  before the blur samples it; `render_scene_offscreen` already drops the
  `bind`/`render_output` target before returning.
- A cross-GPU copy would need explicit synchronization and a `MultiTexture`
  import; do not attempt it in P2A/P2B.
- The DRM frame's `queue_frame` must happen exactly once per output-frame.
  The offscreen scene pass must not queue a frame.
- The cursor element is first in the list; adding a full-output scene texture
  should not disturb cursor-plane assignment, but verify cursor latency and
  hardware cursor behavior on hardware.
- Damage tracking: the scene texture is a full-output element with a stable
  `Id`; if its content changes every frame, it damages the full output. Keep a
  stable element `Id` and use the `OutputDamageTracker` result; do not force a
  full damage frame unless the scene actually changed.
- Fullscreen direct scanout, overlay ordering, and frame callbacks are
  regression risks; the P2A acceptance must include a fullscreen client and a
  cursor sweep.

---

## 9. Instrumentation and baseline defects

These are P0 blockers, independent of the renderer.

### 9.1 `frame timing avg_us` is invalid after the ring fills

Before this change, `RenderStats::record_frame` added every duration to
`frame_time_us_total`, and `RenderStats::frame_summary` divided that lifetime
total by `samples.len()`, which is capped at `FRAME_TRACE_CAPACITY = 8192`.
After 8192 frames, `avg_us` was `lifetime_total / 8192`, not the average of the
current window. The live `avg_us=45005` is therefore not a 45 ms average of the
last 8192 frames. The `docs/PROGRESS.md` T-20.4 note that quotes this value
predates the fix; treat it as a lifetime-total artifact, not a steady-state
measurement.

The fix is implemented: the summary is computed from the ring, with
`samples_total`, p50/p95/p99, and `window_max_us` appended. The `frame timing`
line remains append-only: `samples`, `avg_us`, `max_us`, and `max_interval_ms`
keep their names for existing consumers, while `avg_us` now describes the
window.

### 9.2 No presentation interval or dropped-frame instrument (implemented)

`frame timing max_interval_ms` is the gap between rendered frames, including
idle gaps. The live `59066 ms` is an idle gap, not a presentation interval.
`frame_finish` has the real vblank timestamp (`DrmEventMetadata::time`) and
delivers `wp_presentation_feedback`, but the compositor kept no internal
per-output presentation-interval ring, no p95/p99, and no missed-deadline
count. The implementation now records a `presentation stats` line per DRM
output from the actual vblank interval, only when a queued frame was submitted;
intervals over 1.5x the mode period count as missed. A native rebuild/restart
is required before the live session reports it.

### 9.3 CPU duration is not GPU duration or presentation

`session.rs:171-202` times the whole `render` hook. It includes element
construction, `DrmCompositor::render_frame`, and any blocking inside it. It
does not separate CPU submission from GPU execution or from the vblank
interval. A frame can be fast on CPU and still miss the deadline.

### 9.4 60 Hz constants on a 144 Hz output

- `animation::FRAME_INTERVAL = 16 ms` (`animation.rs:32`).
- `AnimationClock::frame_interval` returns that constant (`animation.rs:226-230`).
- `input::schedule_animation_timer` arms a 16 ms timer (`input.rs:1018-1038`).
- `DegradeController::new` defaults the budget to `FRAME_INTERVAL`
  (`degrade.rs:280-286`).

At 143.999 Hz the deadline is 6.94 ms. A 16 ms budget accepts frames that miss
two or three vblanks. The controller's 12/180 hysteresis is not inherently
wrong, but it is being applied against the wrong deadline and with no
per-output awareness.

### 9.5 Shell HiDPI

The shell receives logical `2560x1440` at a 1.5x output scale, renders at that
logical size, and commits a buffer with no `wl_surface.set_buffer_scale`. The
compositor upscales the chrome by 1.5. This softens text, icons, and the rim
independently of the material path. Fix this before judging P1/P2 visual
quality: either render the shell at physical resolution with the correct
buffer scale, or set an explicit fractional scale path that Qt supports.
This is not a reason to rewrite the shell; it is a correctness fix.

---

## 10. Smallest viable path to a correctly blurred native Dock

The smallest path that proves the real thing, in order:

1. **P0.1** Fix the frame summary and add windowed percentiles (pure stats).
2. **P0.2** Add per-output presentation-interval and missed-deadline
   instrumentation; run a release baseline at idle, Dock sweep, app launch,
   and Mission Control with 10+ windows.
3. **P1.1** Add an additive compositor → shell `material_state` event carrying
   effective tier, blur-active, fallback reason, and output/role. Emit only on
   change.
4. **P1.2** Add milky fallback tokens and make the Dock/menu bar choose the
   fallback from the effective state, not from a duplicated policy.
5. **P2A** On DRM, render the scene into a cached `GlesTexture` with glass
   disabled, composite it as a custom `SceneTextureElement`, and prove the
   output is pixel-equivalent to the feather path. No blur yet.
6. **P2B** Add the Kawase chain for one stationary Dock panel on one GPU and
   one output. Reuse `BLUR_SHADER`; no refraction. `route=local` only.
7. **P2C** Add the glass shader (SDF lens, specular, adaptive tint) for
   `Full`/`Reduced`.

P2A/P2B is the smallest native blur milestone. It deliberately excludes
multi-GPU, multi-output, refraction, panel-region optimization, and native
capture.

---

## 11. Proposed commits / tasks with acceptance criteria

### P0.1 — windowed frame statistics (implemented)

**Files:** `compositor/src/state.rs`, tests.

- Correct `avg_us` to the current ring.
- Add `samples_total`, `p50_us`, `p95_us`, `p99_us`, `window_max_us`.
- Keep `max_us` as lifetime max for compatibility.
- Unit test with more than `FRAME_TRACE_CAPACITY` samples.

**Acceptance:** a test fills the ring with 1 ms frames and then 100 ms frames;
`avg_us` is 100000, not 101000; p50/p95/p99 match the sorted window; empty
stats produce a parseable line.

### P0.2 — presentation instrumentation (instrumentation implemented; baseline pending rebuild)

**Files:** `compositor/src/backend/drm.rs`, `compositor/src/state.rs`,
`compositor/src/render.rs`.

- Record the vblank interval per output from `frame_finish`'s monotonic
  timestamp, only when `frame_submitted()` returns a queued frame.
- Count intervals over `1.5 * frame_duration` as missed.
- Add a `presentation stats` line with `samples`, `p50_us`, `p95_us`,
  `p99_us`, `max_us`, `missed`, and `refresh_hz`.
- Keep idle gaps out of the interval ring.

**Acceptance:** on a 144 Hz output, a 60 Hz animation reports ~16.7 ms
intervals and misses; a 144 Hz animation reports ~6.94 ms and near-zero
misses; an idle desktop adds no samples.

### P1.1 — effective material state (implemented)

**Files:** `protocols/dragonfruit-shell.xml`, generated protocol bindings,
`compositor/src/shell/mod.rs`, `compositor/src/render.rs`,
`shell/src/shellprotocol.{h,cpp}`, `shell/src/shellcontroller.{h,cpp}`.

- Add an additive `df_shell.material_state` event (version bump) with
  `output`, `role`, `tier`, `blur_active`, and `fallback_reason`.
- Compute it from `state.material_path.material_tier(state.degrade.tier())`
  **and** whether the backend actually draws the sampled pass. DRM with no
  blur implementation must report `blur_active=0`, not `path=gpu`.
- Emit on change only.

**Acceptance:** a protocol conformance test observes one event per effective
change; the shell exposes the state to QML; no event is emitted per frame.

### P1.2 — milky fallback (implemented)

**Files:** `design-system/tokens/tokens.json`, generated `Theme.qml` /
`design_tokens.rs`, `shell/dock/Dock.qml`, `shell/tests/tst_dock.qml`.

- Add light/dark fallback fill/opacity tokens under `component.dock.plate`
  (and the menu bar equivalent).
- When `blur_active=0`, draw a near-opaque milky neutral, bright inner rim,
  anchored-edge lip, border, and shadow; keep rounded geometry and elevation.
- When `blur_active=1`, keep the current translucent complement.
- Do not hardcode `#E5E5E5`; it is a composited reference, not a fill token.

**Acceptance:** forcing Full/Reduced/Minimal preserves a deliberate Dock in
light and dark; `tst_dock` asserts the fallback is opaque enough to read as a
material; gallery goldens and `tst_design_system` stay green.

### P2A — DRM offscreen scene pass (implemented, default on)

**Files:** `compositor/src/backend/drm.rs`,
`compositor/src/window/blur.rs` (or a new generic helper),
`compositor/src/render.rs`.

- Add a per-output/per-GPU `MaterialTextureCache`.
- Generalize `render_scene_offscreen` over
  `Renderer + Offscreen<GlesTexture> + Bind<GlesTexture>`.
- Add a custom `SceneTextureElement` drawn through `AsMut<GlesFrame>`.
- Add `DrmOutputElements::Scene`; branch when `chrome_blur_panels` is
  non-empty. Keep the feather branch when empty.
- Glass off, blur off; visual output must be equivalent.

**Acceptance:** native render shows the scene pass in a new counter; no visual
difference from the feather path; no direct-scanout regression for a
fullscreen client; no texture allocation after the first frame at a fixed
mode/scale; rotation/scale test.

### P2B — DRM Kawase blur (implemented, default on)

**Files:** `compositor/src/backend/drm.rs`,
`compositor/src/window/blur.rs`, `compositor/src/render.rs`.

- Make `BlurRenderer` renderer-agnostic; implement it for `GlowRenderer` and
  the concrete `MultiRenderer`.
- Add `DrmOutputElements::Blur`; build per-panel textures with the existing
  cache pattern.
- One GPU, one output, one stationary Dock panel, `route=local`, no
  refraction, no multi-GPU.

**Acceptance:** `blur_passes>0` **and** a new `blur_draws>0`/draw-success
counter; the blurred panel visibly samples live desktop/window content behind
the Dock; 4K release meets the configured deadline at 60 Hz and is measured at
144 Hz; nested output unchanged; no per-frame texture allocation; no cursor
regression.

### P2C — glass details (implemented, default)

- Reuse `GLASS_SHADER`, `GlassSpec`, and the generated tokens; no native-only
  aesthetic.
- `Full`: refraction + specular + tint. `Reduced`: specular + tint, no
  refraction. `Minimal`: no sampled pass, P1 fallback.

**Acceptance:** Full/Reduced/Minimal preserve a deliberate Dock in both
themes; no duplicate shader business logic; the glass pass is measurable
separately from the blur pass.

### P3 — shell transport/HiDPI (OPEN; the next P0 unit)

- First fix HiDPI buffer scale and reduce copies (buffer pool, dirty-rect
  damage, avoid `grabWindow` full-window re-render).
- Then, if `QSG_INFO=1` confirms software rendering and the baseline shows the
  shell on the critical path, prototype a GPU-backed `QQuickRenderControl` or
  Qt Wayland path. Do not build a custom QPA plugin or a second renderer
  without that evidence.

**Acceptance:** a before/after frame-time and copy-count report; no change to
the private layer protocol; the shell remains independently restartable.

### P4 — refresh-aware scheduling (OPEN)

- Replace the global 16 ms budget with a per-output refresh deadline.
- Animation progress remains monotonic elapsed time; scheduling uses the
  output's presentation deadline and VRR state.
- Do not replace 16 ms with a global 8 ms.

**Acceptance:** 60/120/144/240 Hz table in section 12; p95/p99 and missed
deadlines reported; mixed-refresh test; no idle damage when animations settle.

### P5/P6 — multi-GPU and native verification (OPEN)

- P5: local-only first, then same-GPU multi-output, then hotplug/mode changes,
  then explicit cross-GPU copy or a reported fallback. Never sample across
  GPUs.
- P6: trusted, explicitly enabled native capture or external photographs;
  nested captures are reference evidence only. Never label native glass
  verified from a nested screenshot.

---

## 12. Required regression matrix (from the advisor, mapped to this repo)

| Area | Checks |
|---|---|
| Rendering | light/dark; Full/Reduced/Minimal; 1080p/4K; single/multiple glass panels; native/nested equivalence; scale/rotation; resource reuse |
| Interactions | rapid Dock magnification; Applications drawer; popovers; Mission Control 10+ windows; multi-window minimize/restore; fullscreen direct scanout; lock/unlock; session restart |
| Performance | 60 Hz reference; 120/144 Hz; iGPU under glass/overview; dGPU 4K; idle CPU/GPU; no persistent memory/GPU growth |
| Compatibility | Qt/GTK; Xwayland; multi-output hotplug; suspend/resume; screenshots/screencast/portal boundaries; conformance/teardown soak |

Frame deadlines:

| Refresh | Deadline |
|---|---|
| 60 Hz | 16.67 ms |
| 120 Hz | 8.33 ms |
| 144 Hz | 6.94 ms |
| 240 Hz | 4.17 ms |

Averages alone are insufficient; report p50/p95/p99 and missed deadlines.

---

## 13. Decisions still needed

> **Resolved:** 1 (milky neutral fallback + `material_state`), 2 (P2A/P2B
> cheap DRM blur first), and 3 (glass port after the blur) are implemented.
> **Still open:** the quality floor, the shell transport/HiDPI fix, and native
> verification. See [`STATUS.md`](STATUS.md).

1. **P1 fallback design:** milky neutral (recommended), wallpaper-tinted
   sample, or a cheap static approximation? **Resolved:** milky neutral,
   tokenized, driven by `df_output.material_state`.
2. **Quality floor:** should persistent chrome stay at `Reduced` (blur on)
   while transient overlays may reach `Minimal`, or is `Minimal` acceptable for
   the Dock if the fallback is convincing? The advisor recommends a perceptual
   floor, not a mandatory GPU-blur floor. This audit agrees. **Still open
   (G-12).**
3. **P2 scope:** ship the cheap DRM blur first and defer refraction (P2A/P2B),
   or port the full glass shader at once? **Resolved:** P2A/P2B first, then
   P2C glass; both are implemented.
4. **Shell transport:** fix HiDPI and copy count before P2, or after? **Still
   open (G-01).** The audit recommends fixing HiDPI before judging visual
   quality, but not rewriting the transport until P0.2 measures it. The
   measurement now exists; this is the next P0 unit.
5. **Native verification:** trusted capture vs external photographs vs
   diagnostics-only. **Still open (G-03).** This audit does not authorize a
   screenshot path; it records that nested captures are not native proof.

---

## Appendix A — evidence commands

```bash
readlink -f /usr/bin/dragonfruit-compositor
readlink -f /usr/bin/dragonfruit-shell
bash scripts/native-session.sh status
journalctl --user -u dragonfruit-compositor.service -n 120 --no-pager
kill -USR1 <compositor-pid>
journalctl --user -u dragonfruit-compositor.service --since "1 min ago" --no-pager
export XDG_RUNTIME_DIR=/run/user/$(id -u) WAYLAND_DISPLAY=dragonfruit-wayland
wayland-info | grep -A10 "interface: 'wl_output'"
```

## Appendix B — code references

> Line numbers are from the audit commit (`6c1acd6`); use the symbol names
> when navigating the current tree.

| Concern | Reference |
|---|---|
| DRM renderer type | `compositor/src/backend/drm.rs:71-76` |
| DRM element enum | `compositor/src/backend/drm.rs:85-97` |
| DRM render | `compositor/src/backend/drm.rs:1194-1427` |
| DRM vblank/presentation | `compositor/src/backend/drm.rs:1029-1190` |
| Nested two-pass | `compositor/src/backend/nested.rs:363-608` |
| Nested scene texture | `compositor/src/backend/nested.rs:613-655` |
| Nested blur elements | `compositor/src/backend/nested.rs:662-776` |
| Feather backdrop | `compositor/src/render.rs:222-297` |
| Blur panel resolution | `compositor/src/render.rs:321-390` |
| Blur/glass shaders | `compositor/src/window/blur.rs:45-201` |
| Blur element | `compositor/src/window/blur.rs:387-554` |
| Degrade tiers/controller | `compositor/src/window/degrade.rs:102-447` |
| Animation clock | `compositor/src/animation.rs:32,226-230` |
| Animation timer | `compositor/src/input.rs:1018-1067` |
| Frame timing | `compositor/src/state.rs` (`RenderStats`, `PresentationStats`, `dump_stats`) |
| Shell offscreen QPA | `shell/src/main.cpp:30-33` |
| Dock commit path | `shell/src/shellcontroller.cpp:6999-7142` |
| Menu bar commit path | `shell/src/shellcontroller.cpp:7144-7169` |
| Frame commit gate | `shell/src/framecommitgate.h` |
| SHM transport | `shell/src/shellprotocol.cpp:91-106,1574-1635` |
| Profile selection | `scripts/native-session.sh:63-92,237-247` |
| Smithay MultiRenderer | `smithay-0.7.0/src/backend/renderer/multigpu/mod.rs:949-1056` |
| Smithay MultiFrame AsMut | `smithay-0.7.0/src/backend/renderer/multigpu/mod.rs:929-947` |
| Smithay DRM render_frame | `smithay-0.7.0/src/backend/drm/compositor/mod.rs:1684-1695` |
