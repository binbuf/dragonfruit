# 0182 — The Tahoe liquid-glass material is a GPU sampled backdrop pass

## Status

accepted

## Context

ADR [0122](0122-tahoe-interface-language-across-chrome.md) names macOS **Tahoe**
as the reference for every piece of chrome, and ADRs
[0013](0013-backdrop-blur-pass.md)/[0091](0091-dock-tahoe-floating-glass-language.md)
shipped a token-driven **approximation** of its glass: a stack of translucent
rounded feather layers drawn by the compositor under each chrome surface, with
the real texture-sampling blur (Kawase vs dual-pass Gaussian) explicitly
deferred and "no refraction is claimed". That approximation cannot sample the
scene it stands over — text and video under the menu bar do not blur — and it
cannot express Tahoe's Liquid Glass (refraction, a specular rim, an adaptive
tint).

The deferred work is now unblocked. The renderer is Smithay 0.7 `GlowRenderer`
(GLES via glow); the nested backend already renders the scene into an offscreen
`GlesTexture` (`compositor/src/backend/nested.rs::capture_frame`, via
`Offscreen`/`Bind`/`export`), and Smithay 0.7 exposes
`GlesRenderer::compile_custom_texture_shader` /
`GlesFrame::render_texture_from_to` and
`smithay::backend::renderer::element::texture::TextureRenderElement`. The
project now **assumes a GPU** for the material (a GLES 3.x / Vulkan-capable
device); the software/headless path is a CI fallback, not the target.

## Decision

- **Two-pass composition.** Once per frame the renderer renders the scene
  (wallpaper, desktop, windows, shadows) into an output-sized offscreen scene
  texture, then composes the output from the **scene texture** + one **blurred
  backdrop element per chrome panel** + the **chrome Wayland surfaces**. The
  blurred panel samples the scene texture region under the panel through a
  custom texture shader, so the effect is over **live** buffers (FR-1), never a
  screenshot or a re-render. The existing `BackdropPass` bookkeeping stays: one
  effect pass per output per frame, no double-blur.
- **Blur is downsample + Kawase.** The scene region under a panel is downsampled
  and blurred with a small Kawase (or separable Gaussian) program compiled once
  per renderer; the blur radius comes from the role's material token. The
  implementation is swappable behind the token values, as ADR 0013 required.
- **The panel material is role-driven.** `MaterialRole` continues to select the
  token group (`Chrome` for the menu bar, `Dock` for the Dock, `Popup` for
  menus/context menus/popovers/OSD/Control Center/notifications), and a new
  `Drawer` role covers the full-output Applications drawer, which declares its
  card via `df_layer_surface.set_panel_rect` (the Dock's mechanism, ADR 0089).
  Blur radius, opacity, and the rounded-corner radius come from the generated
  tokens, never literals.
- **Liquid-glass pass.** On top of the blur the panel adds the Tahoe signature:
  an SDF-based edge lens (refraction) that displaces the sampled backdrop near
  the rim, a bright specular inner rim, and an adaptive tint derived from the
  blurred backdrop's luminance. All parameters are semantic material tokens per
  scheme.
- **Degrade honestly.** `DegradeTier` is unchanged: `Full` = blur + refraction +
  specular, `Reduced` = smaller radius/scale and no refraction, `Minimal` = the
  current deterministic translucent/flat feather fallback (blur off). The tier
  selector keeps the same frame-budget input and one-pass guard.
- **GPU baseline.** A recommended GPU (GLES 3.x / Vulkan-capable) is documented
  as the material's baseline; the headless/software path renders `Minimal`, so
  CI stays deterministic and low-end machines stay legible.
- **Multi-GPU.** The scene texture is owned per GPU; `renderer_multi` composes
  the blur on the GPU that owns the output, copying the scene texture when an
  output is rendered by a different GPU.

## Consequences

- This **supersedes ADR 0013's "real blur deferred"** and **amends ADR
  0091/0122's "no refraction is claimed"**: the feather-layer builder becomes
  the `Minimal` fallback and the refraction pass is scheduled.
- New semantic material tokens are added per role (refraction strength, specular
  intensity, adaptive-tint amount); `Theme.qml` and
  `compositor/src/design_tokens.rs` regenerate from `tokens.json` (`make
  check-tokens`).
- The frame-budget risk moves from an iGPU cliff to a GPU budget; the degrade
  ladder still guards software and low-end devices, and idle stays zero-wakeup
  because the pass runs only on frames a backend renders with chrome mapped.
- Headless/CI goldens need tolerance (or a CPU reference blur), because GL blur
  output is driver-dependent; the deterministic `Minimal` approximation is the
  headless path.
- The compositor samples live buffers inside the render pass, so the
  no-screencopy gate and the "effects are compositor passes" rule in
  [02-compositor.md](../02-compositor.md) hold.
- Rollout is per-surface work, not new machinery: once the pass exists the menu
  bar, Dock, context menus, popovers, the OSD, notifications, Control Center,
  and the Applications drawer inherit it through their `MaterialRole`, and the
  shell's QML "glass claim" layers (fill/rim) can drop to a token-driven
  complement rather than standing in for the material.

## Implementation (T-20.1)

The first slice landed as the nested proof of the two-pass composition; the
DRM/headless paths still render the deterministic feather stack.

- `compositor/src/window/blur.rs` (new) owns the renderer-facing half: the
  `BLUR_SHADER` 4-tap Kawase custom texture program, the pure token-resolved
  `BlurSpec` (radius → bounded iteration count), the `BlurRenderer` seam, and
  the `BackdropBlurElement` that draws a downsampled scene region back over a
  panel clipped to the shared `CornerMask` spans.
- `compositor/src/backend/nested.rs` renders the scene into an output-sized
  offscreen `GlesTexture` once per frame, downsamples each panel's region
  through the Kawase chain (falling back to the built-in bilinear downsample if
  the custom program failed to compile), and composes
  `chrome surfaces + blur elements + scene texture`. The material pass is taken
  by `render::chrome_blur_panels`, so the one-pass/no-double-blur guard holds.
  `Minimal`, no mapped chrome, or a driver without the shader keeps the
  pre-T-20 feather elements — headless stays a no-readback path.
- `MaterialRole::Drawer` selects the popup blur tokens and rounds from
  `primitive.radius.xl` (the card's own QML radius); the Applications drawer
  declares its card through `ShellProtocol::setAppsDrawerPanelRect`, mirroring
  the Dock's `set_panel_rect`.
- No new material token or literal blur radius: `material.{chrome,popup,dock}Blur`
  is the only radius source. The iteration count and downsample factor are
  derived mapping constants.

Refraction, the specular rim, and the adaptive tint (T-20.2), the Dock/menus/OSD
rollout (T-20.3), and multi-GPU/headless determinism (T-20.4) were unchanged by
T-20.1.

## Implementation (T-20.2)

The liquid-glass pass landed on top of the T-20.1 blur without re-architecting
it: the compose of a panel's blurred texture now runs a second custom program.

- Twelve semantic tokens were added to `semantic.{light,dark}.material`:
  `chrome/popup/dock` × `Refraction`, `Specular`, `SpecularWidth`, `Tint`
  (`design-system/tokens/tokens.json`; regenerate `Theme.qml`/`design_tokens.rs`
  with `make check-tokens`). No blur downsample token: the Kawase iteration
  count stays a derived mapping constant as in T-20.1, and the shader takes no
  literal value.
- `compositor/src/window/blur.rs` owns `GLASS_SHADER`, a GLES2 custom texture
  program with the `//_DEFINES_` marker and the built-in `tex`/`alpha`/
  `v_coords` contract. It reconstructs panel-local pixels from `v_coords`
  (the panel-normalized coordinate), evaluates the rounded-rect SDF at the
  token corner radius, pulls the backdrop sample toward the centre near the rim
  (the edge lens), adds a specular band, and blends toward the scheme tone by
  the adaptive-tint amount scaled by how far the blurred backdrop's mean
  luminance is from the tone. `GlassSpec` resolves the tokens to shader uniforms
  (scaled to physical pixels); `BackdropBlurElement::with_glass` carries the
  spec + compiled program.
- `MaterialRole::glass_spec(scheme)` resolves the per-role/per-scheme tokens
  (the Drawer shares Popup, like its blur/tone); `render::chrome_blur_panels`
  attaches the spec to each `BlurPanel`, and `DegradeTier::glass` maps it:
  `Full` unchanged, `Reduced` drops refraction to zero (keeping specular +
  tint), `Minimal` returns `None`.
- `compositor/src/backend/nested.rs` compiles `GLASS_SHADER` once per renderer
  (non-fatal on failure: the panel keeps the plain blurred compose) and passes
  it to `build_blur_elements`; `query material` reports the resolved glass
  parameters per role after the tier.
- The Applications card's QML fill was retuned to the Dock's glass level
  (`controls.appsDrawer.panelOpacity` 0.90 → 0.5) so the compositor material
  reads through the complement; the menu bar already used
  `material.chromeOpacity` (T-20.1). The remaining surfaces' QML complements are
  T-20.3.
- Headless unit tests assert the per-role/scheme token mapping, the glass
  uniform packing, and the tier behaviour; the pixel effect is verified by the
  committed nested captures (`docs/captures/t20-liquid-glass-*.png`).

The Dock/menus/OSD rollout (T-20.3) and multi-GPU/headless determinism (T-20.4)
are unchanged.