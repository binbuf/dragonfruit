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

- [x] Apply shared attached/detached/status presets to system and exported app menus, submenus, tray/status menus, and the Dock context menu. Keep window chooser and folder stack content and interaction. Route Files, desktop and SSD context menus to the same style via their existing ownership, using local fallback where no trusted backdrop is available.
- [x] Match corner treatment, rim, shadow and anchor geometry. Animate the departing shell menu visually toward its anchor, close its logical interaction immediately, and handle reopen/retarget before the exit finishes. Honour Reduce Motion.
- [x] Apply the measured light attached-menu slider curve. Use a documented relative adjustment for other light menu presets, preserving their default fits. Keep dark attached and dark context/status presets separate. Calibrate Dark File against the valid mid/black samples and keep the white row open for recapture.
- [x] Preserve Wi-Fi, Battery, Volume and StatusNotifier interactions. Connect the currently disabled Bluetooth status slot to the existing adapter and Menu Bar setting if the adapter supports it; share the existing Bluetooth action policy and respect its absent/read-only state.

## Out of scope

- An AirDrop service or new integration for unavailable status services. First-party toplevel backdrop protocol (T206). New menu-model APIs.

## Design notes

Target Light File α ~0.55 at default, ~10 pt blur, ~20 pt shadow extent, bottom corners ~12.5 pt, and ~183 ms close with 0.95 terminal scale. Keep content sizing dynamic. Treat menu open as prompt and interruptible, since the source does not resolve timing. Use all-rounded corners for detached menus. Treat the provisional Dark File preset as an adaptation and verify foreground contrast on light, dark and patterned backdrops. A decorative shadow or exit animation must leave no click interception. Preserve compositor hit testing, keyboard grabs and focus restoration. Keep QML local paint deterministic when app-owned surfaces lack sampling.

## Done when

- [x] Capture Light/Dark attached, desktop/Files/Dock context, representative status menus and slider endpoints. Match light File α within ±0.03, blur within ±1 pt and shadow extent within ±3 pt. Report the Dark File crop decision and fallback behavior.
- [x] Exercise menu broker actions and accelerators, nested submenu traversal, checkmarks, disabled items, focus restore, repeated open/close, edge clipping, Reduce Motion and adapter absence. `tst_menubar`, `tst_dock`, relevant Files/design-system tests and `make gates` pass. Fill in Hand-off.

## Hand-off

**Status: done.**

**Preset mapping.** `menuAttached` = menu-bar dropdown; `menuDetached` =
tray context menu, Dock context menu and app-owned context menus; `statusMenu`
= Wi-Fi, Bluetooth, volume and battery popovers. Submenus are detached cards
under an attached parent. `Material.menuFill()`/`menuResolution()` is the one
QML complement; the compositor resolves the same presets in
`material_preset`. Trusted shell surfaces sample; `ContextMenu` defaults to
`materialBackdrop: "local"` so Files/desktop/Select paint the opaque scheme
plate. The SSD window menu resolves `menuDetached` with the same local
fallback and keeps its geometry/hit-testing.

**Geometry and motion.** Attached bottom radius 12.5 pt, square top; detached
and status all-rounded; shadow extent 20 pt. Menu close is 183 ms with 0.95
terminal scale toward the anchor. The logical close and input release are
immediate; the shell clears the decorative panel, keeps the overlay mapped for
a 210 ms tail, and cancels the unmap on reopen/retarget. Reduce Motion disables
the Behaviors and uses terminal scale 1. The shell declares per-region material
metadata on `menubar-popup`/`dock-popup` with the attached/detached/status
preset and per-corner radii.

**Slider and dark calibration.** Light `menuAttached` keeps the measured
anchors. Light `menuDetached`/`statusMenu` use a documented relative adjustment
(alpha ratio, tint delta) and preserve their default fits; dark presets do not
follow the light slider. Dark File is anchored to the valid dark mid/black
context/status family (0.66/0.26) and mixed 25% toward a deeper adapted baseline
(resolved 0.6425/0.2775), distinct from dark detached. The contaminated white
crop is rejected and the white row stays open for a controlled recapture.

**Status coverage.** Wi-Fi, volume, battery and StatusNotifier interactions are
unchanged. Bluetooth now reads the existing bridge-host adapter, honours
`menu.control.bluetooth`, and raises the same power/known-device/settings
actions as the Control Center tile; absent hides the slot, error/read-only is
visible but inert.

**Evidence.** `make menu-families-capture` writes
`docs/captures/t202-attached-{light,dark,light-low,light-high}.png`,
`t202-status-{wifi,bluetooth}.png`, `t202-context-{dock,desktop,files}.png`,
the matching query text, and `t202-capture.json`. Model evidence:
`docs/captures/t202-menu-families.{json,md}`. Light File alpha 0.552 vs target
0.55 (+0.002), blur 10 pt (+0), shadow 20 pt (+0), bottom corner 12.5 pt. Live
slider endpoints move in the measured direction (interior means 152.9 / 147.1 /
141.1 for low/default/high over the checker).

**Checks run (final tree).**
- `python3 scripts/gen-tokens.py --check` — current.
- `python3 scripts/material_presets.py --check` — pass.
- `python3 tests/glass/test_material_presets.py` — 173 passed.
- `python3 scripts/t202-menu-families.py --check --write` — pass.
- `cargo test -p dragonfruit-compositor --lib material_preset` — 14 passed.
- `cargo test -p dragonfruit-compositor --test material_presets` — 8 passed.
- `cargo test -p dragonfruit-compositor --test reduced_motion_sweep` — 3 passed.
- `ctest --test-dir build -R '^tst_design_system$|^tst_menubar$|^tst_dock$|^tst_files_shell$'` — pass.
- `make qml-test` — 83/83 passed.
- `make menu-families-capture` — 9/9 stills written.
- `make gates` — PASS.

**Deviations / remaining work.**
- The Files-window capture uses the demo client path (`make demo` with
  `DF_FILES_START_MENU=background`) rather than the `dev-full --shell` rail;
  its wallpaper is the shipped default rather than the checker.
- The dark attached white row is intentionally open pending a controlled
  recapture; only the mid/black rows are calibrated.
- Menu open timing remains unresolved by the source; 160 ms is the prompt,
  interruptible baseline.
- Docs: ADR `docs/design/adr/0221-menu-family-presets-and-exit.md`,
  `docs/design/10-design-system.md`, `04-shell.md`, `07-system-integration.md`,
  `docs/settings-keys.md`. Reusable notes:
  `docs/tasks/glass/plan/progress/T202.md`.
