# Dock learnings — Cutefish glass study and a transparency setting

| | |
|---|---|
| **Status** | Proposal (not implemented) |
| **Date** | 2026-11-04 |
| **Area** | `shell/dock`, `shell/menubar`, `design-system`, `compositor/src/window`, `apps/settings`, `services/settingsd`, `protocols` |
| **Related** | [`dock.md`](dock.md), [`rendering.md`](rendering.md), [`settings-keys.md`](settings-keys.md), ADR [0013](design/adr/0013-backdrop-blur-pass.md), [0015](design/adr/0015-material-degrade-tiers.md), [0091](design/adr/0091-dock-tahoe-floating-glass-language.md), [0182](design/adr/0182-tahoe-liquid-glass-material-pass.md), Track [20](design/tracks/20-tahoe-liquid-glass.md) |
| **Source studied** | `~/repos/github/cutefish` — `cutefish-compositor` @ `de4014b`, `cutefish-toolkit` @ `fbd5e09`, `settings` @ `a3cdcfa` (working tree, 2026-11-04) |

## 0. Summary

Cutefish's Dock has **no bitmap glass assets to import**. Its dock chrome is
procedural: a compositor-drawn glass lens, a small amount of QML/paint for
dividers and icons, and a shared blur/glass toolkit. The useful material is
therefore the *shader*, the *parameter curves*, and the *layering discipline*.

The most important finding is a tuning one:

> Cutefish's glass is nearly clear. Its default dock glass uses a blur sigma
> of **1.0 px**, a lens displacement of **36 px**, a bevel of **14 px**, a
> depth of **16 px**, an IOR of **1.7**, and a tint alpha of **0.29** (dark) /
> **0.25** (light). Its own source says a wide blur or a strong tint "turns it
> into frosted plastic and hides the refraction that makes it read as glass at
> all."

Our Dock currently combines a **30–34 px Kawase blur**, a **0.42 QML fill**,
a **0.20–0.22 adaptive tint**, and **three or four separate edge treatments**.
That is a frosted slab, not a lens. The first visual win is not a new effect;
it is removing the double tint and double edge and rebalancing blur against
refraction.

Recommendations, in order:

1. **Fix the material stack first (P0).** One owner for the body tint, one
   owner for the lit rim, and a clear-glass preset. This needs no new setting.
2. **Add one global transparency setting (P1).** `appearance.transparency`
   (0 = more tinted, 1 = more clear), applied by the compositor and mirrored to
   QML. This is the requested System Settings slider.
3. **Upgrade the lens (P2).** Port the superellipse lens model from Cutefish's
   `glass_fragment_shader.glsl`: bevel, depth, IOR, profile, displacement,
   chroma, saturation, ambient occlusion, and rim. Keep our adaptive tint.
4. **Extend glass to client surfaces (P4).** Settings and first-party dialogs
   are regular toplevels today, so the compositor does not frost them. Add a
   private surface-material protocol with a shape mask for arrows and rounded
   bodies.
5. **Roll out through one QML material layer (P5).** Menu bar, popovers, OSD,
   notifications, Control Center, Settings, and dialogs should read the same
   effective values rather than re-deriving them per component.

---

## 1. What was examined

| Path | What it contains |
|---|---|
| `cutefish-compositor/src/render/shaders/glass_fragment_shader.glsl` | The full superellipse lens: height field, normals, refraction, chromatic split, saturation, ambient occlusion, specular/sheen, lit rim. |
| `cutefish-compositor/src/render/glass.h` | `GlassParams` defaults and the design rationale for each optical field. |
| `cutefish-compositor/src/shell/dock/dock_window.cpp` | Dock panel rect, radius, glass parameter tuning, the `GlassTint` pace curve, and the clear/default/tinted blend. |
| `cutefish-compositor/src/shell/dock/dock_settings.{h,cpp}` | `Dock/GlassTint` persistence, clamping, and `setGlassTint`. |
| `cutefish-compositor/src/shell/dock/dock.cpp` | `org.cutefish.Dock.setGlassTint` D-Bus method. |
| `cutefish-compositor/src/shell/dock/dock_tooltip.cpp` | Tooltip glass follows the bubble path; the arrow is part of the frosted shape. |
| `cutefish-compositor/src/shell/osd/osd.cpp`, `.../clipboard/clipboard_picker.cpp`, `.../statusbar/status_bar_window.cpp` | The same glass material reused for OSD, clipboard, and status-bar plates. |
| `cutefish-compositor/protocols/cutefish-surface-blur-v1.xml` | Per-surface blur/glass protocol, including material parameters, tint, saturation, rim, and arbitrary shape masks. |
| `cutefish-toolkit/src/ui/multi_effect_blur.cpp` | Six-level mip blur: four taps per level, LOD weights, level count by `blurMax`. |
| `cutefish-toolkit/src/ui/fast_blur.cpp`, `header_backdrop.cpp` | GPU/CPU variants and the frosted header that fades in as content scrolls under it. |
| `cutefish-toolkit/src/ui/popup_glass.cpp` | Popup defaults: higher blur/tint for text readability, no rim. |
| `settings/src/pages/dock/dockpage.cpp`, `settings/src/pages/dock/docksettings.cpp` | The "Glass effect" slider: "More clear" ↔ "More tinted", writing through D-Bus. |

### 1.1 There are no dock plate assets

A search for dock/glass assets across the Cutefish tree finds only:

- `settings/src/images/dock_left.svg`, `dock_bottom.svg`, `dock_right.svg` —
  small **position preview thumbnails** for the settings page, not the Dock
  chrome;
- the usual status/launcher SVGs under `cutefish-compositor/data/shell`;
- no dock background bitmap, nine-patch, or glass texture.

The Dock's plate is drawn by the compositor (`setBackgroundBlur(panelRect,
radius, glass)`), and the panel view paints only the dividers. The visual
quality comes from the glass shader and the parameter choices, not artwork.

### 1.2 License note

The Cutefish source files carry BSD-3-Clause headers
(`SPDX-License-Identifier: BSD-3-Clause` in the toolkit, and the BSD text in
the compositor's `LICENSE`). BSD-3-Clause is compatible with this repository's
MIT license, but a port must preserve the copyright notice and add the
attribution to `NOTICE` / `LICENSES/`. This proposal assumes that is done for
any shader code we adapt.

---

## 2. How Cutefish builds the glass

### 2.1 One material, many roles

`render::GlassParams` is the single material description. The Dock, popups,
OSD, clipboard picker, status bar, and tooltips all start from
`fishuiGlass()` and override a few fields:

| Role | Overrides |
|---|---|
| Dock | blur 1.0, displacement 36, saturation 1.3, rim 1.0, tint per scheme |
| Popups/menus | blur 6, displacement 20, chroma 0.03, saturation 1.2, tint alpha 0.6 |
| OSD | popup glass with darker/lighter tint (0.10/0.90) |
| Tooltip | popup glass, shape follows the bubble path |
| Status bar | default glass, shape clipped to the plate |

This is close to our `MaterialRole` idea, but Cutefish carries the optical
parameters per surface rather than resolving them from role tokens in the
compositor.

### 2.2 The lens model

The shader is not a simple "blur plus tint". It models a rounded superellipse
slab:

1. A signed distance field gives the rounded-rectangle shape.
2. A height field (`height(sd, band)`) rolls the surface off over the bevel
   band, with `profile` controlling dome vs flat top.
3. `normalAt` derives the surface normal; `lensOffset` refracts the view ray
   through the surface using `ior` and `displacement`.
4. The backdrop is sampled four times with a widened footprint near the lip,
   then saturation is adjusted.
5. The tint is mixed over the refracted backdrop, under the highlights.
6. Ambient occlusion darkens the inside of the edge.
7. Specular/sheen are screen-blended, so they brighten toward white rather
   than clipping.
8. A lit rim can be drawn by the material itself; when it is on, the
   compositor's window outline is suppressed so the two do not stack.

`render/glass.h` explains the intent better than any summary:

> "Glass is nearly clear: a strong tint or a wide blur turns it into frosted
> plastic and hides the refraction that makes it read as glass at all."

The default values are:

| Field | Default | Effect |
|---|---:|---|
| `blurSigma` | 1.2 | Softening before refraction; much smaller than a plain blur. |
| `bevel` | 14 | Width of the visible lip / roll-off band. |
| `depth` | 16 | Height of the glass above its back face; with bevel, sets edge steepness. |
| `ior` | 1.7 | Index of refraction. |
| `profile` | 2.5 | Superellipse exponent: low = dome, high = flat top with a sharp lip. |
| `displacement` | 36 | Length of the refracted ray inside the glass. |
| `chroma` | 0.05 | Red/blue split as a fraction of the offset; 0 disables the rainbow fringe. |
| `lightAngle` | 50° | Light direction for specular/sheen/rim. |
| `specularIntensity` | 0 | Mirror highlight; off for panels (reads as plastic). |
| `shininess` | 42 | Specular exponent. |
| `sheen` | 0 | Directional body lift; off for panels. |
| `ambientOcclusion` | 0 | Edge darkening; off by default. |
| `saturation` | 1.0 | Backdrop saturation before tint. |
| `rim` | 0 | Material-drawn lit edge; Dock sets 1.0. |
| tint RGBA | white @ 0.12 | Straight-alpha tint under the highlights. |

### 2.3 The Dock's three-point glass tuning

The Dock maps one slider value (`Dock/GlassTint`, 0 = More clear, 1 = More
tinted, default 0.5) through a non-linear pace table, then blends three
explicit presets with a monotone cubic:

```text
kPace = {0.000, 0.048, 0.116, 0.174, 0.249, 0.314, 0.361,
         0.398, 0.435, 0.469, 0.500, 0.587, 0.630, 0.666,
         0.701, 0.739, 0.781, 0.827, 0.879, 0.931, 1.000}

glassBlend(amount, clear, default, tinted)  // monotone cubic through 0/0.5/1
```

The Dock's presets (`dock_window.cpp`):

| Parameter | Clear | Default | Tinted |
|---|---:|---:|---:|
| `blurSigma` | 0.6 | 1.0 | 8.0 |
| `displacement` | 36 | 36 | 0 |
| `saturation` | 1.4 | 1.3 | 1.0 |
| tint alpha, dark | 0.12 | 0.29 | 0.50 |
| tint alpha, light | 0.16 | 0.25 | 0.53 |
| premultiplied grey, dark | 0.046 | 0.110 | 0.190 |
| premultiplied grey, light | 0.060 | 0.150 | 0.530 |
| straight tint, dark | 0.383 | 0.379 | 0.380 |
| straight tint, light | 0.375 | 0.600 | 1.000 |
| `rim` | 1.0 | 1.0 | 1.0 |
| `chroma` | 0 | 0 | 0 |

Two details are worth stealing:

- **The tint is decomposed into a straight colour and an alpha.** The code
  computes a grey value and divides it by the alpha before sending it, so the
  tint reads as a neutral grey at a controlled opacity rather than a
  premultiplied colour that darkens as it becomes more transparent.
- **The "tinted" end removes refraction** (`displacement` 36 → 0) and raises
  blur. The slider is therefore not just opacity: it moves the material from
  a clear lens to a frosted panel. That is exactly the axis a transparency
  slider should expose.

The pace table makes the middle of the slider feel neutral and gives more
resolution near the ends. A linear slider would spend most of its travel in a
visually similar middle range.

### 2.4 One slider, two processes

Cutefish splits the setting across processes in a way we should not copy:

- the Dock persists `Dock/GlassTint` in `shell.conf` and exposes
  `org.cutefish.Dock.setGlassTint`;
- the Settings app reads `shell.conf` directly and sends the D-Bus method;
- the Dock also writes the file itself.

Our `settingsd` model is cleaner: one owner, one persisted key, one `Changed`
signal, and the shell/compositor consume it. The proposal below keeps that
model.

### 2.5 The blur chain

Cutefish uses a six-level mip blur rather than Kawase:

- each level is half the previous level's size;
- each level takes four bilinear taps, one logical pixel along one axis and
  `0.36` along the other, with a dither of `0.33`;
- the final composite weights the source and the levels by how near each
  level is to the requested level of detail (`calculateLod`);
- `multiEffectBlurLevels(blurMax)` selects 1–5 levels (4 levels for
  `blurMax = 32`).

This gives a smoother large-radius blur than repeated Kawase, at a similar
cost: every level is a 4-tap pass on a smaller texture, and the composite is
one weighted sample of each level. Our Kawase chain uses 4 diagonal taps per
iteration, with the centre weighted `4` and the neighbours `1`, and saturates
at 4 iterations (`BLUR_PER_ITERATION = 8`, `MAX_BLUR_ITERATIONS = 4`). Both
are cheap; the mip blur's advantage is that the radius is decoupled from the
iteration count and the level weights avoid the boxy look of repeated
downsamples.

### 2.6 Layering discipline

The most important non-shader lesson is what Cutefish does **not** paint:

- The Dock panel view paints only dividers; the plate, tint, rim, and shadow
  are the compositor's glass.
- Popups paint no translucent fill over the glass when the compositor material
  is available. The protocol documentation says clients should use
  `set_glass_tint` so the tint sits under the highlights instead of over them.
- The tooltip's glass is masked by the bubble path, so the arrow is frosted
  and outlined as part of the same shape.
- The status bar passes a rounded path to `setBackgroundBlur`, so fractional
  layout positions do not produce a second rasterized edge.

In other words: **the compositor owns the optical stack; the client owns
content and, at most, a fallback surface.**

### 2.7 Other reusable details

- **Radius from thickness.** The Dock's corner radius is
  `thickness * 28 / 86`, so a resized Dock keeps the same corner proportion.
- **Header fade.** `HeaderBackdrop` pads the blur by 32 px, tints at 0.72, and
  fades the material in over 16 px of content scrolling under it. This is a
  good pattern for Settings sidebars, file-manager toolbars, and any future
  scroll-under chrome.
- **Popup readability.** Popups use a much stronger blur and tint than the
  Dock (blur 6, tint alpha 0.6) because text must read over any backdrop. A
  single global slider must not make popups as clear as the Dock.
- **Shape masks.** `set_shape` takes a path of `move_to`/`line_to`/
  `cubic_to`/`close` commands. The blur, outline, and shadow follow it. This
  is how the tooltip arrow stays frosted.
- **Icon treatment.** Cutefish masks icons to a rounded square at roughly
  `width * 2 / 54`, and its dock metrics scale every padding from the icon
  size (`dock_metrics.h`). Our Dock already has tokenized squircles, insets,
  and a metric scale, so this is a sanity check rather than a port.
- **Motion.** Row transitions are 260 ms on OutQuint; a new icon reveals from
  75 ms over 200 ms on OutQuint. Our Dock's motion is already tokenized and
  spring-based; do not replace it, but use the timings as a comparison point.

### 2.8 What not to copy

- The CPU/Skia toolkit and its CPU blur path. Our compositor already has a GPU
  path; porting the shader and the parameter model is enough.
- The split settings ownership (`shell.conf` plus D-Bus plus a second reader).
- A fixed tint with no adaptive contrast. Our `GLASS_SHADER` already adapts
  the tint to the backdrop luminance, which is better for text over busy
  wallpapers. Keep it.
- Unbounded per-client glass with no area or rate policy. Cutefish exposes it
  to every client; we should gate it (see P4).

### 2.9 Port / adapt / skip

| Decision | Items |
|---|---|
| **Port** | The superellipse lens math (height field, normal, `refract`, widened 4-tap sampling), the `pace` + monotone-cubic three-point blend, the lit-rim-owns-the-outline rule, the shape-mask idea for arrows/tooltips, and the header-fade constants. |
| **Adapt** | Cutefish's optical defaults into our token schema; its clear/default/tinted presets into one global `appearance.transparency` key; its per-surface glass into our `MaterialRole` resolver; its D-Bus/config split into `settingsd` + one protocol policy request. |
| **Skip** | The CPU Skia toolkit and CPU blur path, the split settings ownership, a fixed tint with no adaptive contrast, and unbounded client glass without a policy. |

---

## 3. Our baseline

| Area | Current implementation | Gap |
|---|---|---|
| Glass shader | `GLASS_SHADER` in `compositor/src/window/blur.rs`: rounded SDF, simple smoothstep lens falloff, 3×3 adaptive tint, specular band. | No bevel/depth/IOR/profile, no chromatic split, no saturation, no ambient occlusion, no material rim. The lens is a displacement along the SDF normal, not a refracted ray through a surface. |
| Glass parameters | `GlassSpec`: refraction, specular, specular width, tint, tint colour. | Only five knobs; all optical shape is implicit. |
| Blur | Kawase, 4 taps, 1–4 iterations; `material.dockBlur` 30 (light) / 34 (dark). | Heavy frost; the refraction has little to bend because the backdrop is already flat. |
| Tint | Compositor adaptive tint 0.22 (light) / 0.20 (dark) **plus** QML `dockFill` at `plate.fillOpacity` 0.42. | Double tint; the body is much more opaque than the tokens suggest. |
| Edges | Compositor specular band **plus** QML `dockRim`, `dockGloss`, `dockBorder`, `dockEdge`. | Three or four edges stack; the result is a bright, busy lip rather than one lit edge. |
| Roles | `MaterialRole::{Chrome, Dock, Popup, Drawer}` resolved from layer + namespace. | Good foundation; no per-surface client material. |
| Runtime control | Compile-time tokens plus degrade tiers. | No user-facing intensity; changing the look requires a rebuild. |
| Client surfaces | Only `df_layer_surface` chrome (layer ≥ 2) is blurred. | Settings and dialogs are regular toplevels and get no frost. |
| Shapes | `set_panel_rect` gives a rounded rect. | No arbitrary path; popover arrows are not frosted. |
| Fallback | `materialBlurActive` + milky tokens in Dock/MenuBar. | Popovers/OSD/Control Center still assume blur (R4.4 in `rendering.md`). |

Relevant code:

- `shell/dock/Dock.qml:3654–3810` — the plate stack (shadow, fill, gloss, rim,
  edge, border).
- `shell/menubar/MenuBar.qml:163–175` — the menu bar's chrome fill and
  fallback expression.
- `design-system/Theme.qml:157–179` (light) and `237–259` (dark) — material
  tokens; `368–381` — dock plate tokens.
- `compositor/src/window/blur.rs:112–205` — `GLASS_SHADER`; `333–380` —
  `GlassSpec`; `224` — `LENS_BAND_FACTOR`.
- `compositor/src/window/backdrop.rs:247–306` — `glass_spec`; `322` —
  `chrome_blur_panels`.
- `compositor/src/render.rs:303–380` — `BlurPanel` and panel resolution.
- `compositor/src/window/degrade.rs:168–190` — effective material; `280–330` —
  tier scaling.
- `protocols/dragonfruit-shell.xml:115` — `set_panel_rect`.
- `protocols/dragonfruit-toplevel.xml:209` — `set_motion_policy` (the pattern
  for a new policy request).
- `shell/src/themebinding.cpp:52–89` — settings → `Theme` mapping.
- `shell/src/compositorpolicy.cpp:22–58` — settings → compositor policy view.
- `services/settingsd/src/schema.rs:22`, `216+` — schema version and keys.
- `apps/settings/AppearancePane.qml`, `apps/settings/DesktopDockPane.qml` —
  where a slider would live.

---

## 4. Diagnosis: why the Dock reads frosted rather than glass

The current material is not wrong; it is over-layered. Four effects compound:

1. **Blur dominates refraction.** Our dock blur is 30–34 px while the lens
   displacement is 8–10 px. Cutefish's default is the opposite: 1 px blur,
   36 px displacement. A 30 px blur destroys the high-frequency detail that
   refraction is supposed to bend, so the lens has nothing to show.
2. **The body is tinted twice.** The compositor applies a 0.20–0.22 adaptive
   tint; the QML plate adds a 0.42 fill on top. The combined body is roughly
   55% opaque before the rim and gloss. Cutefish's client paints no body tint
   at all; the compositor tint is the body.
3. **The edge is lit three or four times.** The compositor's specular band,
   the QML interior rim, the gloss band, the hairline border, and the anchored
   edge highlight all draw near the same boundary. Each is subtle in
   isolation; together they read as a bright plastic lip.
4. **The lens has no thickness.** Our shader's "lens" is a smoothstep falloff
   along the SDF normal. It bends the sample a little near the edge, but it
   does not model a surface with a bevel, depth, or IOR. The characteristic
   thick-glass edge — the compressed, slightly magnified band just inside the
   rim — is missing.

At a glance:

| Metric | Dragonfruit Dock (today) | Cutefish Dock (default) |
|---|---:|---:|
| Blur | 30–34 px Kawase | 1.0 px sigma |
| Refraction / displacement | 8–10 px | 36 px |
| Bevel / depth | none | 14 / 16 px |
| IOR / profile | none | 1.7 / 2.5 |
| Tint | 0.20–0.22 + QML fill 0.42 | 0.25–0.29, no client fill |
| Saturation | 1.0 | 1.3 |
| Edge | specular + rim + gloss + border | one material rim |
| User control | none | 0–1 clear/tinted slider |

The fix is therefore a rebalance, not a new feature:

| Change | Effect |
|---|---|
| Lower blur, raise displacement | The backdrop stays legible and the lens has detail to bend. |
| Remove QML body fill when blur is active | One tint owner; the glass becomes transparent instead of milky. |
| One rim owner | A single lit edge instead of a stack. |
| Add bevel/depth/profile/IOR | A real lip with a compressed band. |
| Raise saturation slightly | Clear glass keeps the backdrop vivid instead of washing it grey. |

Keep the adaptive tint. It is the one place where our implementation is
ahead of Cutefish: text contrast over a bright wallpaper is handled by the
tint adaptation, not by a fixed high alpha.

---

## 5. Proposals

### P0 — Fix the material stack (no new setting)

**Goal:** make the current Dock read as glass before adding controls.

**Work:**

1. **One body tint owner.** When `materialBlurActive` is true, the QML plate
   fill goes to zero (or a very small text scrim, ≤ 0.06). The compositor
   `GlassSpec.tint` is the body. When blur is inactive, keep the existing
   milky `dockFallbackFill` at `dockFallbackOpacity`.
2. **One rim owner.** The compositor draws the optical rim (specular band and,
   later, the material rim). The QML keeps only the outer hairline border and
   the shadow, which are not optical. Disable the QML `dockGloss`, `dockRim`,
   and `dockEdge` when `materialBlurActive`; keep them in the fallback. If a
   design review prefers the QML rim, disable the compositor specular instead
   — but do not ship both.
3. **Clear-glass preset.** Retune the dock tokens: lower `dockBlur`, raise
   `dockRefraction`, lower `dockTint`, add saturation. This is a token-only
   change, behind `DRAGONFRUIT_GLASS_PRESET=clear|current` until captures are
   approved.
4. **Capture baseline.** Light/dark × pale/colourful/dark wallpaper, at rest
   and magnified. Compare the current and clear presets side by side.

**Acceptance:**

- No visible double edge at the plate boundary.
- The plate is transparent enough that the wallpaper's colour and structure
  are visible through it, not just a grey wash.
- Text and icons retain contrast on a white wallpaper and on a dark one.
- The fallback plate (blur off) still reads as a deliberate milky surface.

**Risk:** the clear preset may look too transparent on a busy wallpaper. The
adaptive tint and the P0 capture matrix are the mitigation; the preset flag is
the rollback.

### P1 — Add the global transparency setting

This is the main deliverable requested. It is specified in detail in
section 6.

**Short version:** add `appearance.transparency` (double, 0–1, default 0.5) to
`settingsd`; mirror it into `Theme` for QML and forward it to the compositor
over a new additive `df_toplevel_manager.set_material_policy` request; add a
slider to the Appearance pane; make the compositor blend three material
presets (tinted / balanced / clear) and make QML scale its fill and fallback
values from the same scalar.

**Acceptance:**

- Dragging the slider updates the Dock, menu bar, and open popovers live,
  without resizing a surface or dismissing a popover.
- 0 = more tinted (frosted, readable), 0.5 = the approved default, 1 = more
  clear (lens visible, wallpaper legible).
- The value persists across restart and survives a settingsd restart.
- The compositor and QML agree on the effective fill; no double tint.
- With no compositor support (older build), the slider still changes the QML
  fill and does not break.

### P2 — Upgrade the lens shader

**Goal:** adopt the superellipse lens model while keeping our adaptive tint.

**Work:**

1. Add uniforms/tokens for `bevel`, `depth`, `ior`, `profile`,
   `displacement`, `chroma`, `saturation`, `ambient_occlusion`,
   `ambient_occlusion_radius`, `light_angle`, and `rim`.
2. Replace the smoothstep lens with the height-field/normal/refract path from
   `glass_fragment_shader.glsl`. Keep the 4-tap widened sampling near the lip.
3. Keep the existing 3×3 adaptive-tint *amount* and our scheme-aware tint
   colour, but adopt Cutefish's straight-alpha tint decomposition (a straight
   colour plus alpha) and apply it once, under the highlights. Do not stack a
   fixed tint and an adaptive tint in series. Suggested order: refract →
   saturation → adaptive tint → ambient occlusion → specular/sheen → rim.
4. Add the material rim as an option. If `rim > 0`, suppress the window
   outline around the body so the two do not stack (Cutefish does this).
5. Add the new tokens under `primitive.glass` (shared optical constants) and
   per-role `displacement`/`saturation`/`rim` values. Generate with
   `scripts/gen-tokens.py`; do not hand-edit `Theme.qml` or
   `compositor/src/design_tokens.rs`.
6. Degrade behavior: `Reduced` drops refraction (already does), `Minimal`
   drops blur/glass; the reduce-transparency override drops both. Measure the
   4K iGPU budget.

**Acceptance:**

- A visible compressed/magnified band just inside the plate edge, following
  the corner arcs, with no hard ring or halo.
- The lip bends the wallpaper rather than only brightening it.
- No banding on a smooth gradient wallpaper.
- 60 Hz on the reference GPU at `Full`; `Reduced` stays within budget; no
  regression in `material stats`.

**Risk:** per-pixel normals and refraction are more expensive than the current
shader. The degrade ladder already has a `Reduced` tier that drops refraction;
use it. If the cost is too high at 4K, keep the current shader for `Full` and
use the new one only when the panel is small, or reduce the normal sampling
to a lower-resolution pass.

### P3 — Evaluate the mip-blur chain

**Goal:** decide whether Cutefish's six-level mip blur is a better quality/cost
trade than our Kawase chain.

**Work:**

- Prototype the level weights and the 4-tap pass in a branch, behind an env
  flag.
- Compare banding, halo, and edge stability on a checkerboard, a smooth
  gradient, and a high-contrast photo.
- Measure GPU time at 1080p and 4K, panel-sized and full-output.
- Only switch if the quality gain is visible at equal or lower cost. The mip
  blur's weighted composite is the part worth keeping even if we keep Kawase.

**Acceptance:** a documented comparison in `docs/captures/` and a decision
(keep Kawase, switch, or hybrid). No visual regression at the panel edge.

### P4 — Client-requested glass for Settings and dialogs

**Problem:** the compositor only blurs chrome surfaces (`df_layer_surface`,
layer ≥ 2). The Settings app and its dialogs are regular toplevels, so they
cannot get the frosted backdrop today. This is why "apply the same visuals to
Settings and dialogs" is not just a QML change.

**Proposal:** add a private surface-material protocol (for example
`protocols/dragonfruit-surface-material.xml`), modeled on
`cutefish-surface-blur-v1.xml` but with our trust model:

- `set(surface, x, y, w, h, radius)` — rounded region, surface-local logical
  pixels.
- `set_shape(surface, path[])` — optional outline for arrows and non-rect
  bodies; the blur, outline, and shadow follow it.
- `set_glass(surface, params)` — optional material overrides (or a role name
  such as `popup`/`dialog` so the compositor resolves tokens).
- `unset(surface)` — remove on commit.
- All requests are pending surface state and apply atomically on commit.

**Policy:** do not expose this to every client without limits. Options:

1. **Allowlist by `app_id`** for first-party system apps (Settings, Files,
   dialogs) initially. Simplest and safest.
2. **Capability token** (`TrustedRole::SystemUi`) minted by the session for
   first-party apps. More work but fits the existing trust model.
3. **Any client, with a per-client area/rate budget.** Cutefish's model; most
   flexible, but a GPU DoS surface and a larger security review.

Recommendation: start with (1), design the protocol so (2) is a drop-in
policy change, and keep the per-surface area capped. The compositor should
never read back the scene; the client only gets the rendered result.

**Also:** use the shape mask for popover arrows and tooltips. Today the arrow
is a rotated QML rectangle with a fill but no frost; the shape mask makes the
whole bubble one material.

**Acceptance:**

- The Settings window and a modal dialog show the same frosted material as
  the menu bar, with the correct rounded corners.
- A tooltip's arrow is frosted and outlined as one shape.
- A client without the capability cannot request glass; a malformed path is
  rejected without affecting other surfaces.
- No measurable frame-budget regression with 3–4 glass surfaces open.

### P5 — Roll out through one QML material layer

**Goal:** no component re-derives the material.

**Work:**

- Add a hand-written `GlassTheme` singleton (registered as
  `Dragonfruit.GlassTheme`; `Theme.qml` stays generated) with
  `transparency`, `reduceTransparency`, and the effective values:
  `dockFillOpacity`, `chromeFillOpacity`, `popupFillOpacity`,
  `dockFallbackOpacity`, `chromeFallbackOpacity`, `popupFallbackOpacity`,
  `rimOpacity`, `glossOpacity`, `borderOpacity`.
- Replace direct uses of `Theme.material.*Opacity` and
  `Theme.controls.dock.plate.fillOpacity` in:
  - `shell/dock/Dock.qml`
  - `shell/menubar/MenuBar.qml`
  - `design-system/components/Popover.qml`, `Popup.qml`, `Dialog.qml`,
    `ContextMenu.qml`, `Tooltip.qml`
  - `shell/control-center/*`, `shell/notifications/*`, `shell/osd/*`
  - `apps/settings/SettingsShell.qml` and the panes' cards
- Make the fallback coverage explicit: every glass surface gets a milky
  fallback when `materialBlurActive` is false, not just the Dock/menu bar
  (this closes R4.4 in `rendering.md`).

**Acceptance:**

- Changing the slider changes every chrome surface consistently.
- No surface is left as a flat translucent slab when blur is off.
- The design-system gallery still passes; any new material tokens have
  gallery coverage or a documented exception.

### P6 — Polish details

Lower priority, but cheap and visible:

- **Continuous corners.** Use the same corner construction in the compositor
  and QML (squircle/continuous vs circular) so the frost and the fill do not
  disagree at the corner.
- **Shadow tint.** A black shadow under a light plate can read as a grey halo;
  consider a scheme-tinted shadow token so the shadow follows the same
  light/dark intent as the glass tint.
- **Hover lift.** Our existing magnification is already good; keep it rather
  than adding a second hover transform.
- **Header fade.** Adopt `HeaderBackdrop`'s 16 px fade for scroll-under chrome
  in Settings/Files.
- **Divider opacity.** Cutefish draws dock dividers at 0.18; ours may be
  stronger. Compare in the capture matrix.

---

## 6. Detailed design: `appearance.transparency`

### 6.1 Semantics

- **Key:** `appearance.transparency`
- **Type:** double (`d`)
- **Range:** 0.0–1.0
- **Default:** 0.5
- **UI label:** "Transparency effects"
- **Captions:** `More tinted` (0) … `Balanced` (0.5) … `More clear` (1)
- **Meaning:** higher values make the chrome material more transparent and
  more lens-like; lower values make it more tinted and frosted. It does not
  change window opacity, text opacity, or icon opacity.
- **0 is not "off".** It is the most tinted *glass* end: high blur, high tint,
  low refraction. The off state is the accessibility toggle below, which is a
  separate boolean so the slider can stay a material-character control rather
  than a power switch.

Why this direction? It matches the user's request ("transparency effects
intensity") and makes higher values mean more transparent, which is the usual
reading of a transparency control. The *axis* is Cutefish's "More clear ↔
More tinted" axis, but the value direction is inverted: Cutefish stores 0 =
clear and 1 = tinted, while this proposal stores 0 = tinted and 1 = clear so
the key name and the UI agree. The middle of the slider is the safe default.
A separate accessibility toggle (below) provides a true off state.

**Companion accessibility key (recommended):**
`accessibility.reduceTransparency` (bool, default false). When true, the
compositor skips the sampled pass and every surface uses the milky fallback;
the slider remains persisted but has no effect. This is the macOS-style
"Reduce transparency" behavior and is cheap to implement because the fallback
already exists.

### 6.2 Schema

Add to `services/settingsd/src/schema.rs` (bump `SCHEMA_VERSION` from 17 to
18):

```rust
KeySpec {
    key: "appearance.transparency",
    group: KeyGroup::Appearance,
    kind: KeyType::Number,
    default: KeyDefault::Number(0.5),
    allowed: &[],
    min: Some(0.0),
    max: Some(1.0),
    owner: "settingsd",
    consumer: "shell/design-system Theme, compositor/window material",
    since: 18,
    summary: "Chrome material transparency: 0 more tinted, 1 more clear.",
},
```

Optional companion. The schema has no `Accessibility` key group (the existing
`accessibility.reduceMotion` lives in `Animation`), so keep the key name and
use the `Appearance` group rather than adding a group for one key:

```rust
KeySpec {
    key: "accessibility.reduceTransparency",
    group: KeyGroup::Appearance,
    kind: KeyType::Bool,
    default: KeyDefault::Bool(false),
    allowed: &[],
    min: None,
    max: None,
    owner: "settingsd",
    consumer: "shell/design-system Theme, compositor/window material",
    since: 18,
    summary: "Force opaque chrome and disable the sampled glass material.",
},
```

`owner` is `settingsd` for consistency with the other `appearance.*` keys; the
Appearance pane writes through `Settings.set`, which goes through settingsd.
Update `docs/settings-keys.md`, the schema doc test, and the frozen v1 key-set
test (additive only). The migration fills the default for older files.

### 6.3 Effective parameter mapping

Do not scale every token linearly. Use three presets and a monotone blend,
like Cutefish, but express the presets as **multipliers on today's tokens**
so the change is easy to review and revert.

| Parameter | At 0 (tinted) | At 0.5 (balanced) | At 1 (clear) | Notes |
|---|---:|---:|---:|---|
| blur | ×1.35 | ×1.00 | ×0.30 | Lower blur lets refraction show. |
| refraction | ×0.25 | ×1.00 | ×2.50 | The main "lens" axis. |
| specular | ×0.60 | ×1.00 | ×1.10 | Keep the rim subtle. |
| specular width | ×0.80 | ×1.00 | ×0.80 | Narrower at the clear end. |
| tint alpha | ×2.50 | ×1.00 | ×0.35 | The main "frost" axis. |
| client fill | ×1.40 | ×1.00 | ×0.00 | QML body fill; 0 when blur active at clear. |
| saturation | ×0.85 | ×1.00 | ×1.30 | New token; clear glass keeps colour. |
| displacement | ×0.30 | ×1.00 | ×1.50 | New token; per role. |
| bevel | ×1.00 | ×1.00 | ×1.20 | Shared token; slightly wider at the clear end. |

Role floors prevent unreadable chrome:

- **Dock:** client fill may reach 0 at the clear end (icons are opaque; the
  plate is decorative).
- **Menu bar:** keep a minimum chrome fill of 0.12 at the clear end; text is
  thin and sits directly on the material.
- **Popups/dialogs:** keep a minimum fill of 0.20 and a tint alpha of at
  least 0.12; Cutefish's popup glass uses tint 0.6 for exactly this reason.
- **Fallback:** `dockFallbackOpacity` / `chromeFallbackOpacity` /
  `popupFallbackOpacity` stay near their current values; the fallback is for
  "no blur", not "user wants clear".

These multipliers are a starting point for capture review, not final art
direction. The important part is the *shape* of the mapping: blur down,
refraction up, tint down as transparency rises.

### 6.4 The curve

Reuse Cutefish's pace table and monotone cubic:

```rust
// compositor/src/window/material.rs (new)
pub const PACE: [f64; 21] = [
    0.000, 0.048, 0.116, 0.174, 0.249, 0.314, 0.361,
    0.398, 0.435, 0.469, 0.500, 0.587, 0.630, 0.666,
    0.701, 0.739, 0.781, 0.827, 0.879, 0.931, 1.000,
];

pub fn pace(value: f64) -> f64 { /* piecewise linear interpolation */ }

/// Monotone cubic through (0, tinted), (0.5, balanced), (1, clear).
pub fn blend3(amount: f64, tinted: f64, balanced: f64, clear: f64) -> f64 { /* ... */ }
```

Why not linear? The optical parameters do not have equal perceptual weight per
unit. The pace table spends more travel near the ends, where the eye notices
the change from "frosted" to "clear", and keeps the middle stable. It also
makes 0.5 exactly the default preset.

QML does not need the full curve. It only needs the client fill and fallback
opacities, which can use a simple `smoothstep` or the same three-point blend
implemented once in `GlassTheme.qml`. Keep the optical curve in Rust; keep the
QML curve limited to fill/rim values, and add a test that the two endpoints
agree.

### 6.5 Data flow

```text
AppearancePane Slider
  └─ Settings.set("appearance.transparency", v)
       └─ settingsd (owner; clamp 0..1; persist)
            └─ SettingsClient::changed
                 ├─ ThemeBinding::apply
                 │    ├─ GlassTheme.transparency = v
                 │    ├─ GlassTheme.reduceTransparency = reduce
                 │    └─ GlassTheme.* (effective fill/rim/fallback)
                 │         └─ Dock / MenuBar / Popover / OSD / Settings QML
                 └─ ShellController::applyCompositorPolicy
                      └─ ShellProtocol::setMaterialPolicy(v, reduce)
                           └─ df_toplevel_manager.set_material_policy (v8)
                                └─ DfState.material_transparency / reduce
                                     └─ MaterialRole::spec/blur_spec/glass_spec
                                          └─ BlurPanel -> Kawase + GLASS_SHADER
```

One write, two consumers, no second settings source.

### 6.6 Protocol

Add an additive request to `df_toplevel_manager` (version 8):

```xml
<!-- Additive in v8. The shell forwards the one settingsd-owned transparency
     value; the compositor owns the material curve. -->
<request name="set_material_policy" since="8">
  <description summary="set the chrome material transparency">
    `transparency` is 0.0 for the most tinted/frosted material and 1.0 for
    the clearest lens. `reduce_transparency` is the accessibility override:
    non-zero means solid chrome with no sampled pass, regardless of
    `transparency`. The compositor clamps `transparency` to 0..1 and blends
    its token-defined material presets. Unknown or older writers leave the
    current values untouched. This is a global policy, not per output.
  </description>
  <arg name="transparency" type="fixed" summary="0.0 more tinted .. 1.0 more clear"/>
  <arg name="reduce_transparency" type="uint" summary="1 forces solid chrome"/>
</request>
```

Bump `MANAGER_INTERFACE_VERSION` to 8 in `compositor/src/shell/mod.rs`, add
the generated dispatch arm, and update `docs/private-protocols.md` and
`docs/ipc-versioning.md`. The shell must check `m_managerVersion >= 8` before
calling, like `setMotionPolicy` checks `>= 5`.

Keep the two values separate. Do not collapse the accessibility override into
`transparency = 0`: the slider's 0 end is still glass, and a sentinel would
make the material curve and the off state impossible to reason about.

### 6.7 Compositor

1. **State.** Add `material_transparency: f64 = 0.5` and
   `material_reduce_transparency: bool = false` to `DfState`.
   `set_material_policy` clamps the number, ignores non-finite values, and
   sets `needs_redraw` only on change.
2. **Preset blend.** Add `compositor/src/window/material.rs` with the pace
   table, `blend3`, and a `MaterialIntensity` struct. Keep it pure and unit
   tested; it must not depend on Wayland or renderer types.
3. **Role resolution.** Thread the intensity into `MaterialRole::spec_for`,
   `blur_spec_for`, and `glass_spec`. Add `*_with(intensity)` variants and
   keep the existing methods as `*_with(DEFAULT)` so current tests and callers
   do not churn. `chrome_blur_panels` and `chrome_backdrop_render_elements`
   pass the state value.
4. **Solid mode.** When `material_reduce_transparency` is true, return no
   `BlurPanel` and no glass; the shell uses the fallback. `EffectiveMaterial`
   must report `blur_active = false` and a new
   `MaterialFallback::UserDisabled` wire value `4`. Update the protocol
   comment in `dragonfruit-toplevel.xml` (the fallback enum is additive).
5. **Degrade order.** Apply the intensity blend first (design), then the
   degrade tier (quality floor). At `Reduced`, drop refraction; at `Minimal`,
   drop blur/glass. The accessibility override wins over every tier and over
   the slider.
6. **Stats.** Add `transparency=<v>` to the material stats line and keep
   `blur_passes`/`blur_panels` honest. A solid mode must show `blur_passes=0`
   and `blur_active=0`, so performance is measurable.
7. **No allocation churn.** The intensity only changes uniforms; it must not
   reallocate textures or change panel geometry. Do not fold it into the
   `BlurSpec` cache key in a way that invalidates the scene texture every
   slider move; update uniforms and damage the panel region.

### 6.8 Shell and QML

1. **A hand-written material singleton, not a generated token.** `Theme.qml`
   is generated from `tokens.json` and must not be hand-edited. Add
   `design-system/GlassTheme.qml`, register it in the design-system QML module
   as `Dragonfruit.GlassTheme`, and give it runtime state and effective
   values:

   ```qml
   pragma Singleton
   import QtQuick

   QtObject {
       id: root

       // Set by ThemeBinding; never persisted here.
       property real transparency: 0.5
       property bool reduceTransparency: false

       readonly property real dockFillOpacity: ...
       readonly property real chromeFillOpacity: ...
       readonly property real popupFillOpacity: ...
       readonly property real dockFallbackOpacity: ...
       // rim/gloss/border if they scale
   }
   ```

   `GlassTheme` should be a pure function of the generated `Theme.material`
   tokens, `transparency`, and `reduceTransparency`, so there is one place to
   audit. The examples below write `GlassTheme.*`; if the generator is
   extended later to emit a `Theme.glass` block, the components do not change.

2. **ThemeBinding.** Watch `appearance.transparency` (and
   `accessibility.reduceTransparency`) in `onSettingsChanged`; set the
   `GlassTheme` properties in `apply()` alongside `dark`, `reducedMotion`, and
   `accent`. The Settings app is a separate process, so mirror the same two
   keys onto its own `GlassTheme` in `apps/settings/SettingsShell.qml`,
   exactly as it already mirrors `appearance.colorScheme`,
   `appearance.accent`, and `accessibility.reduceMotion`.

3. **Dock.** Replace:

   ```qml
   opacity: dock.materialBlurActive
            ? Theme.controls.dock.plate.fillOpacity
            : Theme.material.dockFallbackOpacity
   ```

   with:

   ```qml
   opacity: dock.materialBlurActive && !GlassTheme.reduceTransparency
            ? GlassTheme.dockFillOpacity
            : GlassTheme.dockFallbackOpacity
   ```

   and gate `dockGloss` / `dockRim` / `dockEdge` on `materialBlurActive` if P0
   chooses the compositor as the rim owner.

4. **Menu bar and popups.** Use `GlassTheme.chromeFillOpacity` and
   `GlassTheme.popupFillOpacity` in the fill expressions. Keep the text and
   icons fully opaque; only the fill alpha changes.

5. **No geometry work.** Do not call `applyDockSettings(true)` or
   `configureDockSurface` for a transparency change. Add a separate
   `applyMaterialSettings()` path that only updates `GlassTheme` and the
   compositor policy. This avoids a surface reconfigure and a popover
   dismissal on every slider move. Keep the policy early-out: a
   transparency-only change must not resend the unchanged motion/input
   policies or reapply the color scheme.

### 6.9 System Settings UI

Place it in `apps/settings/AppearancePane.qml` (global visual effect), not in
Desktop & Dock. If the design review prefers Dock-specific, the same control
can live in `DesktopDockPane.qml`; the key is global either way.

```qml
// At the pane root:
readonly property real transparencyValue: {
    var v = Settings.values["appearance.transparency"];
    return v === undefined ? 0.5 : Number(v);
}

SettingsGroup {
    id: transparencyGroup
    width: parent.width
    title: qsTr("Transparency")

    SettingsRow {
        width: parent.width
        label: qsTr("Transparency effects")
        controlData: Slider {
            id: transparencySlider
            width: 220
            from: 0.0
            to: 1.0
            minLabel: qsTr("More tinted")
            midLabel: qsTr("Balanced")
            maxLabel: qsTr("More clear")
            accessibleName: qsTr("Transparency effects")
            onMoved: (value) => Settings.set("appearance.transparency", value)
        }
    }

    // Optional accessibility companion.
    SettingsRow {
        width: parent.width
        label: qsTr("Reduce transparency")
        controlData: Toggle {
            id: reduceTransparencyToggle
            text: ""
            onToggled: (checked) => Settings.set("accessibility.reduceTransparency", checked)
        }
    }
}

// The T-09.1b pattern: a Binding, not a `value:` expression, because
// Slider.setValue assigns the property and would break the binding on drag.
Binding {
    target: transparencySlider
    property: "value"
    value: root.transparencyValue
}
Binding {
    target: reduceTransparencyToggle
    property: "checked"
    value: Settings.values["accessibility.reduceTransparency"] === true
}
```

Add `property alias transparencySlider: transparencySlider` (and the toggle)
to the pane's test surface. Do not write the value back inside `onMoved`; the
`Binding` to `Settings.values` is the feedback path, unlike Cutefish's
`glass->setValue(...)` round-trip.

**Preview caveat until P4.** The Settings app is a regular toplevel, so the
compositor does not frost it yet. The slider's live preview is the Dock, menu
bar, and any open popovers; the Settings pane's own cards can still scale
their QML fill from the same key, but do not promise a frosted Settings window
until the client surface-material protocol lands.

### 6.10 Live-write policy

- Write on `moved` for live preview, matching the other panes (T-09.1b).
- `settingsd` clamps and persists; if writes are too chatty on a drag,
  debounce persistence in `settingsd` (the value is still broadcast live) or
  use `committed` for the write and a local preview property. The existing
  `dock.size` slider already writes on move, so start with the same pattern.
- The compositor only changes uniforms; it must not reallocate the scene
  texture, rebuild panels, or reconfigure surfaces.
- The shell must not rebuild Dock entries, close popovers, or change the
  reserved zone.

### 6.11 Failure and fallback matrix

| State | Expected result |
|---|---|
| GPU, `Full`, transparency 0.5 | Lens + adaptive tint + specular rim; QML fill scaled to the balanced value. |
| GPU, `Full`, transparency 1.0 | Clear lens; minimal fill; Dock may have no QML body fill; menu bar/popups keep their floors. |
| GPU, `Full`, transparency 0.0 | Most tinted glass: high blur, high tint, low refraction; still a sampled pass, still readable. |
| GPU, `Reduced` | No refraction; specular/tint remain; blur scaled by the tier; slider still changes tint/fill. |
| GPU, `Minimal` | No blur/glass; milky fallback; slider only changes the fallback tint (or is a no-op). |
| DRM single-GPU | Same as GPU path; the material pass is default-on. |
| DRM multi-GPU / rotated | Feather fallback; milky plate; slider still adjusts the fallback tint. |
| Software/headless | `Minimal`; milky plate; no readback. |
| `accessibility.reduceTransparency` | Forces solid mode: no blur pass, `blur_active=0`, fallback reason `user-disabled`, milky plate, regardless of the slider. |
| No settingsd | Defaults (0.5) from the client; writes stay in memory; no crash. |
| Older compositor (< v8) | Shell skips `set_material_policy`; QML fill still changes; the compositor material stays at its tokens. Document this as a degraded but safe state. |

### 6.12 Tests and acceptance

**Schema / settings:**

- `services/settingsd` schema test: key exists, type/range/default, `since 18`,
  `SCHEMA_VERSION == 18`, docs table in lockstep.
- Frozen v1 key set remains a subset (additive only).
- `SettingsBridge`/`SettingsClient` test: default 0.5, clamp, persist, restart.
- `tst_settings_appearance.qml`: slider exists, binds to the key, writes on
  move, follows an external change; reduce toggle writes the boolean.

**Shell:**

- `compositorpolicy` unit test: `appearance.transparency` parsed, clamped,
  missing → 0.5; `reduceTransparency` resolves to solid.
- `ThemeBinding` test: key change updates `GlassTheme.transparency`.
- `tst_dock.qml`: with `materialBlurActive` true, the plate opacity equals
  `GlassTheme.dockFillOpacity`; with false, the fallback; changing
  transparency changes the fill without changing `panelRect`.
- `tst_menubar.qml` / popover tests: same for chrome/popup fills.
- No popover dismissal or entry rebuild on a transparency change (assert the
  surface configure count and the Repeater count).

**Compositor:**

- `material.rs` unit tests: pace endpoints, monotonicity, `blend3` at
  0/0.5/1, clamp, NaN.
- `backdrop.rs` tests: intensity 0.5 reproduces the token values; 1 raises
  refraction/lowers tint; 0 raises blur/tint and lowers refraction.
- `degrade.rs` tests: `reduceTransparency` + `Full` → `blur_active=false`,
  `UserDisabled`; `Reduced` drops refraction; `Minimal` drops everything.
- Render test: changing intensity does not change the panel rect or texture
  size; it only changes uniforms/damage.

**Visual / native:**

- Capture matrix: light/dark × transparency 0/0.5/1 × pale/colourful/dark
  wallpaper, at rest and magnified, with a menu bar, an open popover, and the
  Settings window.
- Check text contrast (WCAG-ish eyeball plus pixel sampling), no double edge,
  no banding, no halo, no clipped corner.
- Native DRM capture, not only nested (`docs/rendering.md` R6).

**Performance:**

- `material stats` at 4K: `blur_passes`, `blur_panels`, `blur_draws`,
  frame timing before/after; `reduceTransparency` must show `blur_passes=0`.
- Slider drag: no allocation growth, no configure storm, no frame spikes.

### 6.13 Migration and compatibility

- Additive key, default 0.5; existing users keep a coherent material.
- Protocol v8; older compositors ignore the request. The shell should log
  once when the compositor is too old, not per frame.
- If the default 0.5 is re-baselined to the new clear preset, call it out in
  the release notes and keep the old preset available via
  `DRAGONFRUIT_GLASS_PRESET=current` for one release.
- The QML fallback tokens stay the source of truth for no-blur states; do not
  delete them when adding the new material layer.

---

## 7. Token proposal

Keep the token surface small. Most optical constants are shared; only a few
vary by role.

**New `primitive.glass` (shared):**

| Token | Value | Notes |
|---|---:|---|
| `bevel` | 14 | Lip width in logical px. |
| `depth` | 16 | Surface height above the back face. |
| `ior` | 1.7 | Index of refraction. |
| `profile` | 2.5 | Superellipse exponent. |
| `chroma` | 0.05 | Red/blue split; 0 for the Dock (no rainbow fringe). |
| `lightAngle` | 50 | Degrees. |
| `ambientOcclusion` | 0.0 | Off by default; opt-in for large panels. |
| `ambientOcclusionRadius` | 4 | Logical px. |
| `rim` | 1.0 | Material-drawn lit edge. |

**Per-role overrides:**

| Token | Dock | Chrome | Popup |
|---|---:|---:|---:|
| `displacement` | 36 | 24 | 20 |
| `saturation` | 1.3 | 1.2 | 1.2 |
| `rim` | 1.0 | 1.0 | 0.0 |

`bevel`, `depth`, `ior`, `profile`, `chroma`, `lightAngle`, and the ambient
occlusion pair stay shared; Cutefish uses the same values for the Dock and its
popups, and only varies blur, displacement, saturation, and tint.

**Retuned existing tokens (balanced preset, light/dark):**

| Token | Current | Proposed | Why |
|---|---:|---:|---|
| `dockBlur` | 30 / 34 | 18 / 20 | Let the lens bend detail. |
| `dockRefraction` | 8 / 10 | 14 / 16 | The main lens strength. |
| `dockTint` | 0.22 / 0.20 | 0.16 / 0.14 | Less milk; adaptive tint still protects text. |
| `dockSpecular` | 0.5 / 0.34 | 0.45 / 0.30 | One subtle rim. |
| `chromeBlur` | 24 / 28 | 16 / 18 | Same direction, more conservative. |
| `chromeRefraction` | 6 / 8 | 10 / 12 | |
| `chromeTint` | 0.16 / 0.14 | 0.12 / 0.10 | Keep the menu bar readable with a fill floor. |
| `popupBlur` | 30 / 32 | 24 / 26 | Popups stay frosted for text. |
| `popupRefraction` | 8 / 10 | 10 / 12 | |
| `popupTint` | 0.2 / 0.18 | 0.28 / 0.26 | Raise readability, not lower it. |
| QML `plate.fillOpacity` | 0.42 | 0.0 when blur active; 0.42 fallback | One tint owner. |
| QML `plate.rimOpacity` | 0.7 | 0.0 when blur active; 0.7 fallback | One rim owner. |
| QML `plate.glossOpacity` | 0.22 | 0.0 when blur active; 0.22 fallback | One gloss owner. |
| QML `plate.edgeOpacity` | 0.5 | 0.0 when blur active; 0.5 fallback | One edge owner. |

These are proposals for capture review. The values should be generated from
`design-system/tokens/tokens.json` and `scripts/gen-tokens.py`; never
hand-edited in `Theme.qml` or `design_tokens.rs`.

---

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Text becomes unreadable on a busy/light wallpaper at the clear end. | Keep the adaptive tint; enforce role floors (menu bar 0.12, popups 0.20); capture matrix; a11y reduce toggle. |
| The clear preset looks "empty" or flat. | Keep the specular rim, saturation, and a subtle tint floor; compare against the reference captures, not just code. |
| Refraction shader is too expensive at 4K. | `Reduced` drops refraction; the reduce-transparency override disables the pass; measure before/after; keep the old shader behind a flag. |
| Double tint/edge persists because QML and compositor both change. | P0 is a prerequisite: one owner for body tint and rim; tests assert QML opacity is 0 when blur is active. |
| Slider writes cause surface reconfigures or popover dismissal. | Separate `applyMaterialSettings` path; no geometry changes; test configure counts. |
| Protocol version mismatch. | Additive v8; shell checks version; older compositor ignores; log once. |
| Client glass becomes a GPU DoS. | Allowlist/capability, per-surface area cap, rate limit; start with first-party only. |
| Ported shader license. | BSD-3-Clause attribution in `NOTICE`/`LICENSES`; keep the copyright header in the ported file. |
| Token churn breaks gallery goldens. | Material tokens are not gallery colors; add a material capture instead; run `scripts/check-gallery-snapshots.py`. |
| The slider feels non-linear or steppy. | Use the pace table + monotone cubic; test endpoints and midpoints; write on move for live feel. |

---

## 9. Suggested phasing

| Phase | Scope | Effort | Exit criteria |
|---|---|---|---|
| **A — P0** | Tint/rim ownership, clear preset behind a flag, capture baseline. | 1–2 days | Side-by-side captures; no double edge; approved look. |
| **B — P1** | Schema key, protocol v8, compositor intensity, Theme/ThemeBinding, Appearance slider, tests. | 2–3 days | Slider drives Dock + menu bar + popovers live; persists; tests green. |
| **C — P2** | Superellipse lens uniforms/shader, token generation, degrade behavior, perf. | 3–5 days | Lens visible at the lip; 60 Hz at `Full`; captures approved. |
| **D — P4/P5** | Client surface material protocol + shape masks; roll out to Settings/dialogs/popovers/OSD/notifications. | 1–2 weeks | Settings/dialog glass; tooltip arrow frosted; fallback coverage complete. |
| **E — P3/P6** | Mip-blur evaluation, header fade, shadow/corner polish. | opportunistic | Documented decision; no regressions. |

Do **A before B**. A slider on top of a double-tinted, triple-edged material
will only make the wrong look adjustable.

---

## 10. Open questions

1. **Placement:** Appearance pane (recommended, global) or Desktop & Dock?
2. **Default:** keep 0.5 at today's look for one release, or re-baseline 0.5
   to the new clear preset? Recommendation: re-baseline after the P0 capture
   review, with `DRAGONFRUIT_GLASS_PRESET=current` as the rollback.
3. **Key name:** `appearance.transparency` (recommended, user-facing) vs
   `appearance.glassIntensity` (more precise, less friendly).
4. **Accessibility toggle now or later?** Recommended now; it is small and
   gives a true off state.
5. **Client glass policy:** allowlist, capability token, or any client with a
   budget? Recommendation: allowlist first, design for a capability token.
6. **Rim ownership:** compositor rim (recommended, physically lit) vs QML
   hairline. Do not keep both.
7. **Should the menu bar be allowed to reach full clarity?** Recommendation:
   no; keep a 0.12 fill floor for text contrast.

---

## 11. Appendix — reading list

### Cutefish

- `cutefish-compositor/src/render/shaders/glass_fragment_shader.glsl` — the
  superellipse lens.
- `cutefish-compositor/src/render/glass.h` — `GlassParams` and rationale.
- `cutefish-compositor/src/shell/dock/dock_window.cpp:56–77, 385–390,
  432–461, 871–896` — pace curve, radius, dock glass, panel/glass update.
- `cutefish-compositor/src/shell/dock/dock_settings.cpp:64–75, 118–126` —
  `Dock/GlassTint`.
- `cutefish-compositor/src/shell/dock/dock.cpp:56` — D-Bus `setGlassTint`.
- `cutefish-compositor/src/shell/dock/dock_tooltip.cpp:120–135` — tooltip glass
  + bubble path.
- `cutefish-compositor/src/shell/osd/osd.cpp:130–136`,
  `.../clipboard/clipboard_picker.cpp:82–92`,
  `.../statusbar/status_bar_window.cpp:330–345` — role reuse.
- `cutefish-compositor/protocols/cutefish-surface-blur-v1.xml` — per-surface
  blur/glass/shape protocol.
- `cutefish-toolkit/src/ui/multi_effect_blur.cpp:107–128` — weights/levels.
- `cutefish-toolkit/src/ui/fast_blur.cpp:22–50` — fast blur variant.
- `cutefish-toolkit/src/ui/header_backdrop.cpp:33–38, 221, 352` — header
  fade/padding/tint.
- `cutefish-toolkit/src/ui/popup_glass.cpp` — popup readability defaults.
- `cutefish-toolkit/tests/data/fast_blur/` — reference blur fixtures at
  radius 6/20/48 and scale 1.0/1.5/2.0, useful for the P3 comparison.
- `settings/src/pages/dock/dockpage.cpp:66–77`,
  `settings/src/pages/dock/docksettings.cpp:52–59` — the slider.

### Dragonfruit

- `shell/dock/Dock.qml:3654–3810` — the current plate stack.
- `design-system/Theme.qml:157–179, 237–259, 368–381` — material and plate
  tokens.
- `compositor/src/window/blur.rs:112–205, 224, 333–380` — shader, lens factor,
  `GlassSpec`.
- `compositor/src/window/backdrop.rs:247–306, 322` — role glass and panel
  resolution.
- `compositor/src/window/degrade.rs:130–190, 280–330` — fallback enum and
  tier scaling.
- `compositor/src/render.rs:303–380` — `BlurPanel`.
- `protocols/dragonfruit-shell.xml:115` — `set_panel_rect`.
- `protocols/dragonfruit-toplevel.xml:209` — `set_motion_policy` pattern.
- `shell/src/themebinding.cpp:52–89` — settings → Theme.
- `shell/src/compositorpolicy.cpp:22–58` — settings → compositor policy.
- `shell/src/shellcontroller.cpp:4815, 4970` — Dock apply and compositor
  policy.
- `services/settingsd/src/schema.rs:22, 216+` — schema.
- `apps/settings/AppearancePane.qml` — slider placement.
- `design-system/components/Slider.qml` — slider captions/API.
- `docs/settings-keys.md` — the key table to update.

### Glossary

- **Bevel:** width of the rounded roll-off at the glass edge; the visible lip.
- **Depth:** how far the glass surface rises above its back face.
- **IOR:** index of refraction; higher bends the backdrop more.
- **Profile:** superellipse exponent; low = dome, high = flat top with a sharp
  lip.
- **Displacement:** how far the refracted ray reaches inside the glass.
- **Chroma:** red/blue split of the refracted sample; 0 disables the fringe.
- **Saturation:** backdrop colour intensity before tint.
- **Ambient occlusion:** darkening inside the edge; anchors the glass on a
  bright backdrop.
- **Rim:** a lit edge drawn by the material instead of the window outline.
