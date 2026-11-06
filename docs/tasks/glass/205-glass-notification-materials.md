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

_(Implementation session records measured versus adapted variants, region behavior, and residuals.)_
