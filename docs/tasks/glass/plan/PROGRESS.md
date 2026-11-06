# Liquid Glass progress

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **T194 — reference evidence and baseline**: `docs/captures/glass-reference-manifest.json` pins revision; `scripts/t194-reference-manifest.py` writes/verifies the manifest. Root
- **T195 — role presets and material resolution**: `design-system/tokens/tokens.json` gained the top-level `glass` group:; 13 optical presets: `dock`, `menuBarVeil`, `menuAttached`,
- **T196 — live Appearance/Accessibility settings and the compositor bridge**: **settingsd schema revision 18** (`services/settingsd/src/schema.rs`):; `appearance.transparency` — double, 0.0–1.0, default `0.5255`,
- **T197 — compositor material geometry and composition**: Protocol v3 additive `df_layer_surface` requests:; `set_panel_region(key, preset, x, y, w, h, tl, tr, br, bl, overscan)`.
- **T198 — Dock optics fit and capture**: Dock preset tokens (`design-system/tokens/tokens.json`, both schemes):; `compositor/src/window/dock_optics.rs`:
- **T199 — Dock magnification, envelope, and tooltip**: **Spatial raised cosine.** `shell/dock/Dock.qml` now evaluates; **Peak anchors.** `component.dock.magnifyPeak = 1.217` (default
- **T200 — Menu-bar no-blur veil**: **Measured Light depth LUT.** `design-system/tokens/tokens.json` now stores; **Pure veil model.** New `compositor/src/window/menubar_veil.rs` owns
- **T201 — Menu-bar visibility settings**: **`menu.showBackground` is one switch for both rails.** The QML fill and the; **`menu.autoHide` is compositor-owned per output.** Additive

## Task notes (maintained by symphony — do not edit)

- [T194](progress/T194.md) — `docs/captures/glass-reference-manifest.json` pins revision
- [T195](progress/T195.md) — `design-system/tokens/tokens.json` gained the top-level `glass` group:
- [T196](progress/T196.md) — **settingsd schema revision 18** (`services/settingsd/src/schema.rs`):
- [T197](progress/T197.md) — Protocol v3 additive `df_layer_surface` requests:
- [T198](progress/T198.md) — Dock preset tokens (`design-system/tokens/tokens.json`, both schemes):
- [T199](progress/T199.md) — **Spatial raised cosine.** `shell/dock/Dock.qml` now evaluates
- [T200](progress/T200.md) — **Measured Light depth LUT.** `design-system/tokens/tokens.json` now stores
- [T201](progress/T201.md) — **`menu.showBackground` is one switch for both rails.** The QML fill and the
<!-- symphony:digest:end -->

Implementation notes go in `progress/TNN.md`. Record the actual test commands, evidence paths, measured deltas, and any open rows with their rerun steps.
