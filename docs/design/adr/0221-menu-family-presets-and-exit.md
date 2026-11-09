# 0221 — Menu families share three glass presets and one measured exit

> Updated by [ADR 0243](0243-shell-material-focus-and-hover-corrections.md): see the current shell material, focus and hover policy.

- Status: Accepted
- Date: 2026-11-06
- Task: T202

## Context

Menus were the last popup family still painted from the generic
`material.popupOpacity` scrim. The reference evidence distinguishes an
attached File-style dropdown (square top, ~12.5 pt bottom corners, 10 pt blur,
~20 pt shadow) from detached context/status cards, and the light attached
alpha follows the Appearance slider. The shell renders menus into two trusted
overlay surfaces (`menubar-popup`, `dock-popup`), while Files and the desktop
own their context menus in app processes with no compositor backdrop.

## Decision

1. **Three presets, one mapping.** `menuAttached` is the menu-bar dropdown;
   `menuDetached` is the tray, Dock and app-owned context menu; `statusMenu` is
   Wi-Fi, Bluetooth, volume and battery. Submenus are detached cards even under
   an attached parent. QML and the compositor resolve the same generated
   presets through `Glass`/`material_preset`.
2. **Per-region shell metadata.** The shell declares each open menu panel with
   `df_layer_surface.set_panel_region`: the attached dropdown carries square
   top corners and the measured bottom radius, the nested panel and status card
   are all-rounded. The overlay surface input region is enabled only while the
   menu is logically open; the close tail passes clicks.
3. **Light slider is measured, other menus are relative.** Light `menuAttached`
   keeps the three measured File anchors. Light `menuDetached` and `statusMenu`
   preserve their own default fits and apply the attached curve relatively:
   alpha scales by the attached alpha ratio, tint takes the attached tint
   delta. Dark presets do not follow the light slider.
4. **Dark File is an adaptation with an open white row.** The published dark
   File fit is a Light File crop and is rejected. The attached dark baseline is
   calibrated against the valid dark mid/black context/status family and stays
   distinct from `menuDetached`; a controlled white-row recapture remains open.
5. **App-owned menus use the local fallback.** Files, the desktop and Select
   pass `materialBackdrop: "local"`; the resolver selects the opaque scheme
   plate and no sampling. Trusted shell menus pass `trusted` and sample.
6. **Measured exit, prompt open.** Menus close over 183 ms with a 0.95 terminal
   scale, shrinking toward their anchor. The logical close and input release
   happen immediately; a reopen/retarget before the timer expires cancels the
   pending unmap. Reduce Motion collapses the duration and terminal scale.
   The open animation stays prompt (160 ms) because the reference does not
   resolve it.
7. **Bluetooth joins the status row.** The previously hidden slot reads the
   existing bridge-host Bluetooth view, honours `menu.control.bluetooth`, and
   raises the same power/device/settings actions as the Control Center tile.
   Absence hides it; error/read-only states stay visible but inert.

## Consequences

- Menu content sizing stays dynamic; only the surface optics and exit motion
  change. The window chooser, folder stacks and Dock content/interaction are
  untouched.
- The compositor remains the only backdrop sampler; app-owned menus never
  paint a translucent slab over unfrosted content.
- `tst_menubar`, `tst_dock`, `tst_files_shell`, `tst_design_system`, the
  material-preset suites and `make menu-families-check` pin the mapping,
  slider response, fallback, motion and Bluetooth policy.

## References

- `docs/tasks/glass/plan/REFERENCE.md` — menu evidence and the dark File crop
  conflict.
- `design-system/tokens/tokens.json`, `design-system/Material.qml`,
  `design-system/components/{Popup,ContextMenu,MenuBarMenu}.qml`.
- `shell/menubar/{MenuBar,BluetoothMenu}.qml`,
  `shell/src/{shellcontroller,shellprotocol}.cpp`.
- ADR [0214](0214-glass-role-presets-and-resolution.md),
  [0205](0205-popup-family-material-fallback.md),
  [0219](0219-menubar-no-blur-veil.md).
