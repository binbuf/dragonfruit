# Liquid Glass progress

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **T194 — reference evidence and baseline**: `docs/captures/glass-reference-manifest.json` pins revision; `scripts/t194-reference-manifest.py` writes/verifies the manifest. Root
- **T195 — role presets and material resolution**: `design-system/tokens/tokens.json` gained the top-level `glass` group:; 13 optical presets: `dock`, `menuBarVeil`, `menuAttached`,
- **T196 — live Appearance/Accessibility settings and the compositor bridge**: **settingsd schema revision 18** (`services/settingsd/src/schema.rs`):; `appearance.transparency` — double, 0.0–1.0, default `0.5255`,

## Task notes (maintained by symphony — do not edit)

- [T194](progress/T194.md) — `docs/captures/glass-reference-manifest.json` pins revision
- [T195](progress/T195.md) — `design-system/tokens/tokens.json` gained the top-level `glass` group:
- [T196](progress/T196.md) — **settingsd schema revision 18** (`services/settingsd/src/schema.rs`):
<!-- symphony:digest:end -->

Implementation notes go in `progress/TNN.md`. Record the actual test commands, evidence paths, measured deltas, and any open rows with their rerun steps.
