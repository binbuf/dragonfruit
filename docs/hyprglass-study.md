# hyprglass study — porting notes for Dragonfruit's material

| | |
|---|---|
| **Status** | Research note (not a commitment) |
| **Date** | 2026-11-04 |
| **Sources** | `hyprnux/hyprglass` @ `317ed8c` (BSD-3-Clause); `shlok-shinde/hyprglass` branch `nothing-liquid` @ `3f174a7` (BSD-3-Clause); `shlok-shinde/nothing-liquid` @ `1ad5878` (GPL-3.0 scripts; behavior reference only) |
| **Related** | [`dock-learnings.md`](dock-learnings.md) §11–12, [`rendering.md`](rendering.md), ADR [0182](design/adr/0182-tahoe-liquid-glass-material-pass.md), ADR [0189](design/adr/0189-drm-sampled-material-default-and-fallbacks.md) |

## 0. Verdict

Yes — studying hyprglass was worth it, and the **`nothing-liquid` fork is the more
useful reference than upstream `hyprglass`** for Tahoe fidelity. Upstream is a
thick-slab lens with a key-light bevel/specular model. The fork replaces that
with a **squircle-bezel Snell model**, a **body lens**, **materialize-by-lensing**,
**touch light**, **clear/tinted** control, and a **per-shape alpha coverage
field**. Those are the pieces closest to Apple's Liquid Glass behavior.

What is worth porting:

1. **The squircle-bezel height model and Snell refraction.** It is smaller and
   more Tahoe-like than a generic edge falloff.
2. **The exact `tahoe_clear` / `tahoe` / `tahoe_window` presets.** They give us
   grounded numbers for blur, bezel, thickness, tint, adaptive tone, and rim.
3. **The alpha coverage field.** It lets a shell surface's actual drawn shape —
   dock plate, popover with an arrow, notification card, pill — define its own
   bezel, instead of using the layer's bounding rectangle. This can remove the
   need for a separate shape protocol for most shell surfaces.
4. **Materialize by modulating lensing**, not by fading opacity.
5. **Touch light.** A press point, a radius, an exponential falloff, and a rim
   line/band plus a spot. It is a relatively cheap, high-impact "alive" feature.
6. **The background cache/damage model.** Sample padding, scene generation,
   live-resample throttling, and damage of the padded box are the production
   pieces our current per-frame path needs.

What not to port: the Hyprland plugin hooks, ABI-sensitive integration, the
full-monitor temp FBO per layer, the `noblur` workaround, and the GPL
`nothing-liquid` scripts. Dragonfruit owns the compositor; we should not
recreate a plugin architecture.

---

## 1. What was pulled down

The study used shallow clones under `/tmp/opencode`:

- `hyprglass/` — upstream, `hyprnux/hyprglass` @ `317ed8c`, BSD-3-Clause.
- `hyprglass-nl/` — `shlok-shinde/hyprglass` branch `nothing-liquid` @
  `3f174a7`, BSD-3-Clause.
- `nothing-liquid/` — the shell/dotfiles/installer repo @ `1ad5878`; its
  scripts are GPL-3.0, so only its README and behavior were used.

The fork is a superset of upstream: it keeps `xray`, `self_sample`,
`focus_light`, `frost`, and `contour`, and adds the squircle-bezel model,
body lens, touch light, clear/tinted control, alpha coverage field, and the
Tahoe presets. The upstream `edge_height` lens is still present in the fork,
but the Tahoe presets use the squircle path.

License: both hyprglass trees are BSD-3-Clause (Copyright © 2025 Jeremy
Trufier). Any ported code must keep the copyright notice and be added to
`NOTICE` / `LICENSES/`. The `nothing-liquid` scripts are GPL-3.0; do not copy
them into Dragonfruit's MIT code.

---

## 2. The two models

### 2.1 Upstream: thick slab with an optional lens edge

Upstream models the pane as a thick convex slab. The base edge is a soft
exponential falloff; an optional `edge_height` turns the rim into a quarter
ellipse, with Snell refraction, and a `lens_reflection` Fresnel mirror. It
also has a key-light specular/bevel model: `bevel_angle` decides which side is
lit, and `specular_angle` biases the top highlight.

This is good, but it still reads as a lit slab. The default is a soft edge
(`edge_height = 0`), so its default look is frostier than its best preset.

### 2.2 Fork: squircle bezel, body lens, no key light

The fork models the rim as a convex **squircle bezel**, the profile Apple uses
for Liquid Glass. The pane is flat in the interior and rolls over to a
vertical wall at the outline. The height profile is:

```text
h(t) = T * (1 - (1 - t)^4)^(1/4),     t = depth / bezelWidth
```

where `T` is the glass thickness and `t` is the normalized depth from the
outline. The slope is:

```text
slope(t) = (1 - t)^3 * (1 - (1 - t)^4)^(-3/4)
```

A ray entering the surface at angle `atan(slope * T / bezelWidth)` is refracted
by Snell's law (`n1 = 1`, `n2 = IOR`), and the lateral shift at the backdrop
is:

```text
thetaI = atan(slope * T / bezelWidth)
thetaR = asin(sin(thetaI) / IOR)
shift  = h(t) * tan(thetaI - thetaR)
```

The fork uses `IOR = 1.5`, a dispersion of `chroma * 0.12` across R/G/B, and a
glass thickness of `refractionStrength * bezelWidth * materialize`.

Two things make this closer to Tahoe:

- **No key light.** There is no fixed bright corner. The rim is a crisp
  one-pixel line, nearly invisible unless it reflects something bright behind
  it, and it lights where the pane is touched.
- **The whole pane is a shallow convex lens**, not just the rim. A `bodyLensPx`
  term pulls the backdrop in across the whole face, strongest at the rim.

### 2.3 Bezel width is absolute, not relative

This is an important tuning difference:

```text
bezelPx = clamp(edgeThickness * 400 * monitorScale, 1, 0.5 * minDim)
```

`edgeThickness = 0.06` therefore means **24 logical pixels**, not 6% of the
pane. A dock or bar gets a real 24 px rim; a small pill whose half-height is
under 24 px becomes lens all the way through. That size-dependent behavior is
part of the real material. Upstream's `edgeThickness * minDim` scales with the
pane and gives a different look.

For Dragonfruit, this suggests a per-role `edgeThicknessPx`:

| Role | Suggested bezel | Why |
|---|---:|---|
| Dock | 20–24 px | The plate is tall enough for a real rim. |
| Menu bar | 10–14 px | The bar is thin; cap at half its height. |
| Popovers / menus | 12–16 px | Text panels need a visible but not dominant rim. |
| Small pills / tooltips | 8–12 px | The shape is small; cap at half its height. |
| Large dialogs | 16–20 px | The pane is large; the rim should not disappear. |

The shader already caps at `0.5 * minDim`, so a thin surface becomes lens all
the way through.

---

## 3. The fork's shader, piece by piece

### 3.1 SDF and crease-free edge distance

The shape is a rounded rectangle with per-corner radii and a `roundingPower`
(superellipse exponent). `getRoundedBoxSDF` gives the signed distance. The
bevel distance is computed with a corner-aware quadratic so the edge
refraction does not show a seam along the 45° corner diagonals. This is a
detail worth copying: a naive `max(q.x, q.y)` distance creases at the corners,
and the lens shows it.

### 3.2 Refraction direction

The fork uses two directions:

- `refractionDir` — pixel-space direction toward the pane center. Smooth
  everywhere, no SDF gradient needed.
- `edgeDir` — edge-following direction, blending through the diagonals. Used
  when `refractionFlow > 0` (the "pomme" preset uses flow 1.0).

For a Tahoe-like look, `refractionFlow = 1.0` on thin edges and `0` on large
panes is a good starting point.

### 3.3 Body lens

```text
bodyLensPx = lensDistortion * min(0.14 * minDim, 130 * monitorScale)
domeUV     = -q * bodyLensPx * (0.45 + 0.55 * r2) * materialize / fullSize
```

`q` is the normalized position across the pane (`-1..1`), `r2` is
`0.5 * dot(q,q)`. The result is a subtle magnification at the center and a
stronger pull at the rim. The fork disables it on full-monitor layers (overview)
because a single lens does not fit a screen full of separate panes. That is a
good rule for Dragonfruit's overview/Exposé surfaces too.

### 3.4 Materialize

```text
materialize = smoothstep(0, 1, glassOpacity)
thicknessPx = refractionStrength * bezelPx * materialize
```

The lensing ramps in with the pane's alpha instead of a cross-fade. This is
Apple's "materialize by modulating lensing" principle, and it is cheap: no
extra pass, just a uniform. Our current glass already has an `alpha` uniform;
we can add `materialize` as a derived factor and use it in the thickness and
body-lens terms.

### 3.5 Tone mapping and adaptive tint

The fork's tone stack is richer than a fixed tint:

```text
color = mix(vec3(lum), color, saturation)
dark:  color *= brightness * (1 - adaptiveDim * lumCurve)
light: color += vec3(adaptiveBoost * (1 - lumCurve) * 0.5)
color = mix(vec3(0.5), color, contrast)
color = mix(vec3(currentLum), color, 1 + vibrancy * sat * darkFactor)
color = mix(color, tintColor, tintAlpha)
if (tinted) color = mix(color, tintedColor, tinted * 0.62)
```

The dark/light defaults are deliberately mirrored:

| Setting | Dark | Light |
|---|---:|---:|
| `brightness` | 0.82 | 1.12 |
| `contrast` | 0.90 | 0.92 |
| `saturation` | 0.80 | 0.85 |
| `vibrancy` | 0.15 | 0.12 |
| `adaptiveDim` | 0.40 | 0.00 |
| `adaptiveBoost` | 0.00 | 0.40 |

This is a better model than our single adaptive-tint amount. Our
`GLASS_SHADER` already compares the backdrop luminance with the scheme tone and
mixes toward the tint; the fork's `adaptiveDim`/`adaptiveBoost` plus
`brightness`/`contrast`/`saturation`/`vibrancy` is a superset. We should merge
the two, not stack them: use our adaptive tint as the *amount* control, and the
fork's tone stack as the *color* path.

### 3.6 Rim, Fresnel, and inner shadow

The fork's rim is a one-pixel line:

```text
line = 1 - smoothstep(0, 1.25, depthPx)
band = exp(-depthPx / bevelSize) * 0.28
rim  = line * bevelStrength * rimLevel * (0.42 + 0.58 * env)
     + (line * 1.6 + band * 3.0) * touch
```

`env` is the luminance of the backdrop just outside the edge. Over a dark
backdrop the rim almost disappears; over a bright one it lights up. There is no
key light, so no corner is systematically brighter. Upstream's `bevelShadow`
darkens the side away from the light; the fork reuses the same uniform as a
shape-aware drop shadow just outside the coverage shape, which is how the Tahoe
presets use it for shell panels.

The Fresnel/inner glow is:

```text
glow = rimFall^2 * fresnelStrength * 0.10 * (0.3 + 0.7 * env)
```

It is a soft light scattered inside the curved rim, not a bright outline. This
is a good replacement for our current `specular` band + QML gloss stack.

### 3.7 Touch light

The fork's touch light is a global input state, not a per-surface animation:

```text
touch = pressGlow * specularStrength * exp(-2 * r2)
spot  = pressGlow * specularStrength * exp(-5 * r2)
r2    = dot(pixel - pressPos, pixel - pressPos) / pressRadius^2
```

The rim uses `line * 1.6 + band * 3.0` scaled by `touch`; the pressed spot adds
`spot * 0.16`. The light swells from the press point, follows the pointer while
held, and fades after release. The timing is in `Touch.cpp`:

- `RADIUS = 150` logical px reach
- `ATTACK_MS = 90` to full light
- `GROW_MS = 300` to full radius
- `RELEASE_MS = 650` fade out
- `TICK_MS = 8`

This is a very portable idea: Dragonfruit already has pointer/input state and
a compositor-owned render loop. We can pass a `pressPos`, `pressGlow`, and
`pressRadius` uniform to every glass panel; the shader does the rest. It does
not require per-surface animations or extra passes.

### 3.8 Shape-aware shadow

When the coverage is below the glass threshold, the fork draws a soft shadow
from the coverage field:

```text
shadowA = bevelShadow * pow(clamp(fieldCoverage(uv - (0, 3*scale)/fullSize) * 2, 0, 1), 1.6)
```

This gives arbitrary shapes a shadow that follows their outline, without a
separate shadow pass. It is a candidate for Dragonfruit's popovers,
notifications, and cards, where QML shadows can be clipped by the surface
bounds.

---

## 4. Exact Tahoe presets (the most useful artifact)

These are the fork's `BuiltInPresets.hpp` values, with alphas decoded. They are
a grounded starting point for our tokens, not final art direction.

### 4.1 Shared base (`applyTahoeBase`)

| Setting | Value | Note |
|---|---:|---|
| `blurIterations` | 2 | Two Gaussian passes (H+V) per iteration. |
| `edgeThickness` | 0.06 | 24 logical px via `* 400`. |
| `lensDistortion` | 0.8 | Body lens across the whole pane. |
| `fresnelStrength` | 0.15 | Soft inner glow. |
| `fresnelTint` | 0.4 | Glow picks up the backdrop hue. |
| `specularStrength` | 1.0 | Touch-light scale. |
| `bevelStrength` | 0.6 | Rim line. |
| `bevelSize` | 4.0 | Rim band width. |
| `bevelTint` | 0.2 | Rim picks up backdrop hue. |
| `bevelShadow` | 0.28 dark / 0.18 light | Shape-aware drop shadow. |
| `glassOpacity` | 1.0 | Materialize starts at full. |
| Dark `contrast` | 1.0 | |
| Dark `vibrancy` | 0.15 | |
| Dark `adaptiveBoost` | 0.0 | Dark mode never lifts the backdrop. |
| Light `contrast` | 1.0 | |
| Light `saturation` | 1.08 | |
| Light `vibrancy` | 0.12 | |
| Light `adaptiveDim` | 0.0 | Light mode never dims. |

### 4.2 `tahoe_clear` — bar, dock, chrome over content

| Setting | Value |
|---|---:|
| `blurStrength` | 0.2 → **2.4 px** |
| `refractionStrength` | 1.8 |
| `chromaticAberration` | 0.5 |
| `bevelStrength` | 0.75 |
| Dark `brightness` | 0.80 |
| Dark `saturation` | 1.10 |
| Dark `adaptiveDim` | 0.80 |
| Dark `tintColor` | `0x0a0a0c40` (near-black, alpha **0.251**) |
| Light `brightness` | 1.04 |
| Light `adaptiveBoost` | 0.55 |
| Light `tintColor` | `0xffffff2a` (white, alpha **0.165**) |

This is the closest preset to the Dock/menu bar: very low blur, strong lens,
a dark smoked tint in dark mode and a milky lift in light mode.

### 4.3 `tahoe` — panels that carry text (launcher, sidebars, notifications)

| Setting | Value |
|---|---:|
| `blurStrength` | 1.0 → **12 px** |
| `bevelStrength` | 0.85 |
| `refractionStrength` | 1.3 |
| `chromaticAberration` | 0.35 |
| Dark `brightness` | 0.70 |
| Dark `saturation` | 1.10 |
| Dark `adaptiveDim` | 0.85 |
| Dark `tintColor` | `0x0b0b0e66` (near-black, alpha **0.400**) |
| Light `brightness` | 1.03 |
| Light `adaptiveBoost` | 0.85 |
| Light `tintColor` | `0xfbfbfd70` (near-white, alpha **0.439**) |

Use this for popovers, menus, notifications, Control Center, and any panel
where text must stay readable over a busy backdrop.

### 4.4 `tahoe_window` — app windows

| Setting | Value |
|---|---:|
| `blurStrength` | 1.2 → **14.4 px** |
| `blurIterations` | 3 |
| `refractionStrength` | 1.2 |
| `chromaticAberration` | 0.3 |
| `edgeThickness` | 0.05 → **20 px** |
| `fresnelStrength` | 0.12 |
| `bevelStrength` | 0.75 |
| Dark `brightness` | 0.70 |
| Dark `saturation` | 1.10 |
| Dark `adaptiveDim` | 0.50 |
| Dark `tintColor` | `0x0c0c0f58` (near-black, alpha **0.345**) |
| Light `brightness` | 1.0 |
| Light `adaptiveBoost` | 0.70 |
| Light `tintColor` | `0xf5f5f780` (near-white, alpha **0.502**) |

This is for Settings, Files, dialogs, and any client window that can show a
translucent body. It is intentionally frostier than `tahoe_clear`.

### 4.5 What the numbers mean for our current tokens

| Our token | Current | `tahoe_clear` equivalent | `tahoe` equivalent |
|---|---:|---:|---:|
| `dockBlur` | 30 / 34 | 2.4 px | 12 px |
| `dockRefraction` | 8 / 10 px | `refractionStrength` 1.8 (thickness / bezel) | 1.3 |
| `dockTint` | 0.22 / 0.20 | 0.251 dark / 0.165 light | 0.400 / 0.439 |
| `dockSpecular` | 0.5 / 0.34 | 1.0 (touch scale) | 1.0 |
| `dockSpecularWidth` | 2.5 | `bevelSize` 4 | 4 |
| `dockBevel` | n/a | 0.75 | 0.85 |
| `dockLens` | n/a | 0.8 body lens | 0.8 |
| `dockEdgeThickness` | n/a | 24 px | 24 px |

**Unit change.** Our `dockRefraction` is in pixels; hyprglass's
`refractionStrength` is a thickness ratio (`thicknessPx = strength * bezelPx`).
Do not map 8 px to 1.8 directly. Introduce a `thickness` token (ratio) and keep
the existing px token only for the fallback/feather path, or rename it.

---

## 5. The alpha coverage field (the shape trick)

This is the most valuable architectural idea for Dragonfruit.

Hyprglass needs glass on arbitrary layer shapes: a bar, a dock, a notification
card, a popover with an arrow, a pill. It cannot assume the layer's rectangle.
It solves this in two ways:

1. **Protocol region** — a client declares a blur region
   (`ext-background-effect-v1`). Glass is drawn only inside those rectangles.
2. **Alpha coverage field** — the compositor renders the layer's surface into a
   temp FBO, thresholds its alpha into "glass / not glass", and Gaussian-blurs
   it into a coverage field. From that field it derives:

   ```text
   depthPx = fieldSigmaPx * probit(coverage)
   normal  = normalize(gradient(coverage))
   ```

   The field is a smooth proxy for the signed distance to the drawn shape. The
   bezel, lens, rim, and shadow all follow it. A dock plate, an arrow-shaped
   popover, and a stack of notification cards each get their own rim without a
   separate shape protocol.

Why this matters for Dragonfruit:

- Our shell QML already draws the real shapes: the Dock plate, the menu bar
  strip, popovers with arrows, OSD cards, notification cards. Their alpha
  channel is the shape.
- We do not need to add a `set_shape` protocol for every shell surface. We can
  render the chrome surface to a temp FBO (we already have the surface buffer),
  build a half-resolution coverage field, and let the shader use it.
- For client windows, the buffer is usually opaque, so the coverage field is
  not useful; those still need a declared region/shape. That is where P4's
  `df_surface_material` protocol comes in.

Cost: one temp FBO the size of the monitor for the surface alpha (hyprglass
uses monitor-sized), plus two Gaussian passes on a half-resolution field. The
fork rebuilds the field only when the layer's content or geometry changes, and
uses a stale field for a frame or two if it cannot rebuild. We can do better:
we can render only the panel rect, not the whole monitor, and cache the field
per panel.

**Recommendation:** implement the coverage field for shell surfaces first. It
is the cheapest way to make popovers, arrows, cards, and the Dock plate all
share one material without a new protocol.

---

## 6. Performance and caching

Hyprglass is built for production; these are the pieces we should copy.

### 6.1 Sample padding

`SAMPLE_PADDING_PX = 60` device pixels (about 30 logical px at 2x). The
backdrop is sampled and blurred with that margin around the pane so the lens
can pull in content from beyond the edge without reading clamped/black texels.
The same padding is added to damage so the margin is re-rendered.

Our current `set_panel_rect` path samples the scene texture at the panel rect;
we need a padding margin for the new lens. Start with 32–60 px depending on
bezel size; the lens reach is at most the bezel plus the body lens.

### 6.2 Blur

- `blurRadius = blurStrength * 12 / downscale`.
- `downscale = 2` when `blurStrength >= 0.35`, else 1.
- `blurIterations` 1–5; default 3; Tahoe presets use 2–3.
- Two-pass Gaussian (H+V) per iteration.
- `foldBlurPasses` reduces the pass count when the radius is large:
  `totalRadius = radius * sqrt(iterations)`, `cappedRatio = totalRadius / 8`,
  `foldedIterations = max(1, ceil(cappedRatio^2))`.
- The Gaussian shader pairs adjacent taps into one bilinear fetch, halving the
  tap count.

Our Kawase chain is cheaper per pass but has a different look. The study
suggests benchmarking Gaussian vs Kawase after the lens is in place; the
Gaussian's quality at large radii is better, but our Kawase may be sufficient
once the blur radius drops to 2–14 px.

### 6.3 Background cache

Each window/layer caches its sampled+blurred background. It resamples only when:

- the scene generation for the monitor changed (a window moved, resized,
  committed damage behind the glass);
- the glassed window/layer itself moved, resized, or animated;
- the monitor/scale/format changed;
- the x-ray mode changed;
- a workspace slide/fade is animating.

On a cache hit, the expensive blit + blur passes are skipped. On a cache miss,
the sample and blur run; if the damage does not cover the padded box, the
resample is deferred and the stale cache is drawn for a frame.

Our compositor has a scene texture per output and a damage tracker. We can add
a per-panel cache keyed by `(panel rect, scene generation, scale, format,
preset, intensity)`. The scene generation should bump on any damage behind the
panel. This is the single biggest performance win for a 4K session.

### 6.4 Live resample

`windows_live_resample` / `layers_live_resample` re-render when content behind
changes (a playing video, another window). `*_live_resample_fps` caps the
resample rate (default 30). A static scene costs nothing. This is the right
model for a shell that must stay at 60 Hz while a video plays behind the Dock.

### 6.5 Damage

Damage the padded box, not just the pane. Otherwise the sampling margin keeps
stale pixels and produces noise at the edges. On a cache hit, no damage is
needed. On a miss, damage the padded box and force a re-render.

### 6.6 Dedupe

Hyprland renders a floating window over fullscreen more than once per frame.
Hyprglass stamps a frame serial and queue index so the glass is applied exactly
once, sampling the framebuffer under the last copy. Our compositor has its own
render ordering; we need an equivalent "one material pass per panel per frame"
guard, which our `material_pass` already provides.

---

## 7. Dynamics

### 7.1 Materialize

`materialize = smoothstep(0, 1, glassOpacity)`. Ramp the lens thickness and
body lens with it. This is the cheapest Apple-like behavior and should be in
the first shader port.

### 7.2 Touch light

Port the `TouchLight` state machine:

- On press over a glass surface, record `pos`, `pressAt`, `held`.
- While held, follow the pointer.
- On release, start a 650 ms fade.
- Tick every 8 ms; damage the previous and new light boxes.
- Pass `pressPos`, `pressGlow`, `pressRadius` to every glass panel.

The shader uses the point in pane-local pixels. Rotated outputs can disable it
initially (as hyprglass does).

### 7.3 Focus light

The active window's rim is lit all the way round, brightest on the side the
bevel light comes from. The fork keeps this, but the Tahoe presets use touch
light as the primary interaction. For Dragonfruit, start with touch light;
focus light is a nice secondary.

### 7.4 Frost

`frost` blurs the glass further where other windows are behind it, and leaves it
clear over the wallpaper. It is a low blur strength that makes the material
read as glass when it overlaps content. This is a later-stage feature; it
requires knowing which windows are behind the pane.

### 7.5 Scroll edge

Hyprglass does not implement Apple's scroll-edge dissolve; its layer glass is a
static material. This remains a Dragonfruit differentiator. The `HeaderBackdrop`
idea from Cutefish (fade over 16 px of scrolling content) is a simpler starting
point.

---

## 8. Concrete port plan for Dragonfruit

### Stage 1 — Shader (highest visual impact)

Replace the current simple lens in `compositor/src/window/blur.rs` with a
`GLASS_SHADER_V2` based on the fork's squircle model:

- Keep the rounded-rect SDF and per-corner radii.
- Add a crease-free bevel distance (corner-aware quadratic).
- Add `bezelPx`, `thicknessPx`, `bodyLensPx`, `dispersion`, `materialize`,
  `tinted`, `tintedColor`, `rimLevel`, `bevelStrength`, `bevelSize`,
  `bevelTint`, `bevelShadow`, `fresnelStrength`, `fresnelTint`, and the tone
  uniforms (`brightness`, `contrast`, `saturation`, `vibrancy`,
  `vibrancyDarkness`, `adaptiveDim`, `adaptiveBoost`).
- Keep our adaptive tint as the amount control; merge it with the fork's tone
  path, do not stack two tints.
- Start with the `tahoe_clear` preset for the Dock and the `tahoe` preset for
  text panels; keep `tahoe_window` for client surfaces.

Acceptance: the Dock edge shows a compressed/magnified band, a single rim, a
flat interior, and no double edge; 60 Hz at 4K on the reference GPU.

### Stage 2 — Coverage field for shell surfaces

- Render the chrome surface to a temp FBO (panel-sized, not monitor-sized).
- Threshold the alpha and Gaussian-blur into a half-resolution coverage field.
- Derive `depthPx` and the inward normal from the field.
- Use it for the Dock plate, menu bar, popovers, OSD, notifications, Control
  Center, and Settings/dialog cards.
- Keep the existing `set_panel_rect` as the fallback shape when the field is
  not available (first frame, rotated output, software path).

Acceptance: a popover arrow, a notification card, and a pill all get their own
rim and shadow; no per-surface shape protocol is needed for shell QML.

### Stage 3 — Caching and damage

- Add `SAMPLE_PADDING_PX` (start 32–60) around the panel.
- Add a per-panel background cache keyed by scene generation, panel rect,
  scale, format, preset, and intensity.
- Damage the padded box on a miss; skip the sample/blur on a hit.
- Add live-resample throttling (30 fps default) for video behind the glass.
- Reuse textures across frames; no steady-state allocation.

Acceptance: a stationary Dock with a playing video behind it stays at 60 Hz;
no full-output damage; no allocation growth over a 5-minute run.

### Stage 4 — Dynamics

- Materialize from the surface alpha.
- Touch light: press/held/release state, 8 ms tick, uniforms to all panels.
- Focus light (optional), frost (later), scroll edge (later).

Acceptance: pressing the Dock lights the rim from the contact point; a pane
opening ramps its lens rather than cross-fading; reduced motion disables the
elastic parts.

### Stage 5 — Presets and tokens

- Add `tahoe_clear`, `tahoe`, and `tahoe_window` role presets.
- Map the exact values from §4 into `design-system/tokens/tokens.json` and
  generate with `scripts/gen-tokens.py`.
- Keep the existing degrade tiers; at `Reduced`, drop refraction; at `Minimal`,
  drop blur/glass.
- Add the `appearance.transparency` slider from `dock-learnings.md` as the
  Clear/Tinted control, with `reduceTransparency` as the solid fallback.

Acceptance: the slider moves between the `tahoe_clear` and `tahoe` characters
without a rebuild; text stays readable; the fallback is milky.

### Stage 6 — Calibration

- Capture the Dock, menu bar, popovers, OSD, notifications, Settings, and a
  dialog at 1x/2x, light/dark, pale/colorful/dark wallpaper.
- Compare against macOS Tahoe on the rubric in `dock-learnings.md` §12.4.
- Measure lens profile, edge falloff, tint response, shadow response,
  materialize, press light, and scroll edge.
- Do not declare "nearly perfect" from a single screenshot.

---

## 9. What not to port

- **Hyprland plugin hooks and ABI tricks.** `CGlassDecoration`,
  `CGlassLayerSurface`, `CGlassPassElement`, and the `noblur` property exist
  because Hyprland does not own the material. Dragonfruit does.
- **Full-monitor temp FBO per layer.** We can render only the panel rect and
  cache the coverage field. This is a chance to be cheaper than hyprglass.
- **The `noblur`/`manage_window_blur` workaround.** Our compositor controls the
  order; no double-blur workaround is needed.
- **`xray` and `self_sample` initially.** They add complexity and are not
  required for the first Tahoe look. Revisit after the base material is stable.
- **GPL `nothing-liquid` scripts.** Behavior reference only.
- **Apple assets.** No wallpapers, fonts, sounds, or bitmaps.
- **A key-light bevel model.** The fork deliberately removed it; the rim should
  be environment-lit and touch-lit, not lit from a fixed corner.

---

## 10. Tests and calibration

### Shader math

- `h(t)` and `slope(t)` monotonic on `[0,1]`; endpoints exact.
- `refractShiftPx` is zero at the flat center and maximal at the outline.
- `bezelPx` clamps correctly for a thin pill, a dock, and a 4K panel.
- `materialize` at 0/0.5/1 produces 0/0.5/1 thickness and body lens.
- Dispersion is zero when `chroma = 0`; red/blue shifts are symmetric around
  green.

### Visual

- Dock/menu bar/popover at 1x and 2x, light/dark, pale/colorful/dark wallpaper.
- Lens profile: sample a vertical line across the edge and compare the
  compression/magnification band to the reference.
- Rim: one line, environment-lit, no fixed bright corner.
- Shadow: follows the shape, deeper over text, recedes over light content.
- Touch: press at a known point; the rim within 150 px lights, the spot glows,
  release fades in ~650 ms.
- Materialize: open a popover; lensing ramps, not alpha.

### Performance

- 4K, 60 Hz; `blur_passes`, `blur_panels`, `blur_draws`, `blur_active`.
- Cache hit rate on a static Dock; no full-output damage.
- No texture allocation after warm-up; no GPU memory growth over 5 minutes.
- `reduceTransparency` shows `blur_passes=0`.

### Accessibility

- Reduced transparency: frostier, opaque fallback.
- Increased contrast: black/white surface with a contrasting border.
- Reduced motion: no elastic/lensing animation; materialize snaps.

---

## 11. Open questions

1. **Bezel width:** absolute px per role (recommended) or relative to the
   pane? The fork's `edgeThickness * 400` is absolute; our current
   `dockRefraction` is px. Decide the token unit before porting.
2. **Coverage field for all shell surfaces?** The cost is a temp FBO plus two
   half-res Gaussian passes. Start with non-rect shapes (popovers, cards,
   pills) and the Dock plate; keep the panel rect for the menu bar until
   measured.
3. **Tone stack:** merge our adaptive tint with the fork's
   brightness/contrast/saturation/vibrancy/adaptiveDim/adaptiveBoost, or keep
   two separate paths? Merge; two tints will double up.
4. **Blur:** Gaussian vs Kawase. Benchmark after the lens is in place; the
   smaller radius may make Kawase sufficient.
5. **Shadow ownership:** compositor shape-aware shadow vs QML shadow. Prefer
   compositor for arbitrary shapes; keep QML for the fallback.
6. **Touch light scope:** Dock/popovers only, or every glass surface? Start
   with all chrome surfaces; windows later.
7. **`xray`/`self_sample`:** defer; they are not needed for the first look.

---

## 12. Appendix — code map

### Upstream (`hyprnux/hyprglass` @ `317ed8c`)

- `src/Shaders.hpp` — `liquidglass.frag` (thick slab, `HYPRGLASS_FEATURE_LENS_EDGE`,
  Fresnel reflection, focus light, frost, contour), `gaussianblur.frag`,
  `maskfield.frag`, `contourseed.frag`, `contouredt.frag`,
  `contoursmooth.frag`, `contourfield.frag`.
- `src/GlassRenderer.cpp` — `sampleBackground`, `blurBackground`,
  `buildMaskField`, `applyGlassEffect`, uniform setup, body lens.
- `src/GlassDecoration.cpp` — window glass, background cache,
  `wantsBackgroundResample`, `renderPass`, focus light/frost.
- `src/GlassLayerSurface.cpp` — layer sampling/redirect, alpha/region/contour
  masks, composite.
- `src/BackgroundDamageObserver.cpp` — scene generation, commit/damage
  invalidation, live resample.
- `src/PluginConfig.hpp` — `SPresetValues`, `SOverridableConfig`,
  `SCustomPreset`, resolution chain.
- `src/BuiltInPresets.hpp` — `pomme` and the other presets.
- `src/Frost.cpp` — covered-fraction frost.
- `src/Contour.cpp` — contour field build/caching.

### Fork (`shlok-shinde/hyprglass` @ `3f174a7`)

- `src/Shaders.hpp` — squircle bezel, `squircleHeight`, `squircleSlope`,
  `refractShiftPx`, body lens, materialize, touch light, mask field,
  shape-aware shadow.
- `src/BuiltInPresets.hpp` — `tahoe_clear`, `tahoe`, `tahoe_window`,
  `applyTahoeBase`.
- `src/Touch.cpp` — press/held/release, radius, timing, `hyprctl` test hooks.
- `src/GlassRenderer.cpp` — `bodyLensPx`, `pressPos/Glow/Radius`, `tinted`,
  `rimLevel`, field uniforms.
- `src/GlassLayerSurface.cpp` — alpha coverage field, mask feather, fixed shape,
  composite.
- `src/PluginConfig.hpp` — `tinted`, `tintedBlurScale`, `edgeHighlight`, `lens`,
  `edgeThicknessPx`.

### Dragonfruit targets

- `compositor/src/window/blur.rs` — `GLASS_SHADER`, `GlassSpec`, uniforms.
- `compositor/src/window/backdrop.rs` — `MaterialRole`, token resolution.
- `compositor/src/render.rs` — `BlurPanel`, `chrome_blur_panels`.
- `compositor/src/window/degrade.rs` — tier scaling, `EffectiveMaterial`.
- `protocols/dragonfruit-shell.xml` — `set_panel_rect`; optional shape.
- `shell/dock/Dock.qml`, `shell/menubar/MenuBar.qml`,
  `design-system/components/Popover.qml` — shapes/fills.
- `design-system/tokens/tokens.json` — new material tokens.
- `scripts/gen-tokens.py` — generation.
- `services/settingsd/src/schema.rs` — `appearance.transparency`.
- `shell/src/themebinding.cpp`, `shell/src/compositorpolicy.cpp` — live apply.
- `apps/settings/AppearancePane.qml` — slider.

### Porting checklist

- [ ] Decide token units (bezel px vs ratio; thickness ratio vs px).
- [ ] Port the squircle-bezel shader with `tahoe_clear` defaults.
- [ ] Keep our adaptive tint; merge the tone stack once.
- [ ] Add the alpha coverage field for non-rect shell surfaces.
- [ ] Add sample padding and the background cache.
- [ ] Add materialize and touch light.
- [ ] Add the `tahoe`/`tahoe_window` presets.
- [ ] Add the transparency slider and reduced-transparency fallback.
- [ ] Calibrate against macOS Tahoe; native DRM captures.
- [ ] Add BSD-3-Clause attribution to `NOTICE` / `LICENSES`.
