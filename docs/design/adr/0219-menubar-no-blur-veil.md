# 0219 — The menu-bar veil is a sharp depth LUT, not a blur

> Updated by [ADR 0243](0243-shell-material-focus-and-hover-corrections.md): see the current shell material, focus and hover policy.

## Status

accepted (T200)

## Context

The reference menu-bar backdrop preserves the live scene's frequency content:
checker attenuation is independent of checker size at a fixed depth. Over
black it reads as a white lift with alpha 0.451 at 5 pt, 0.439 at 15, 0.408 at
25, 0.357 at 35, 0.243 at 50, 0.059 at 75, 0.008 at 90 and zero at 95; over
white it is invisible. The reference foreground strip is 39 pt and the veil
ends by 95 pt. A standalone Dark veil was not measured in the first campaign;
the later coverage captures over neutral black/mid/white give a much subtler
black wash (~0.125 top alpha over mid-grey, invisible over white). The
reference text/glyphs also flip to black when the rendered bar is light.

The existing rail painted a QML chrome fill and, on the replacement path,
grouped the menu bar with the sampled persistent chrome. That wash both
duplicated paint and changed the live scene's frequency content, which the
menu bar is explicitly not supposed to do.

## Decision

- The menu-bar preset resolves to the explicit `no-blur-veil` mode and the
  sharp `RegionFilter::Sharp` path. It never allocates a Kawase chain and
  never increments `blur_draws`/`blur_passes`. The sharp element samples the
  compositor's live scene texture at the current pixel.
- The Light veil is a white lift driven by the measured depth LUT **and the
  measured background-luminance response**. The depth alpha scales by
  `1.0/1.06/0.91/0.48/0.0` at `L=0/0.25/0.5/0.78/1.0`, so the top-of-bar
  response is 0.451/0.477/0.409/0.218/0.0 over the report's five neutral
  backgrounds and the veil is invisible over white. The Dark baseline keeps
  the same depth shape at a 0.48 peak but uses a best-guess coverage curve that
  peaks at the measured mid-grey fit (~0.125 alpha) and falls to zero over
  white. Its near-black response is deliberately low: scaling the Light curve
  directly put the strongest black wash over a dark window, where the band is
  most visible. Both LUTs end at exactly zero by 95 pt.
- A bounded local luminance probe supplies the adaptation; there is no
  separate readability floor. The measured Light model is already readable,
  and the reference Dark captures show the text/glyph colour, not a darker
  veil, is what makes a light background readable. The displayed backdrop
  remains the sharp centre sample; the probe is not a spatial filter.
- The menu-bar foreground follows the rendered bar. The shell computes the
  active wallpaper's top-band sRGB luma and injects it into QML; `MenuBar.qml`
  applies the same depth LUT/adaptation and picks black or white text from the
  post-veil luminance. Light keeps dark text over the white lift; Dark uses
  light text over its black wash and flips to black when the veil disappears
  over a light wallpaper.
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
  The bounded scene texture may reuse a larger allocation from an earlier
  region; its source mapping is against the current region's content size, and
  an element leaving the region resets the damage tracker so the vacated strip
  is repainted rather than leaking stale pixels through the veil. Fullscreen
  suppression still hides the `top` layer, so the veil contributes no panel and
  the output remains direct-scanout eligible.
- Desktop icons start below the veil's 95 pt visual extent, not just the 28 pt
  reserved strip. The bar is transparent on the replacement rail, so an icon
  laid out inside the visual band would read as a ghost label under the menu
  bar even though the veil itself is correct.
- `query material` prints a `material veil mode=… extent=… strip=… alphas=…`
  diagnostic so a capture log can prove the no-blur mode and the LUT in use.

## Consequences

- The menu bar no longer needs a blur pass or a QML wash on the replacement
  rail, and its checker response is frequency-independent by construction.
- The Light veil now tracks the measured background luminance instead of
  applying the black-background LUT everywhere. This fixes the visible white
  band over light wallpapers that the fixed-LUT implementation produced.
- The visual extent can exceed the surface's committed/input strip without
  changing reservation or hit testing. The desktop-icon inset follows the
  visual extent, while normal windows still start below the 28 pt strip, so the
  veil stays visual-only for window placement.
- Dark is a documented best-guess fit to the coverage captures, not a
  controlled macOS profile; a dedicated Dark sweep can retune the curve later
  without changing the rendering model.
- The text/glyph flip is a best guess from the neutral captures. A dedicated
  wallpaper sweep can tune the two flip thresholds without touching the veil
  shader.
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
