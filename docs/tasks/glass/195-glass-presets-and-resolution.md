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

- [x] Extend the existing generated tokens/resolver for Dock, veil, attached and detached menus, status menu, tooltip, Control Center outer/module/expanded list, notification banner/stack/lower panel and first-party window chrome. Record provenance and confidence for each calibrated value or local adaptation.
- [x] Resolve scheme, supported slider response, accessibility, and backend capability/tier consistently. Make rendering mode explicit (sampled glass, no-blur veil, solid or software fallback) so a veil cannot be mistaken for unsupported blur.
- [x] Introduce one internal rollout switch for the end-to-end replacement, defaulting to the existing appearance until T208. Make nested and native fixtures select either look reproducibly.

## Out of scope

- Settings writes and Wayland policy transport (T196), compositor draw changes (T197–T200), and consumer migration (T198–T206).

## Design notes

Keep optical presets separate from existing wire tier roles and values. Preserve the Dock's persistent material floor where sampling is allowed. An explicit Reduce Transparency override takes precedence over the slider and disables sampling/lensing, with scheme-appropriate opaque fill. Increase Contrast strengthens edge/foreground contrast even when Reduce Transparency is active. Reduce Motion is orthogonal. Represent neutral alpha/tint and perceptual colour mapping as distinct calibration data. Convert colour inputs explicitly and avoid applying both fits as full-strength overlays. Encode the three measured light File anchors, with piecewise interpolation through all three. Dark and unmeasured roles take named baselines.

## Done when

- [x] Generated outputs are reproducible. Automated coverage checks variant precedence, range and finite-value validation, light slider anchors, unchanged unmeasured slider roles, distinct dark attached/detached presets, and safe unsupported paths.
- [x] `make check-tokens`, `make check-design-tokens`, and `make gates` pass. Document resolution in `docs/design/10-design-system.md` and fill in Hand-off.

## Hand-off

**Landed.** The `glass` token group now defines 13 optical presets (Dock,
menu-bar veil, attached/detached/status menus, tooltip, Control Center
outer/module/expanded list, notification banner/stack/lower panel, first-party
window chrome). Each scheme records neutral alpha/tint, a separate perceptual
fit and weight, blur, edge/foreground contrast, sRGB colour space, provenance,
and confidence. Measured: Dock (light/dark), light attached-menu slider
(0/0.5255/1 anchors), light detached/status menus, Control Center light/dark
outer and light module, notification banner/stack/lower panel light and dark
stack. Adapted/baseline: dark attached menu, dark Control Center
module/expanded list, tooltip, dark banner/lower panel, dark veil, window
chrome's near-opaque composite. The published contaminated dark File fit is not
used.

`scripts/gen-tokens.py` validates the schema (finite numbers, ranges, required
presets, provenance/confidence, three anchor positions, midpoint equality,
distinct dark attached/detached) and generates `Theme.glass` plus
`design_tokens::glass`. Resolvers: `scripts/material_presets.py` (reference and
CLI), `design-system/Glass.qml`, and `compositor/src/material_preset.rs`.
Resolution precedence is Reduce Transparency → rollout look → light
`menuAttached` slider → Increase Contrast → backend/tier. Render modes are
explicit: `sampled-glass`, `no-blur-veil`, `solid`, `software-fallback`. The
veil is always `no-blur-veil`; unsupported sampled paths use an opaque plate
with a named reason; persistent chrome keeps its sampled floor. Reduce Motion is
orthogonal.

One rollout switch, `DRAGONFRUIT_GLASS_ROLLOUT=legacy|replacement`, defaults to
`legacy` until T208. The shell controller applies it to `Glass.rollout` and logs
it; the compositor logs `glass rollout=<mode>` at startup and the resolver
exposes `Rollout::from_env()`. Unset/unknown is always legacy, so nested and
native fixtures can select either look reproducibly.

**Checks.** `make check-tokens` PASS, `make check-design-tokens` PASS,
`make glass-presets-check` PASS (schema + 151 checks), `cargo test -p
dragonfruit-compositor --lib material_preset` 13/13, `cargo test -p
dragonfruit-compositor --test material_presets` 7/7, design-system QML tests
10/10, `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings`
PASS. Final `make gates` PASS: glass self-test 120/120, preset check 151/151,
e2e including `material_presets` 7/7, QML 83/83 tests.

**Docs.** `docs/design/10-design-system.md` and ADR
[0214](adr/0214-glass-role-presets-and-resolution.md) record the vocabulary,
precedence, render modes, and switch. Reusable details and follow-ups are in
`docs/tasks/glass/plan/progress/T195.md`.

**Remaining / deviations.** No blockers. Consumer migration remains T196–T206;
T196 must feed the same resolver inputs rather than adding another curve.
Controlled dark attached-menu and dark banner/lower-panel recaptures remain
OPEN and are labelled `adapted`/`baseline` until then. T208 owns flipping the
default. `make check-i18n` (not in `gates`) reports an unrelated pre-existing
`Monitor %1` template move in `ShellSurfaceBuffer`/`shellprotocol`; T195 does
not touch either file.
