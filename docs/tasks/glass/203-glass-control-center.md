---
verify: make gates
---
# T203 — Distinguish Control Center panel and module materials

## Goal

Give the existing Control Center an outer plate, distinct module surfaces and a dark expanded list calibrated to the observed composite.

## Context (read first)

- Depends on T194–T197. Read the [Control Center notes](plan/REFERENCE.md#what-to-carry-forward).
- `shell/control-center/ControlCenter.qml`, `shell/src/controlcenterpolicy.*` and `shell/src/shellcontroller.cpp::{applyControlCenterData,updateControlCenterGeometry}`.
- In the sibling checkout, compare `control_center_panel__*`, `control_center_wifi__*`, `control_center_bluetooth__*` and `dark_control_center__*` with their entries in `docs/coverage_measurements.json`.

## Scope

- [ ] Apply the distinct outer, module and expanded-list treatments. Tune the Light/Dark outer plate and stronger capsule-end optics using the grid reference. Compare the final composite for the inner module instead of layering the measured effective alpha over the outer panel.
- [ ] Retarget the visible glass region with expanded Wi-Fi/Bluetooth and existing modules. Preserve focus order, anchors and scroll/input bounds in smaller displays.
- [ ] Wire the shared accessibility variants and solid fallback while retaining existing Control Center actions, service availability behavior and live theme/Focus updates.

## Out of scope

- Adding AirDrop, Now Playing, Screen Mirroring, widgets or other services simply because the source includes those panels.

## Design notes

Use the later Light outer fit 0.265/0.373, Dark 0.489/0.677, and the Wi-Fi effective Light fit 0.391/0.317. Use near-background-independent luma ~0.13 for the expanded Bluetooth reference, with appropriate contrast. The wider panel and nested end caustic should not add independent full-output captures. Reduce Transparency is a near-opaque dark reference here. Fit unmeasured combinations as readable Dragonfruit variants.

## Done when

- [ ] Capture collapsed and expanded states, light/dark/contrast/reduce-transparency and absent adapters with neutral and grid backdrops. Report outer and nested error separately, and show the visible material moves with the panel without stale regions.
- [ ] `tst_controlcenter`, `tst_controlcenterpolicy`, and `make gates` pass. Fill in Hand-off with region, preset and adapter mapping.

## Hand-off

_(Implementation session records fixture values and measured/adapted presets.)_
