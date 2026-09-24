# 0013 — The chrome backdrop is a token-driven compositor pass

## Status

accepted

## Context

T-04.2 adds the material *under* translucent chrome surfaces (menu bar, Dock,
popovers/menus) so they read as frosted glass instead of flat rectangles.
`design-system/tokens/tokens.json` already publishes `material.chromeBlur` /
`chromeOpacity` and `material.popupBlur` / `popupOpacity`, and
`TitleBar.qml`/`Dock.qml` already consume them; the design docs require the
effect to be a compositor render pass over **live** surfaces (never a
screenshot or a client re-render) and to be **one pass per frame**.

The compositor renderer is still a flat solid-color renderer (no shader, no
vector rasterizer): every shape so far — traffic lights, glyphs, shadows, the
rounded `CornerMask` spans — is built from rectangles. A texture-sampling
gaussian/Kawase blur is a renderer feature that does not exist yet, and the
element pipeline has no offscreen scene texture to sample. The track also
needs the pass to be **swappable behind the tokens** and to degrade (T-04.4a:
smaller radius, then blur off) under budget pressure.

Two questions had to be answered together: who owns the pass, and what the
backdrop *is* until a GPU sampler exists.

## Decision

- Add `compositor/src/window/backdrop.rs` as the single backdrop-pass model.
  A `MaterialRole` (`Chrome` for `df_shell.layer` < 3, `Popup` for overlay
  chrome) resolves a `BackdropSpec` **only** from the generated tokens:
  `material.chrome*` / `material.popup*` per `ColorScheme`, plus the component
  radius (`component.menu_bar.radius` / `component.popup.radius`). No blur
  radius, opacity, or radius is hardcoded; changing the material means editing
  the token and regenerating (`make check-tokens`).
- The backdrop is a **stack of translucent rounded feather layers** clipped
  with the shared `CornerMask` geometry (the same decomposition as the T-04.1b
  shadow/titlebar), drawing the scheme's chrome/elevated tone at
  `opacity / layers`, from the full panel edge inward. This is the software
  stand-in for a sampled blur, exactly as T-04.1a's layered rectangles stand in
  for a blurred shadow; the token blur sets the feather depth and the derived
  layer count. It is a *separate element drawn below the chrome Wayland
  surface*, so a client's translucent regions blend over it instead of being
  punched out (FR-3).
- `BackdropPass` owns the pass bookkeeping: the renderer opens a frame once
  (`DfState::begin_render_frame`), then each output applies the backdrop at
  most once. A repeated `(frame, output)` request is counted as `skipped` and
  draws nothing, which is the T-04.2 "one effect pass per frame, no
  double-blur" invariant and the counter T-04.4a instruments.
- `render::chrome_backdrop_render_elements` is the single builder; callers
  append its elements **after** the chrome surfaces and **before** the windows,
  so the panel sits behind its chrome and in front of the scene it stands in
  for. The pass never forces a redraw on its own, so the idle trace stays at
  zero damage.
- A real texture-sampling blur (Kawase vs dual-pass Gaussian) is **deferred**,
  kept behind the token values this pass already reads. This pass is the seam
  T-04.3 folds the transform into and T-04.4a degrades; when the sampler lands
  it replaces the layer geometry, not the pass contract.

## Consequences

- Chrome shows a token-derived frosted backdrop with token-rounded, feathered
  edges that cannot drift from the QML `Theme.material` group; the pass and its
  damage are asserted headless (no GPU): token roles, feather containment, the
  rounded union of the chrome band, and the one-pass/skip counters. `make e2e`,
  `make qml-test`, and the gallery goldens stay green.
- The backdrop is a **flat-tone approximation**, not a true sample of the live
  scene: it does not yet blur video/text under the bar. This is the same
  approximation status the T-04.1a shadow shipped with, and it is visually
  correct enough to sign off against the design reference. The GPU blur is the
  remaining T-04 work and is swappable without touching the pass contract.
- The chrome band owns extra damage while chrome is present; when no chrome
  surface is mapped the builder returns nothing, so a bare desktop is
  unaffected. The pass runs only on frames the backend actually renders.
- `ChromeSurface` now carries its output-local `geometry` (T-04.2); the
  compositor still composites by `location`, and the material pass reads the
  rectangle.
