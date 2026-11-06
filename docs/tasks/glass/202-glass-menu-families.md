---
verify: make gates
---
# T202 — Apply glass to dropdowns, context menus, and status menus

## Goal

Give existing menus the measured attached, detached and status appearances with consistent fallback and unchanged menu action routing.

## Context (read first)

- Depends on T194–T197 and T200–T201. Read the [menu evidence and conflicts](plan/REFERENCE.md).
- `design-system/components/{MenuBarMenu,ContextMenu,Popup,Popover}.qml`, `shell/menubar/{MenuBar,WifiMenu,VolumeMenu,BatteryMenu}.qml`, Dock popup components and `shell/src/shellcontroller.cpp::updatePopupGeometry`.
- `apps/files/{DesktopSurface,FilesShell}.qml` host app-owned context menus. `compositor/src/window/menu.rs` owns the server-side window menu.

## Scope

- [ ] Apply shared attached/detached/status presets to system and exported app menus, submenus, tray/status menus, and the Dock context menu. Keep window chooser and folder stack content and interaction. Route Files, desktop and SSD context menus to the same style via their existing ownership, using local fallback where no trusted backdrop is available.
- [ ] Match corner treatment, rim, shadow and anchor geometry. Animate the departing shell menu visually toward its anchor, close its logical interaction immediately, and handle reopen/retarget before the exit finishes. Honour Reduce Motion.
- [ ] Apply the measured light attached-menu slider curve. Use a documented relative adjustment for other light menu presets, preserving their default fits. Keep dark attached and dark context/status presets separate. Calibrate Dark File against the valid mid/black samples and keep the white row open for recapture.
- [ ] Preserve Wi-Fi, Battery, Volume and StatusNotifier interactions. Connect the currently disabled Bluetooth status slot to the existing adapter and Menu Bar setting if the adapter supports it; share the existing Bluetooth action policy and respect its absent/read-only state.

## Out of scope

- An AirDrop service or new integration for unavailable status services. First-party toplevel backdrop protocol (T206). New menu-model APIs.

## Design notes

Target Light File α ~0.55 at default, ~10 pt blur, ~20 pt shadow extent, bottom corners ~12.5 pt, and ~183 ms close with 0.95 terminal scale. Keep content sizing dynamic. Treat menu open as prompt and interruptible, since the source does not resolve timing. Use all-rounded corners for detached menus. Treat the provisional Dark File preset as an adaptation and verify foreground contrast on light, dark and patterned backdrops. A decorative shadow or exit animation must leave no click interception. Preserve compositor hit testing, keyboard grabs and focus restoration. Keep QML local paint deterministic when app-owned surfaces lack sampling.

## Done when

- [ ] Capture Light/Dark attached, desktop/Files/Dock context, representative status menus and slider endpoints. Match light File α within ±0.03, blur within ±1 pt and shadow extent within ±3 pt. Report the Dark File crop decision and fallback behavior.
- [ ] Exercise menu broker actions and accelerators, nested submenu traversal, checkmarks, disabled items, focus restore, repeated open/close, edge clipping, Reduce Motion and adapter absence. `tst_menubar`, `tst_dock`, relevant Files/design-system tests and `make gates` pass. Fill in Hand-off.

## Hand-off

_(Implementation session records preset mapping, status coverage and ownership boundaries.)_
