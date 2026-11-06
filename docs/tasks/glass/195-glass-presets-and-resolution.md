---
verify: make check-tokens && make check-design-tokens && make gates
---
# T195 — Define role presets and material resolution

## Goal

Give the shell and compositor one generated material vocabulary with explicit scheme, accessibility and fallback behavior.

## Context (read first)

- Depends on T194. Read the [evidence decisions](plan/REFERENCE.md#how-to-resolve-conflicting-evidence).
- `design-system/tokens/tokens.json`, `scripts/gen-tokens.py`, `design-system/{Theme,Material}.qml`, and `compositor/src/design_tokens.rs`.
- `compositor/src/window/{backdrop,degrade}.rs`, `shell/src/materialpolicy.*`, and `docs/design/adr/0203-role-keyed-material-floor.md`.

## Scope

- [ ] Extend the existing generated tokens/resolver for Dock, veil, attached and detached menus, status menu, tooltip, Control Center outer/module/expanded list, notification banner/stack/lower panel and first-party window chrome. Record provenance and confidence for each calibrated value or local adaptation.
- [ ] Resolve scheme, supported slider response, accessibility, and backend capability/tier consistently. Make rendering mode explicit (sampled glass, no-blur veil, solid or software fallback) so a veil cannot be mistaken for unsupported blur.
- [ ] Introduce one internal rollout switch for the end-to-end replacement, defaulting to the existing appearance until T208. Make nested and native fixtures select either look reproducibly.

## Out of scope

- Settings writes and Wayland policy transport (T196), compositor draw changes (T197–T200), and consumer migration (T198–T206).

## Design notes

Keep optical presets separate from existing wire tier roles and values. Preserve the Dock's persistent material floor where sampling is allowed. An explicit Reduce Transparency override takes precedence over the slider and disables sampling/lensing, with scheme-appropriate opaque fill. Increase Contrast strengthens edge/foreground contrast even when Reduce Transparency is active. Reduce Motion is orthogonal. Represent neutral alpha/tint and perceptual colour mapping as distinct calibration data. Convert colour inputs explicitly and avoid applying both fits as full-strength overlays. Encode the three measured light File anchors, with piecewise interpolation through all three. Dark and unmeasured roles take named baselines.

## Done when

- [ ] Generated outputs are reproducible. Automated coverage checks variant precedence, range and finite-value validation, light slider anchors, unchanged unmeasured slider roles, distinct dark attached/detached presets, and safe unsupported paths.
- [ ] `make check-tokens`, `make check-design-tokens`, and `make gates` pass. Document resolution in `docs/design/10-design-system.md` and fill in Hand-off.

## Hand-off

_(Implementation session records schema, policy decisions and which presets are calibrated.)_
