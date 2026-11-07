---
verify: make gates
---
# T205 — Calibrate notification banner, stack, and lower panel

## Goal

Give transient banners and the open Notification Center three distinct visual treatments while using the service-backed history from T204.

## Context (read first)

- Depends on T194–T197 and T204. Read [notification evidence and caveats](plan/REFERENCE.md#what-to-carry-forward).
- `shell/notifications/{NotificationBanner,NotificationCenter}.qml` and the shell overlay/material routes.
- Compare sibling `captures/coverage/{notification_banner,notification_center,notification_center_with_banner,dark_notification_center}__*` and `docs/coverage_measurements.json`.

## Scope

- [ ] Apply a distinct banner material and calibrate it over the neutral backgrounds. Preserve urgency, actions, text contrast and sizing.
- [ ] Style the Center with a notification stack and separately toned lower panel. Register geometry from the actual captures and content layout. Validate banner and Center co-visibility without double-frosting or sampling a panel's own content.
- [ ] Apply the measured Light and Dark stack fits and readable Dark banner/lower-panel variants. Route accessibility overrides and the no-sampling fallback through the shared policy. Give the existing history a deliberate empty and scrollable state.

## Out of scope

- Changing service policy, notification storage or widget behavior. Claiming an exact fit for the underconstrained lower panel or uncaptured Dark regions.

## Design notes

Use Light effective α/tint 0.549/0.976 banner, 0.551/0.706 stack, 0.407/0.852 lower, and Dark 0.507/0.550 stack. Prefer separately masked measured regions over the coverage detector's whole-width bounding box. Record residuals of the chosen lower-panel curve or preset. Bound region count and keep panels lightweight and readable when tiers degrade. Use the existing Focus, expiry and action paths.

## Done when

- [ ] Light/Dark captures distinguish all three materials in empty, populated and concurrent banner/Center cases, with sampled, software and accessibility variants. Record independent fit errors and label underconstrained cases.
- [ ] `tst_notifications` and `make gates` pass. Verify transitions do not leave stale material, input regions, focus or active actions. Fill in Hand-off.

## Hand-off

**Landed.** The banner, Center stack and Center lower panel are three
independently keyed materials, resolved through the shared T195/T196 policy and
drawn by the compositor's replacement flat-band path.

- Measured: banner Light 0.549/0.976; stack Light 0.551/0.706, Dark
  0.507/0.550; lower Light 0.407/0.852. Adapted (underconstrained): banner Dark
  0.56/0.30; lower Dark 0.40/0.30 (toned below the Dark stack and distinct from
  the Dark banner). Both Dark adaptations are labelled `adapted` in the tokens
  and `underconstrained` in every capture row.
- Regions: `notification` declares one `banner` region sized to the visible
  card (96 pt, 132 pt with actions); `notification-center` declares disjoint
  `stack` and `lower` regions separated by the 8 pt panel gap. An active banner
  sets `bannerInset`, moving the stack below the card so the surfaces are
  co-visible without overlap or double-frosting. The lists are commit-latched
  with each image, re-declared on history/layout changes, and cleared before
  unmapping.
- Fallbacks: QML is transparent on the sampled path; solid, software and
  Reduce Transparency paint the same resolved tint/alpha; Increase Contrast
  raises the resolved rim; legacy keeps `Material.popupFill()`. The QML shadow
  is disabled only on the sampled path so it cannot darken the opaque
  compositor plate.
- History states: deliberate empty/unavailable text in the stack, a lower-panel
  count/summary, and a visible scrollbar plus "Scroll for more" when rows
  overflow.
- Capture seam: `DF_NOTIFY_FIXTURE=empty|populated|concurrent|scrollable|banner-empty`,
  `DF_NOTIFY_CENTER_FIXTURE=open`, `FORCE_MATERIAL_PATH=gpu|software`.

**Residuals.** `docs/captures/t205-notification-materials.json` records 18
cases. Neutral sampled fit errors: banner 0.0006–0.0014, stack 0.0008–0.0019,
lower 0.0013–0.0016; grid: banner 0.0364–0.0372, stack 0.0003–0.0019, lower
0.0026–0.0029; accessibility/software up to 0.0141. Dark banner/lower rows are
marked underconstrained. The nested host produced 2880x1800; the scorer uses
the live `query scale` rects.

**Checks.** `python3 scripts/gen-tokens.py --check` current;
`python3 scripts/material_presets.py --check` pass;
`python3 tests/glass/test_material_presets.py` 201 passed;
`python3 scripts/t205-notification-materials.py --check` PASS;
`cargo test -p dragonfruit-compositor --lib material_preset` 16 passed;
`cargo test -p dragonfruit-compositor --test material_presets` 9 passed;
`cargo test -p dragonfruit-notifications` 24 unit + 12 session-bus passed;
`ctest -R '^tst_notifications$|^tst_notificationcenter$|^tst_notificationmodel$'`
3/3 passed; `make notification-materials-capture` 18/18 + scorer PASS;
`make gates` PASS (e2e + 84/84 ctest).

**Remaining / follow-ups.** Dark banner/lower panel need a controlled
independent recapture before claiming an exact fit. The lower panel is a
summary/hint footer, not a widget/calendar; a future widget panel should keep
the `lower` key. The sampled path has no QML drop shadow (the compositor flat
band owns the plate); if a measured notification shadow is added, move it into
the compositor. Keyboard Tab still traverses rows only (T204 follow-up).

**Docs changed.** `docs/design/adr/0224-notification-banner-stack-lower-panel.md`,
`docs/design/10-design-system.md`, `docs/design/02-compositor.md`,
`docs/design/04-shell.md`, `docs/private-protocols.md`,
`docs/tasks/glass/plan/progress/T205.md`.
