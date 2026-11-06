# 0219 — The menu-bar veil is a sharp depth LUT, not a blur

## Status

accepted (T200)

## Context

The reference menu-bar backdrop preserves the live scene's frequency content:
checker attenuation is independent of checker size at a fixed depth. Over
black it reads as a white lift with alpha 0.451 at 5 pt, 0.439 at 15, 0.408 at
25, 0.357 at 35, 0.243 at 50, 0.059 at 75, 0.008 at 90 and zero at 95; over
white it is invisible. The reference foreground strip is 39 pt and the veil
ends by 95 pt. A standalone Dark veil was not measured.

The existing rail painted a QML chrome fill and, on the replacement path,
grouped the menu bar with the sampled persistent chrome. That wash both
duplicated paint and changed the live scene's frequency content, which the
menu bar is explicitly not supposed to do.

## Decision

- The menu-bar preset resolves to the explicit `no-blur-veil` mode and the
  sharp `RegionFilter::Sharp` path. It never allocates a Kawase chain and
  never increments `blur_draws`/`blur_passes`. The sharp element samples the
  compositor's live scene texture at the current pixel.
- The Light veil is a white lift driven by the measured depth LUT. The Dark
  baseline is a black wash with the same depth shape scaled to a 0.48 peak.
  Both LUTs end at exactly zero by 95 pt.
- The veil may adjust its alpha from a bounded local luminance probe inside
  the 39 pt reference strip: Light raises a too-dark strip toward 0.35
  post-veil luminance; Dark lowers a too-bright strip toward 0.20. The rule
  only raises the LUT alpha, so the measured black-background samples do not
  move. The displayed backdrop remains the sharp centre sample; the probe is
  not a spatial filter.
- The layer surface is configured to the 95 pt visual extent, but the
  exclusive zone, input buffer and committed strip remain Dragonfruit's 28 pt
  bar. The lower 67 pt is visual-only. The shell declares a `veil` panel
  region of the full extent and clears it when the veil is disabled, hidden,
  in Reduce Transparency, or on a backend without the sharp scene texture.
- In the replacement rail the QML surface paints no fill; the compositor veil
  sits below the foreground and menu popups. The legacy rail keeps the old
  sampled fill. When the compositor cannot draw the veil, QML draws the
  scheme-appropriate milky fallback plate.
- The veil element's commit follows the scene cache's damage commit, so a
  moving window under the bar damages the veil while a settled scene does not.
  Fullscreen suppression still hides the `top` layer, so the veil contributes
  no panel and the output remains direct-scanout eligible.
- `query material` prints a `material veil mode=… extent=… strip=… alphas=…`
  diagnostic so a capture log can prove the no-blur mode and the LUT in use.

## Consequences

- The menu bar no longer needs a blur pass or a QML wash on the replacement
  rail, and its checker response is frequency-independent by construction.
- The visual extent can exceed the surface's committed/input strip without
  changing reservation or hit testing.
- Dark is a documented Dragonfruit baseline, not a measured macOS profile;
  a controlled Dark capture can retune the LUT later without changing the
  rendering model.
- A driver that refuses the veil shader falls back to the flat-band glass
  plate (or, if that also fails, the raw sharp sample); the QML fallback is
  reserved for a genuinely absent material path.

## References

- `compositor/src/window/menubar_veil.rs`
- `compositor/src/window/blur.rs` (`VEIL_SHADER`)
- `compositor/src/render.rs` (`chrome_blur_panels`)
- `shell/menubar/MenuBar.qml`, `shell/src/shellcontroller.cpp`
- `scripts/t200-menubar-veil.py`, `docs/captures/t200-menubar-veil-model.json`
- [REFERENCE.md](../../tasks/glass/plan/REFERENCE.md)
