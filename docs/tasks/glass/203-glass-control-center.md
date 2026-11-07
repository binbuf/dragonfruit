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

Implemented T203 on the replacement rail. The Control Center now declares an
outer plate (`controlCenterOuter`), one capsule per visible module
(`controlCenterModule`) and an expanded Wi-Fi/Bluetooth list
(`controlCenterExpanded`) as independently keyed material regions on the
`control-center` layer surface. The shell clips the module/list regions to the
scroll viewport, re-declares them when the QML layout or scroll changes, and
clears them on hide. The compositor resolves the same three presets and uses
the measured flat-band + 0.30 rim + 6 pt/24 pt capsule-end lens model. The
module preset carries the measured Light 0.391/0.317 effective fit directly
because the flat-band output is opaque; the outer plate stays visible around
the capsules. Dark module is adapted 0.65/0.44; dark expanded list is 0.96/0.13;
light expanded list is 0.94/0.96. `MAX_PANEL_REGIONS` is 32 (outer + 19 modules
+ two lists). QML hides fills on the sampled path and paints the resolved
tint/alpha on solid/software/Reduce Transparency. Increase Contrast raises the
resolved rim. The existing actions, service availability, live theme/Focus
updates, focus order and full-panel input region are retained. New expandable
Wi-Fi/Bluetooth lists use accessible disclosure links; Wi-Fi rows are read-only
and Bluetooth rows keep the existing connect/disconnect action.

Region/preset/adapter mapping is in
`docs/tasks/glass/plan/progress/T203.md` and the generated
`docs/captures/t203-control-center.md`. Region mapping: `outer` →
`controlCenterOuter`; `module-wifi` … `module-clipboard` →
`controlCenterModule`; `expanded-wifi` / `expanded-bluetooth` →
`controlCenterExpanded`. Adapter mapping: `wifi` → Wi-Fi, `bluetooth` →
Bluetooth, `storage` → Storage, `audio` → Sound, `brightness` → Display,
`input` → Keyboard, notification Focus → Focus, settingsd scheme → Dark Mode,
shell projections → Mission Control / Lock Screen / Menu Bar, `updates` →
Software Update, `accounts` → Users, `printers` → Printers, `privacy` →
Privacy, `accessibility` → Accessibility, `vpn` → VPN, clipboard → Clipboard.
Captures: 14 cases
(`collapsed/expanded` × `light/dark` × `neutral/grid`, plus dark Increase
Contrast, dark Reduce Transparency and an absent-adapter case) in
`docs/captures/t203-*.png` with matching `query scale` dumps. The scorer reports
outer error 0.006–0.023 and nested error 0.005–0.018; dark expanded list 0.145;
grid attenuation 0.014 → 0.00002–0.0025; expanded-region movement > 0.05 with
no stale collapsed region. ADR 0222 records the decision.

Checks: `tst_controlcenter` and `tst_controlcenterpolicy` pass; `make gates`
passes (83/83 QML tests). `make control-center-materials-capture` completes
14/14 captures and its scorer passes. No blockers. Remaining work is the
follow-ups in `progress/T203.md`: Wi-Fi join is still read-only, the Light
expanded-list and dark module values are adapted pending controlled reference
captures, and a future module count above 32 needs grouping or a bound update.
