# Proposal — Dock flicker and material artifacts on the native 4K/144 rail

| | |
|---|---|
| **Status** | implemented in part (2026-11-05): F1/F2/F3/F5 landed; F4 region tiling and F6 VRR isolation remain follow-ups. See [ADR 0211](../adr/0211-persistent-chrome-stability-and-blur-cache.md). |
| **Date** | 2026-11-05 |
| **Area** | `compositor/src/window/{material_region,blur,degrade,material_cache}.rs`, `compositor/src/render.rs`, `compositor/src/backend/{drm,nested}.rs`, `shell/src/shellcontroller.cpp`, `shell/dock/Dock.qml` |
| **Related** | T-21.3 (T190a–d), T-21.4b (T192a–d), T193a, ADR 0203–0210, `docs/rendering.md` R1/R4 |
| **Hardware** | native DRM, DP-1 3840×2160 @ 143.999 Hz, scale 1.5 (2560×1440 logical), amdgpu Lucienne |

## 1. Summary

The Dock is not flickering because of the hover QML or a recent titlebar
change. It is flickering because the **global material degrade tier is
oscillating between Full, Reduced, and Minimal**, and the persistent-chrome
floor turns a `Minimal` selection into a different visual state: sampled
glass (`blur=1`) becomes the near-opaque milky fallback
(`tier=reduced blur=0 fallback=4`, “floor-missed”). Every transition repaints
the Dock with a different fill/opacity and, on the compositor side, tears down
or recreates the sampled blur panel. At native 4K/144 this happens often enough
to read as flicker, and the transition can leave one frame of mixed
glass/fallback or stale blur, which reads as artifacting.

The root cause is a cost/feedback loop, not a single bad pixel:

1. The 144 Hz frame budget is **6.944 ms**.
2. The material region resolves to the **full 3840×2160 output**, because the
   single padded union of the top menu bar and bottom Dock covers the whole
   screen. The T-21.3 region optimization therefore does not reduce the common
   case.
3. Every material frame still **rebuilds the Kawase chain for every panel**,
   even when the scene behind the Dock has not changed. The Dock's own hover
   surface is not part of the sampled scene, so a pointer sweep over the Dock
   changes the chrome but not the blur source; the blur work is repeated for
   nothing.
4. The degrade controller sees a tier-dependent load, promotes back into a tier
   that just missed the deadline, and the persistent-chrome floor maps the next
   `Minimal` selection to a blur-off milky plate. That is the visible
   glass ↔ milky cycle.

The recommended fix is a combination of **material stability** (do not swap
persistent chrome between glass and fallback during interaction), a
**predictive/anti-limit-cycle upgrade gate**, and **not recomputing blur when
its source is unchanged**. Region tiling is the durable performance fix; a
panel-damage audit covers the remaining stale-blur risk.

## 2. Evidence from the running native session

The following is from the live session started by `make native-reboot`
(compositor PID 2382, DP-1). It is not a nested or synthetic result.

### 2.1 The material region is the whole output

`SIGUSR1` at 15:04:24:

```text
material stats: path=gpu blur_passes=232752 blur_panels=465688
  blur_draws=232752 blur_downsample_max=16
  material_allocations=95 material_scene_allocations=3
  material_panel_allocations=92
  material_region_px=8294400 material_region=3840x2160

output material stats: output=DP-1 material_frames=232752
  material_allocations=95 material_region_px=8294400
  material_region=3840x2160
```

The full output is 8,294,400 px. The region is full output even though the
Dock is only a strip. `blur_passes == material_frames`, so every material frame
rebuilt the blur chain; the cache is fixing allocations, not GPU work.

### 2.2 The controller is not settling

```text
degrade stats: tier=minimal forced=0 budget_us=6944 samples=283678
  over_budget=10499 downgrades=23 upgrades=21
  tiers=full:69035,reduced:161684,minimal:52959
  budget_source=refresh down_window_us=200000 up_window_us=3000000 latched=1
```

23 downgrades and 21 upgrades in roughly 24 minutes is not a stable selection.
`latched=1` proves the flap latch engaged, but it only gates the boundary
immediately above the current tier; the other boundary can still cycle.

### 2.3 The Dock is visibly switching between glass and fallback

Shell role events from the same session (the shell logs every change-only
role state):

```text
15:02:03  dock tier=1 blur=1 fallback=0   (Reduced, sampled glass)
15:02:04  dock tier=1 blur=0 fallback=4   (floor-missed, milky plate)
15:02:11  dock tier=1 blur=1 fallback=0
15:02:17  dock tier=0 blur=1 fallback=0   (Full)
15:04:49  dock tier=1 blur=1 fallback=0
15:04:53  dock tier=0 blur=1 fallback=0
15:05:47  dock tier=1 blur=1 fallback=0
15:05:50  dock tier=0 blur=1 fallback=0
```

These transitions occur one to seven seconds apart, and each one calls
`applyMaterialState()`, which immediately schedules a Dock repaint. The Dock's
`materialBlurActive` property switches the plate between `dockFill` at
`plate.fillOpacity` and `dockFallbackFill` at `dockFallbackOpacity` with no
transition. That is a deliberate, large visual change on every transition.

### 2.4 The frame budget is borderline

```text
frame timing: p50_us=2103 p95_us=6571 p99_us=8535 max_us=138154
  budget_us=6944 over_budget=10499
presentation stats: p50_us=6945 p95_us=9080 p99_us=12706
  missed=9087 refresh_hz=143.999 vrr=on
```

A p95 close to the 6.944 ms deadline is exactly the condition that makes an
EMA-based controller keep changing its mind. The controller is behaving as
written; the problem is that the cost model and the visible material policy are
not stable at this workload.

### 2.5 Evidence boundary

The logs prove **multi-second material identity changes** on the Dock. They do
not, by themselves, prove whether the user's “flicker” is those swaps, a
per-frame shimmer, or both. A 10 s screen recording or a sequence of compositor
frame captures, timestamped against the `material role` log, is needed to
separate the two. If the recording shows a per-frame shimmer while the role is
stable, F5 (panel damage) and F6 (VRR) move ahead of F1/F2.

Useful live diagnostics:

```bash
journalctl --user -u dragonfruit-shell -f | grep 'material role'
kill -USR1 "$(systemctl --user show -p MainPID --value dragonfruit-compositor.service)"
# then read the newest degrade/material/frame-timing lines:
journalctl --user -u dragonfruit-compositor --since '1 min ago' --no-pager
```

## 3. Root cause in code

### 3.1 One padded union becomes the full output

`MaterialRegion::for_material()` (`compositor/src/window/material_region.rs`)
and `chrome_blur_panels()` (`compositor/src/render.rs`) merge every visible
panel into one padded rectangle. With the menu bar at the top and the Dock at
the bottom, the union is the entire output. The region path is correct but
gives no saving in the most common native configuration.

This is already recorded as a follow-up in the T-21.3 notes: “a single
padded-union rectangle is the full output when the menu bar and Dock are both
blurred; split the top/bottom strips”. It was treated as a performance
follow-up, not a correctness issue. On a 144 Hz 4K panel it becomes a
user-visible stability issue.

### 3.2 The blur chain is rebuilt even when its source did not change

`build_blur_elements()` in `compositor/src/backend/nested.rs` (shared by DRM)
calls `blur_panel_texture()` unconditionally for every panel on every material
frame. The scene texture has a damage tracker and may skip its own render, but
there is no equivalent cache for the panel blur result. A pointer sweep over
the Dock changes the Dock surface, not the sampled scene behind it, so the
blur chain is recomputed with identical input.

The chain base is also still the full output size
(`BlurPrograms::chain_size`), so each panel pays for a full-output-sized
downsample chain even when the panel is a 125 px strip. At four iterations
(`blur_downsample_max=16`) and two panels, this is a large amount of per-frame
work for content that has not changed.

### 3.3 The degrade ladder has a tier-dependent limit cycle

`DegradeController::observe()` (`compositor/src/window/degrade.rs`) steps down
after 200 ms over budget and up after 3 s under `0.75 × budget` (or 10 s under
`0.4 × budget` for a latched boundary). It does not remember *how far over
budget the failed tier was*. If Full costs 8 ms and Reduced costs 5 ms on a
6.944 ms budget, the controller will downgrade Full → Reduced, then later see
Reduced comfortably under budget and promote Reduced → Full, which immediately
misses again. The latch bounds the rate of that cycle but does not remove it,
especially when different boundaries latch independently.

### 3.4 Persistent chrome changes material identity at `Minimal`

`EffectiveMaterial::resolve_for_role()` (`compositor/src/window/degrade.rs`)
is explicit: a persistent role selected at `Minimal` keeps `tier=Reduced` but
sets `blur_active=false` and `fallback=floor-missed`. The shell then draws the
milky complement. This is the intended quality-floor policy from ADR 0203/0204,
but it makes a global tier change a **material identity change** for the Dock
and menu bar. A controller that is allowed to visit `Minimal` will therefore
flash the most visible chrome on the desktop.

### 3.5 Secondary risk: panel damage does not follow the blur source

Panel elements use a stable `Id` but keep `CommitCounter::default()`
(T-21.3). A panel is repainted when its geometry changes or when another
element damages the same output region. If the scene changes only inside the
blur padding just outside the panel (which the blur samples), the panel element
itself reports no damage, so its edge can remain stale until something else
repaints the Dock. This is a plausible contributor to “artifacting” after the
tier flashing is removed, but it is not proven by the current logs. It should
be reproduced with a window moving behind/next to a stationary Dock before
being fixed.

## 4. What is *not* the cause

- The last commit (`f47c6f8`, “fixed traffic lights”) only changes titlebar
  rasterization, bench scripts, and captures. It does not touch the Dock,
  material region, or degrade path. The relevant runtime changes are T190b/c/d
  (region render, stable IDs/damage, scanout counters) and T192a (GPU timing).
- The nested T-21.4b benchmark (`docs/captures/t21-presentation-bench.md`) ran
  at 1920×1200 with host-paced presentation, and its native rows are explicitly
  OPEN for T192c. It could not have caught a native 4K/144 limit cycle.
- The Dock hover/magnification math is not producing the flash: the plate
  geometry is deliberately fixed while the pointer moves along the row, and
  the shell logs show the material role changing at the same time.
- Direct scanout is not involved: `direct_scanout_candidates=0` and
  `direct_scanouts=0` in the live session.
- The material cache is not leaking: `material_scene_allocations=3`,
  `material_allocations=95` after 232k material frames. This is a per-frame
  work problem, not an allocation problem.

## 5. Recommended fixes

> **Implementation status (2026-11-05).** F1, F2, F3, and F5 are implemented
> and covered by unit/conformance tests. F4 (region tiling) and F6 (VRR
> isolation) remain follow-ups; the blur cache removes the repeated hover
> work, so F4 is now a scene-change/multi-panel performance task rather than
> the flicker blocker.

### F1 — Stabilize persistent chrome across tier changes (P0)

Do not let a transient global `Minimal` selection swap the Dock/menu bar from
sampled glass to the milky fallback while the user is interacting.

Recommended shape:

- Keep the last **successful** persistent material while a pointer/keyboard
  interaction is active (or for a short hold-off after the last input).
- If the selected tier reaches `Minimal`, keep persistent chrome on the
  `Reduced` sampled material and let transient roles (`Popup`, `Drawer`) go
  `Minimal`.
- Only enter `floor-missed` after a sustained, idle period (e.g. 10 s) or an
  actual sampled-pass failure.
- Apply a short crossfade (150–250 ms) to the QML plate fill/opacity when the
  material identity does change, so any remaining transition is a fade rather
  than a one-frame flip.

This directly removes the glass ↔ milky flash. It does not by itself reduce
the cost; that is F2/F3.

**Confidence: 85%** that it removes the reported flash; **60%** that it alone
is enough to make the Dock feel stable under a continuous sweep.

### F2 — Add a predictive upgrade gate (P0/P1)

Remember the observed cost of the tier that just failed. Only promote when the
current tier has enough headroom to absorb the measured delta, not merely when
it is below a fixed recovery ratio. For example:

```text
promote only if  ema_current + (ema_failed_tier - ema_current) < budget × 0.85
```

Use the same rule for both ladder boundaries and keep the existing flap latch
as a second line of defence. This is a generic fix for any tier-dependent
limit cycle, not just the Dock.

Add a unit test with a tier-dependent synthetic cost (Full misses, Reduced
fits) that runs a 60 s sweep and asserts at most one downgrade and no
promotion back into the failing tier while the load is unchanged.

**Confidence: 75%** that it stops the Full ↔ Reduced cycling; **65%** that it
does so without slowing genuine recovery after the load drops.

### F3 — Skip blur rebuilds when the source did not change (P1)

Give each panel slot a blur cache key: scene-texture commit, panel source
rectangle, blur spec, chain size, and output scale. Rebuild the chain only
when the key changes. The Dock's own surface commits do not change the sampled
scene, so a pointer sweep should reuse the existing blurred texture.

Implemented as `blur_skips` in `material stats`; `blur_passes` and
`blur_draws` keep their existing meaning, so the acceptance trace can compare
cache hits against the material frames that still built the draw list.

This targets the exact interaction in the report: moving over the Dock should
not pay for a new blur chain when the wallpaper/windows behind it are still.

**Confidence: 80%** that it materially reduces the per-frame material cost;
**55%** that it alone keeps the tier stable at 4K/144.

### F4 — Split the material region and size the chain from the region (P2)

Replace the single padded union with a small set of rectangles (at minimum a
top menu-bar strip and a bottom Dock strip; ideally one region per panel or a
capped tile list). Render the scene texture for each region, not the full
output.

Then size the Kawase chain from the panel's padded region rather than the full
output. The current full-output chain base was chosen to keep the blur
pixel-equivalent to the pre-region path; changing it needs a visual-equivalence
test at 1×, 1.5×, and 2×, plus the existing light/dark and Full/Reduced/Minimal
captures. Keep the full-output diagnostic path behind
`DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1`.

This is the durable fix for 4K/144 and multi-panel glass. It is also the most
likely to change the look if the blur mapping is not preserved exactly.

**Confidence: 70%** on the performance win; **55%** on pixel-equivalent output
on the first attempt.

### F5 — Audit panel damage against the blur source (P1, after F1/F3)

Implemented: a successful blur rebuild advances the panel's own
`CommitCounter`, so a scene change that alters the blurred pixels damages the
panel even when its geometry did not move. The damage unit test covers the
panel-commit path; a live window moving through the blur padding remains the
native confirmation step.

This stays scoped to a real rebuild: a cache hit does not advance the commit,
so the Dock is not repainted for an unchanged blur.

**Confidence: 60%** that this is a real contributor until reproduced;
**85%** that the test will settle it.

### F6 — Isolate VRR as a second variable (P2)

The output reports `vrr=on`, and the presentation interval moves between
6.945 ms and 27.782 ms. VRR brightness flicker is panel-dependent and can look
like a Dock-region flicker on a translucent surface. Run the same Dock sweep
with VRR forced off (or the mode pinned to 120/144 Hz) and compare. Do not
disable VRR globally based on this report alone; only add a policy/opt-out if
the isolated run reproduces the symptom.

**Confidence: 30%** that VRR is a material contributor; this is a
rule-out test, not a recommended primary fix.

## 6. Recommended order and confidence

| Step | Change | Effort | Confidence it addresses the report |
|---|---|---|---|
| 1 | F1 stable persistent material + crossfade | 1–2 days | 85% removes the flash; 60% alone |
| 2 | F2 predictive upgrade gate | 1 day | 75% stops the limit cycle |
| 3 | F3 blur rebuild cache | 1 day | 80% reduces the cost that triggers the cycle |
| 4 | F4 split region + region-sized chain | 2–4 days | 70% performance, 55% visual equivalence |
| 5 | F5 panel-damage audit | 0.5–1 day | 60% if artifacts persist |
| 6 | F6 VRR isolation | 0.5 day | rule-out only |

**Overall confidence: 80%** that F1+F2+F3 removes the reported flicker and
most of the visible artifacting without a hardware-specific workaround.
**Residual risk: ~15–20%** that an independent panel-damage or VRR issue
remains and needs F5/F6.

F1 alone is the right emergency mitigation if a session cycle is expensive;
F1+F2+F3 is the recommended fix for v0.1.1. F4 should follow as the durable
4K performance work rather than being rushed into the same change.

## 7. Acceptance and validation

Run on the same native DP-1 4K/144 seat, not nested:

1. 60 s continuous Dock sweep with `DRAGONFRUIT_CHROME_STATS=1`,
   `DRAGONFRUIT_FRAME_TRACE=1`, and a `SIGUSR1` snapshot before/after.
2. Assert from the shell log:
   - zero persistent-role transitions during the sweep (or at most one, after
     the sweep settles);
   - no `dock ... blur=0 fallback=4` while the pointer is active.
3. Assert from the compositor:
   - `blur_passes` / `blur_rebuilds` no longer advances on every material
     frame;
   - `material_region` is the Dock strip, not 3840×2160, after F4;
   - `material_allocations` is flat across the sweep;
   - `degrade stats` shows no new downgrade/upgrade pair during the sweep.
4. Assert frame timing: p99 below the 6.944 ms budget after warm-up, or a
   documented hardware-class exception. Do not lower the refresh rate to make
   the number pass.
5. Repeat light/dark, Full/Reduced/Minimal, 1×/1.5×/2×, a window moving behind
   the Dock, a Dock popover, and a fullscreen client.
6. Keep `DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1` as the A/B diagnostic and
   `DRAGONFRUIT_DRM_MATERIAL=0` as the rollback.

## 8. Immediate diagnostic workaround

To confirm the diagnosis without waiting for the fix, a session built with
`DRAGONFRUIT_DRM_MATERIAL=0` should show a stable milky Dock with no
glass ↔ milky flicker. `DRAGONFRUIT_DRM_GLASS=0` keeps the blur but removes the
glass compose and may be enough to keep the tier higher. Neither is a fix;
they are isolation runs.

## 9. Open questions

- Should persistent chrome ever reach `Minimal` at all, or should the floor
  become “Reduced with blur on” and the cost problem be solved in the region
  path? ADR 0203/0204 chose the explicit milky fallback; this report shows the
  runtime cost of that choice on 4K/144.
- Is the correct anti-flap unit a **role** (persistent vs transient) or a
  **material identity** (blur on vs milky)? The proposal assumes role-scoped
  stability plus a predictive gate.
- Do we want a short crossfade even after the controller is stable? A fade
  makes an unavoidable transition (e.g. GPU failure) less jarring, but it also
  hides real failures from visual acceptance runs.
