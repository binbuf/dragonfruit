---
verify: make gates
---
# T196 — Connect Appearance and Accessibility to the compositor

## Goal

Make the glass preference and accessibility overrides durable and live through Dragonfruit's existing Settings workflow.

## Context (read first)

- Depends on T195. Read `docs/settings-keys.md`, `docs/ipc-versioning.md`, and `docs/design/adr/0034-compositor-policy-via-shell-bridge.md`.
- `services/settingsd/src/schema.rs`, `libs/settings-client/settingsclient.*`, `apps/settings/{AppearancePane,AccessibilityPane,SettingsShell}.qml`, `apps/settings/SettingsBridge.cpp`, and `shell/src/{themebinding,compositorpolicy,shellcontroller,shellprotocol}.cpp`.
- `protocols/dragonfruit-toplevel.xml`, `compositor/src/shell/mod.rs`, and `compositor/src/state.rs::set_motion_policy` show the trusted settings bridge.

## Scope

- [ ] Add `appearance.transparency` (double 0–1, default 0.5255, clearer toward 1), `accessibility.reduceTransparency`, and `accessibility.increaseContrast` (both bool, default false). Use the existing settingsd schema/default/persistence paths.
- [ ] Add accessible Appearance and Accessibility controls with keyboard operation and immediate preview/feedback. Coalesce live drag writes and flush the final value, following existing Settings write/refresh conventions. Give the preview the same effective preset resolver as the rendered menus. Make affected surfaces clear in the control text.
- [ ] Carry the current effective scheme and material preferences through the existing trusted shell-to-compositor policy bridge. If a request is needed, append and version-gate it. Apply a coherent snapshot at startup, on live updates and after service/shell reconnect. Repaint only affected surfaces and invalidate the matching compositor paint state.
- [ ] Ensure users can activate the accessibility overrides independently of the appearance rollout switch. Preserve the existing Accent, Auto scheme, Reduce Motion and Control Center Dark Mode semantics.

## Out of scope

- Surface optics tuning and any unsupported Dock/window slider curve. Menu Bar and Desktop & Dock controls retain their existing schemas.

## Design notes

Use settingsd as the durable owner. Resolve Auto to the same effective scheme in shell/compositor; the bridge must react to host scheme change while Auto is active. Follow the existing typed validation and offline write behavior. Use schema defaults for missing keys and coherent refresh after service return. Keep legacy IPC semantics and ensure a rejected or unsupported update yields a safe local material. A visual preference must never reconfigure work area or schedule persistent repaint.

## Done when

- [ ] Add regression coverage in settingsd schema/persistence, Appearance/Accessibility QML, `tst_themebinding`, `tst_compositorpolicy`, and a headless shell/compositor policy round trip. Verify old settings, restart, service absence, endpoints, keyboard, active material policy invalidation, and both overrides together.
- [ ] `make gates` passes. Update `docs/settings-keys.md` and the policy documentation. Fill in Hand-off.

## Hand-off

_(Implementation session records the effective setting contract, version behavior and tests.)_
