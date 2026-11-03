# Proposal — Native DRM Dock material: why it does not read as the premium macOS Dock

| | |
|---|---|
| **Status** | proposal / draft |
| **Area** | `compositor/src/backend/drm.rs`, `compositor/src/backend/nested.rs`, `compositor/src/window/{blur,degrade,backdrop}.rs`, `shell/dock/Dock.qml`, `scripts/native-session.sh` |
| **Related** | T-20.1–T-20.4, ADR 0182, ADR 0184, ADR 0186, `docs/dock.md` |
| **Hardware** | native DRM session (the current dogfooding rail) |

## 1. Summary

The native session **is** running the QML Dock and the latest Dock geometry
(`Dock configured 3840x125`, matching `barThickness + magnifyBand + edgeMargin`).
The gap is not the QML layout. It is the **material path**:

1. The native session uses the DRM backend. The DRM backend still composites the
   old **feather backdrop** and never runs the GPU scene-texture blur/liquid-glass
   pass. `material stats` reports `path=gpu` but `blur_passes=0 blur_panels=0`.
2. The currently running session predates the new release default and is still
   the **debug profile** (`/usr/bin/dragonfruit-compositor` →
   `target/debug/...`). At 4K the debug build is far over the 16 ms budget, so
   the adaptive degrade controller drops to `reduced` and eventually `minimal`;
   at `minimal` even the feather backdrop is removed. The Dock is then only the
   QML translucent plate.
3. The QML plate is tuned for a **blur-backed** material. Without compositor
   blur/refraction it reads as a flat translucent slab, especially over a light
   or mid-tone wallpaper. There is no milky-glass fallback tuned for the DRM
   rail.

In short: **the premium look lives in the nested GPU two-pass path, not in the
native DRM path.** The native session is rendering the fallback material.

## 2. Evidence from the running session

```text
$ readlink -f /usr/bin/dragonfruit-compositor
/home/user/repos/binbuf/dragonfruit/target/debug/dragonfruit-compositor
$ readlink -f /usr/bin/dragonfruit-shell
/home/user/repos/binbuf/dragonfruit/build/shell/src/dragonfruit-shell
```

The shell is current (its mtime is after the Dock QML edits), and it logs:

```text
dragonfruit-shell: Dock configured 3840x125
dragonfruit-shell: shell: Dock scene-graph commit path active (FR-14)
```

The compositor reports:

```text
material stats (SIGUSR1): backdrop_passes=11 backdrop_skipped=0 path=gpu
                          blur_passes=0 blur_panels=0 blur_downsample_max=0
degrade stats (SIGUSR1): tier=reduced forced=0 budget_us=16000 samples=13
                         over_budget=12 downgrades=1 upgrades=0
frame timing (SIGUSR1):  samples=13 avg_us=493955 max_us=4004453
```

The previous session's exit dump is even more telling:

```text
degrade stats (exit): tier=minimal ...
                      tiers=full:959,reduced:199558,minimal:203981
```

More than half of that session ran at `Minimal`, where the backdrop is off
entirely. The kernel/input log also reports the debug build is too slow:

```text
libinput error: client bug: event processing lagging behind by 22ms,
                your system is too slow
```

`path=gpu` here means "a GPU renderer is available"; it does **not** mean the
sampled-material pass ran. `blur_passes=0` is the smoking gun.

## 3. Root causes

### 3.1 The DRM backend never calls the GPU blur/glass pass

The nested backend has the complete two-pass composition
(`compositor/src/backend/nested.rs:417-470`):

1. render the scene (wallpaper, desktop, windows, shadows, menus) to an
   offscreen texture;
2. resolve `chrome_blur_panels` (`compositor/src/render.rs:321`);
3. build per-panel Kawase-blurred textures and liquid-glass elements
   (`compositor/src/backend/nested.rs:662-780`);
4. compose chrome + blur + the scene texture.

The DRM backend does not. It still appends
`crate::render::chrome_backdrop_render_elements(...)`
(`compositor/src/backend/drm.rs:1314-1328`), which is the deterministic feather
stack from before T-20.1. The code comment above the multi-GPU marker says so
explicitly: "The DRM rail still composes the deterministic feather stack
(T-20.1 follow-up)" (`compositor/src/backend/drm.rs:530`).

So on DRM the Dock gets a flat tint/feather under the QML plate — no frosted
wallpaper, no edge lens, no specular rim, no adaptive tint.

### 3.2 The adaptive degrade tier removes the backdrop under load

`DegradeTier::Minimal` disables blur entirely
(`compositor/src/window/degrade.rs:146`). The controller steps down after 12
over-budget frames and only recovers after 180 clearly-under-budget frames
(`DOWNGRADE_FRAMES`, `RECOVERY_RATIO`). The debug build at 4K is consistently
over budget, so it sits at `Reduced`/`Minimal`.

At `Reduced`, the feather backdrop still exists but is half-scale and
half-layered; at `Minimal` there is no compositor backdrop at all. The QML
plate remains, but a translucent QML rectangle over a raw wallpaper is not a
premium glass material.

### 3.3 The QML plate has no no-blur fallback tuned for DRM

`Dock.qml` draws a translucent `dockFill` at `plate.fillOpacity` 0.42, a bright
interior rim, a gloss band, a hairline border, and a shadow
(`shell/dock/Dock.qml:3654-3745`). That stack is designed to sit over the
compositor's blurred/glass panel. Without the blur it is a flat translucent
slab; over a light wallpaper it washes toward white instead of the reference's
milky neutral gray.

`docs/dock.md` §3 explicitly recommends starting from a faithful solid/milky
gray panel and adding compositor blur only after the geometry. The current QML
has the opposite emphasis: it relies on the compositor for the body of the
material.

### 3.4 The profile policy (updated)

**Fixed:** the native commands now default to **release**. `make native`,
`make native-rebuild`, and `make native-reboot` build and install
`target/release`/`build-release`; debug is opt-in with `make native-debug`,
`DF_NATIVE_PROFILE=debug`, or `--debug`. The currently running session predates
this change and is still the debug install, so it must be switched once with a
rebuild/reboot. `native-restart` refuses a profile mismatch because it cannot
replace the running compositor.

This matters because the 4K material/damage path is CPU-bound in debug
(`scripts/native-session.sh:44-46`, `Makefile:679-681`), and the adaptive
degrade ladder reacts to that by dropping the material.

## 4. Proposal

### P0 — Immediate, no code change (unblock the current session)

The native commands now default to release; switch the current debug session
once and confirm the material counters:

```bash
make native-reboot
# after login:
make native-status
make native-capture
grep 'material stats' ~/.local/state/dragonfruit/native-captures/<latest>/snapshot-*.txt
```

Expected: `selected profile: release` / `installed profile: release`,
`tier=full` or `reduced`, much lower `frame timing avg_us`, no libinput lag.
**`blur_passes` will still be 0** until P2; this only removes the degrade
pressure and restores the feather backdrop.

`make native-debug` remains available for fast iteration, but it is not the
profile to use when judging the visual result.

### P1 — QML milky-glass fallback (fastest visible improvement)

Make the Dock look intentional on the DRM rail even before the blur pass lands.

- Add a `material.blurAvailable` (or `material.tier`) signal from the shell to
  QML, sourced from the compositor's effective material path/tier. The shell
  already knows the compositor's material state for the reserved zone; extend
  that seam.
- When blur is unavailable, draw a **milky** plate: a neutral gray fill
  (light: around the reference's `#E5E5E5` family, not white), `fillOpacity`
  around 0.72–0.85, the existing bright interior rim, a slightly darker
  anchored-edge lip, and the soft shadow. Keep the body flat; no loud gradient.
- When blur is available, keep the current translucent fill so the compositor's
  frost/refraction shows through.
- Tokenize both variants under `component.dock.plate` (for example
  `fallbackFill`, `fallbackOpacity`) so the design-system gate still owns the
  values.
- Add a `tst_dock` case that forces the fallback and asserts the plate is
  opaque enough to read as a material (not a raw wallpaper tint), plus a native
  capture on the DRM session.

This is the smallest change that makes the native Dock stop looking like a flat
QML rectangle.

### P2 — DRM two-pass scene texture + blur/glass (the real fix)

Port the nested material path to the DRM rail.

1. **Generalize the offscreen scene render.** Move
   `render_scene_offscreen` (`compositor/src/backend/nested.rs:613`) and
   `blur_panel_texture` (`:701`) behind a renderer-generic helper bounded by
   `Renderer + Offscreen<GlesTexture> + Bind<GlesTexture>`. Both `GlowRenderer`
   (nested) and `UdevRenderer` (DRM) satisfy these bounds.
2. **Implement `BlurRenderer` for `UdevRenderer`.** `UdevRenderer` is
   `MultiRenderer<GbmGlesBackend<GlesRenderer>, ...>`
   (`compositor/src/backend/drm.rs:71`). `MultiFrame` exposes
   `AsMut<GlesFrame>`; implement `draw_blur_span` with the same
   `render_texture_from_to` call used for `GlowRenderer`
   (`compositor/src/window/blur.rs:403`).
3. **Add element variants.** Extend the local `DrmOutputElements` macro
   (`compositor/src/backend/drm.rs:86`) with a `Blur` variant
   (`BackdropBlurElement`) and a full-output `Scene` variant
   (`TextureRenderElement<GlesTexture>`), mirroring the nested
   `NestedOutputElements` enum.
4. **Branch in `drm.rs`.** After `chrome_blur_panels`:
   - if non-empty: render the scene list offscreen, then compose
     chrome + blur elements + the scene texture;
   - if empty: keep the existing feather path.
   The lock surface and cursor stay above the stack, exactly as in the nested
   path.
5. **Respect multi-GPU routing.** Use `multi_gpu::plan_scene_routes`; never
   sample a scene texture across GPUs. On `route=software`, fall back to P1's
   milky material rather than a broken glass pass.
6. **Tests and observability.** Add a pure element-list/ordering test for the
   DRM branch (the renderer itself needs hardware), assert
   `blur_passes > 0` and `blur_panels >= 1` in a native capture, and keep the
   nested path unchanged. The `material stats` line is the acceptance signal.

### P3 — Quality floor for persistent chrome

The Dock and menu bar are always visible; they should not fall off the material
ladder as aggressively as transient overlays.

- Add a `chrome_quality_floor` policy: on the GPU path, persistent chrome stays
  at `Reduced` at worst (blur stays on), while transient overlays may still go
  to `Minimal`.
- Prefer cheaper blur at 4K: cap iterations/downsample for the Dock and menu bar
  before dropping blur, and cache per-panel textures (the T-20.1 per-frame
  allocation follow-up).
- Consider a `DRAGONFRUIT_FRAME_BUDGET_US` override for the native session to
  avoid the debug build's 4K death spiral during development.

### P4 — Profile and runbook guardrails

- Print the active and installed profile in `native-status` (done).
- Release is now the default for every native command; debug is explicit.
  Keep the warning when a debug profile is selected for a visual run.
- Document the material acceptance check in `docs/runbook-drm-session.md`:
  `path=gpu`, `blur_passes>0`, `tier=full|reduced`, no libinput lag.

## 5. Acceptance criteria

- [ ] Native DRM `material stats` reports `path=gpu`, `blur_passes > 0`,
      `blur_panels >= 1` on a normal desktop.
- [ ] The Dock reads as a continuous frosted-glass plate over both a light and a
      dark wallpaper: visible background diffusion, a soft specular rim, a
      subtle edge lens, and no flat white/gray slab.
- [ ] At 4K release, `degrade` stays at `Full`/`Reduced` (never `Minimal` for
      persistent chrome) during a normal pointer sweep and app launch.
- [ ] The nested backend's material output is unchanged.
- [ ] `tst_dock`, `tst_dockcore`, `tst_design_system`, the gallery goldens, and
      the full ctest suite stay green.
- [ ] A native capture and a nested capture are committed side by side.

## 6. Alternatives considered

- **Only raise the QML plate opacity.** Cheap and improves the flat look, but it
  cannot reproduce the frosted wallpaper, edge lens, or specular rim. Keep it as
  P1, not the final answer.
- **Use a screenshot of the wallpaper in QML.** Violates the no-screencopy rule,
  adds a full-output texture to the shell, and cannot follow moving windows.
- **Force `Full` tier on DRM.** Does nothing while the DRM rail never calls the
  blur pass, and would make the debug build slower.
- **Disable adaptive degrade entirely.** Removes the safety valve for real
  budget pressure; a quality floor for persistent chrome is the safer policy.

## 7. Risks

- `DrmCompositor::render_frame` may not accept a full-output scene texture
  element without a custom element variant; the P2 step 3 is the main unknown.
- Cross-GPU scene texture copies need the existing `multi_gpu` routing; a naive
  local sample would fail on hybrid laptops.
- Blur at 4K is expensive; without texture caching and a downsample cap, P2
  could make the degrade problem worse rather than better.
- The shell must learn the effective material tier; adding a new protocol event
  is a versioned change. Keep it additive and optional so older shells degrade
  to P1.

## 8. Immediate recommendation

1. Re-run on the default release profile: `make native-reboot`.
2. Confirm `native-status` shows `selected profile: release` /
   `installed profile: release` and `native-capture` shows lower frame time and
   `tier=full|reduced`.
3. If the Dock is still flat, implement **P1** now (milky QML fallback), then
   schedule **P2** (DRM two-pass glass) as the durable fix.
