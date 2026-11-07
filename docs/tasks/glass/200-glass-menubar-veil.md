---
verify: make gates
---
# T200 — Render the no-blur menu-bar veil

## Goal

Replace the menu-bar backdrop with a readable, luminance-adaptive veil that preserves the frequency content of the live scene.

## Context (read first)

- Depends on T194–T197. Read the [veil samples](plan/REFERENCE.md#what-to-carry-forward) and reference `docs/measurements.json` under `menu_bar`.
- `shell/menubar/MenuBar.qml`, `compositor/src/render.rs::chrome_blur_panels`, `compositor/src/window/backdrop.rs`, and the DRM/nested material passes.

## Scope

- [x] Render the veil against the compositor's sharp live scene, with the Light depth LUT ending by 95 logical pt. Calibrate 39 pt as the reference strip height and preserve Dragonfruit's existing layout needs. Keep the full veil visual-only beyond the strip.
- [x] Calibrate local luminance adaptation using the measured neutrals/checkers and foreground readability over moving windows. Implement a documented Dark baseline and accessibility/no-sampling variants. Keep fallback colours scheme-appropriate.
- [x] Replace the old QML-plus-compositor wash on the new rail. Use an explicit no-blur mode and a veil diagnostic. Preserve output damage, reservation, rendering order and fullscreen scanout policy.

## Out of scope

- Menu panel glass (T202), visibility setting and reveal behavior (T201), and matching an unmeasured macOS Dark profile.

## Design notes

Check alpha at the actual sample depth. The reference 0.451 over black is at y=5 pt; the text-free black LUT drops to 0.008 at 90 pt and zero at 95 pt. Foreground decisions may use a bounded local/strip luminance measure, with stable contrast over mixed content. Ensure the adaptation does not introduce spatial filtering into the displayed backdrop. The veil sits below foreground and menu popups. A disabled or hidden veil must contribute no visual element or sampled work.

## Done when

- [x] Light black LUT α within ±0.03 at the reference samples, zero by 95 pt, and no checker-size-dependent loss at a fixed depth within fixture tolerance. Show live-window response, readable Dark/fallback output, and no additional blur pass for the veil.
- [x] Automated tests cover extent, output isolation, input/reservation and fullscreen suppression. `tst_menubar` and `make gates` pass. Fill in Hand-off.

## Hand-off

**Mode.** `menuBarVeil` resolves to the explicit `no-blur-veil` mode and the
sharp compositor path. `VEIL_SHADER` samples the live scene at the current
pixel; there is no Kawase allocation, no `blur_draws`/`blur_passes` increment,
and no QML fill on the replacement rail. The legacy rail keeps its sampled QML
fill; Reduce Transparency, a hidden bar, or no sampled scene texture commits an
empty panel set and QML paints the scheme-appropriate milky fallback.

**Light/Dark policy.** Light is a white lift with the measured depth LUT
(0.45098 at 5 pt, 0.43922 at 15, 0.40784 at 25, 0.35686 at 35, 0.24314 at 50,
0.05882 at 75, 0.00784 at 90, zero at 95). Dark is the documented black
baseline with the same shape scaled to a 0.48 peak. The reference strip is
39 pt; Dragonfruit keeps its existing 28 pt bar layout and reservation. A
bounded 5-tap luminance probe inside the strip can only raise the LUT alpha:
Light targets a 0.35 post-veil minimum, Dark a 0.20 maximum. The displayed
backdrop is always the sharp centre sample, so no spatial filter enters the
image.

**Extent / input / reservation.** The menu-bar layer surface is configured to
the 95 pt visual extent; the exclusive zone and committed buffer/input strip
remain the 28 pt bar. The shell declares a `veil` panel region with the full
extent and clears it when the veil is off. The veil sits below foreground and
menu popups; `visible_in_fullscreen(LAYER_TOP)` still hides it and leaves the
output direct-scanout eligible. The sharp element's commit follows the scene
cache's real-damage commit, so moving windows repaint the veil and settled
scenes do not. A region request is also committed on a damage-free frame
through `applyMenuBarSurfaceGeometry`, so a show/hide/material toggle cannot
leave stale metadata behind.

**Evidence.** `scripts/t200-menubar-veil.py --check` is in `make gates`
(`make menubar-veil-check`) and the report is
`docs/captures/t200-menubar-veil-model.json`: Light LUT within ±0.03 of every
reference sample and zero at 95; checker transmission identical at
2/4/8/16/32/64 pt for every sampled depth; live-window contrast 4.70
(Light/dark), 10.38 (Light/bright), 4.00 (Dark/bright after the readability
rule), 16.05 (Dark/dark). A headless `query material` with
`DRAGONFRUIT_GLASS_ROLLOUT=replacement` prints
`material veil mode=no-blur-veil extent=95.0 strip=39.0 probe=8.0
readability=0.200 alphas=…`.

`make menubar-veil-capture` also records the compositor's own render over a
black and a white X11 window directly under the bar:
`docs/captures/t200-menubar-veil-{light,dark,fallback}.png` plus the live
`query scale`/`query material` dumps. Light capture normalized LUT:
0.4510/0.4392/0.4039/0.3529/0.2392/0.0588/0.0078/0.0000 at
5/15/25/35/50/75/90/95 pt (max delta 0.004); the white window stays 255.
Dark shows the readability band through 39 pt and a smooth fade to the black
LUT; the fallback shows the milky plate.

**Checks.** `make gates` PASS (glass self-test, presets 165/0, menubar veil,
e2e including the new commit-latched `veil` conformance test, QML 83/83).
`cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` 423 passed;
`--test material_presets` 7 passed; `ctest -R '^tst_menubar$'` passed;
`make menubar-veil-capture` passed for Light/Dark/fallback.

**Deviations / remaining.** Dark is a named Dragonfruit baseline, not a
measured macOS profile. The nested Light/Dark/fallback captures are committed;
native DRM capture remains T208's rail. The readability correction fades over
8 pt past the 39 pt strip to avoid a hard seam. If a driver refuses both the
veil and glass shaders, the sharp path currently falls back to the raw scene;
a solid-element last resort is a follow-up. T201 owns the
`menu.showBackground`/`menu.autoHide` reveal policy; T202 owns menu-family
glass above this veil.
