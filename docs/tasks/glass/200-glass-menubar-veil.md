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

- [ ] Render the veil against the compositor's sharp live scene, with the Light depth LUT ending by 95 logical pt. Calibrate 39 pt as the reference strip height and preserve Dragonfruit's existing layout needs. Keep the full veil visual-only beyond the strip.
- [ ] Calibrate local luminance adaptation using the measured neutrals/checkers and foreground readability over moving windows. Implement a documented Dark baseline and accessibility/no-sampling variants. Keep fallback colours scheme-appropriate.
- [ ] Replace the old QML-plus-compositor wash on the new rail. Use an explicit no-blur mode and a veil diagnostic. Preserve output damage, reservation, rendering order and fullscreen scanout policy.

## Out of scope

- Menu panel glass (T202), visibility setting and reveal behavior (T201), and matching an unmeasured macOS Dark profile.

## Design notes

Check alpha at the actual sample depth. The reference 0.451 over black is at y=5 pt; the text-free black LUT drops to 0.008 at 90 pt and zero at 95 pt. Foreground decisions may use a bounded local/strip luminance measure, with stable contrast over mixed content. Ensure the adaptation does not introduce spatial filtering into the displayed backdrop. The veil sits below foreground and menu popups. A disabled or hidden veil must contribute no visual element or sampled work.

## Done when

- [ ] Light black LUT α within ±0.03 at the reference samples, zero by 95 pt, and no checker-size-dependent loss at a fixed depth within fixture tolerance. Show live-window response, readable Dark/fallback output, and no additional blur pass for the veil.
- [ ] Automated tests cover extent, output isolation, input/reservation and fullscreen suppression. `tst_menubar` and `make gates` pass. Fill in Hand-off.

## Hand-off

_(Implementation session records the veil mode, luminance/readability policy and fixture results.)_
