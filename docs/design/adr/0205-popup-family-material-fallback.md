# 0205 — The popup family applies Popup/Drawer through the one material policy

## Status

accepted

## Context

T189a made `df_output.material_role_state` (v4) carry an effective material
per `MaterialRole`, and T189b applied `Chrome`/`Dock` through the shell's
`MaterialPolicy`. The popup family still assumed the sampled backdrop: menus,
Dock popovers, Control Center, notification banners, the OSD, and the
Applications drawer painted the translucent `material.popupOpacity` scrim even
when the compositor reported no blur. That left a flat translucent slab, and
the transient roles were not using the role-keyed state at all.

## Decision

- `ShellController` applies `Popup` to the menu-bar scene (dropdowns, submenus,
  status menus, tray context menu), the Dock scene (context menu, window
  chooser, stack/overflow/trash/app-picker popovers), Control Center,
  notification banners, and the OSD. It applies `Drawer` to the Applications
  drawer. The same `MaterialPolicy` object owns the decision; QML never
  re-runs the degrade ladder.
- The Dock and menu-bar scenes expose `popupMaterial{BlurActive,Tier,Fallback}`
  alongside their persistent `material*` properties. Their popovers bind to
  the popup triple; the shell seeds it before the first commit and updates it
  on every role event.
- `design-system/Material.qml` is the one QML complement:
  `popupFill()`/`drawerFill()` choose the sampled-glass scrim when blur is
  active and the milky `chromeFallbackFill` otherwise. `fallbackReason()`
  records the explicit wire reason or the tier reason (`minimal`/`blur-off`)
  on every surface's `materialFallbackReason`.
- `material.popupFallbackOpacity` is added to both schemes (0.92 light / 0.90
  dark). Existing token values are unchanged; the fallback fill reuses the
  existing Dock/menu-bar milky neutral, so there is no second hardcoded tone.
- Transient roles keep the compositor's tier: they may reach `Minimal`; the
  shell only turns the sampled pass off and selects the milky plate.

## Consequences

- Every blur-assuming popup/overlay has a deliberate milky fallback and a
  recorded reason, and a transient overlay cannot pull the persistent chrome
  down.
- Tests cover the Dock popover family, menu popovers, Control Center,
  notifications, the OSD, and the Applications drawer in light and dark for
  Full/Reduced/Minimal (including `software`/`minimal` reasons).
- Forced-tier captures cover the same overlay family; the capture trace records
  the compositor's `query degrade` and the shell's applied per-role
  tier/blur/fallback lines.
- T189d only changes the hysteresis window; the role application and fallback
  complement are stable.

## References

- `docs/tasks/0.1.1/reference/189-t-21.2c-persistent-chrome-material-floor.md`
- ADR [0203](0203-role-keyed-material-floor.md), ADR
  [0204](0204-shell-role-material-application-policy.md)
- `design-system/Material.qml`, `shell/src/materialpolicy.*`,
  `shell/src/shellcontroller.cpp`
- `scripts/capture-t21-material-tiers.sh`,
  `docs/captures/t21-material-tiers.txt`
