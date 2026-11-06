# 0215 — Glass preferences are settingsd keys and one v9 appearance snapshot

- Status: Accepted
- Date: 2026-11-06
- Task: T196

## Context

T195 defined the optical presets and two runtime resolvers but left the
appearance/accessibility inputs unwired. The reference evidence needs a
measured attached-menu slider plus two accessibility overrides that work even
while the replacement look is still behind the internal rollout switch. The
compositor has no D-Bus stack (ADR 0034), so the durable values must travel
through the trusted shell bridge.

## Decision

1. **settingsd owns three additive keys, revision 18.**
   `appearance.transparency` is a bounded double (0–1, default 0.5255, clearer
   toward 1). `accessibility.reduceTransparency` and
   `accessibility.increaseContrast` are booleans defaulting to false. They use
   the existing schema/default/persistence path, so an older file gains the
   defaults during migration and a rejected write never changes the store.
2. **The panes use one resolver.** The Appearance pane coalesces live drag
   writes (60 ms timer) and flushes the final value on `committed`; its preview
   calls `Glass.resolve("menuAttached", …)` with the live scheme, slider and
   overrides. The Accessibility pane toggles the two overrides and previews
   the same function. Neither pane implements a second optical curve. The
   control text names the affected surfaces (light attached menus; Dock, menu
   bar, windows and panels keep their own materials).
3. **One trusted v9 snapshot.** The shell maps the same `SettingsClient`
   values through `compositorpolicy` and appends the additive
   `df_toplevel_manager.set_appearance_policy` request (since 9). It carries
   the resolved scheme, the slider as millionths (so the 0.5255 default is
   exact on the integer wire), and both overrides. Older compositors ignore
   it and keep the v5 motion path. The compositor applies the whole snapshot
   atomically, ignores an unknown scheme, clamps/falls back on a malformed
   slider, and advances `material_revision`/`needs_redraw` only on a real
   change. A visual preference never reconfigures work area or schedules a
   persistent frame cadence.
4. **Accessibility overrides are rollout-independent.** Both overrides apply
   in the legacy and replacement looks. Reduce Transparency wins over the
   slider and disables sampling; Increase Contrast is applied after the fill
   decision, including on the opaque plate. Reduce Motion stays a separate
   motion-only key, and the existing Accent, Auto scheme and Control Center
   Dark Mode paths are unchanged.

## Consequences

- The shell and compositor share the same resolver vocabulary; T197+ consumes
  the resolved preset at the render boundary rather than re-deriving optics.
- `tst_compositorpolicy`, `tst_themebinding`, the settingsd schema/persistence
  tests, the Appearance/Accessibility QML tests, and the headless
  `shell_protocol_conformance` appearance-policy case pin the contract.
- T208 still owns flipping the rollout default. The slider remains a durable
  preference while the legacy look is selected; its preview readout reports
  the effective mode so the control is never a dead write.

## References

- ADR [0034](0034-compositor-policy-via-shell-bridge.md), ADR
  [0214](0214-glass-role-presets-and-resolution.md).
- `services/settingsd/src/schema.rs`, `apps/settings/AppearancePane.qml`,
  `apps/settings/AccessibilityPane.qml`, `shell/src/compositorpolicy.cpp`,
  `shell/src/shellprotocol.cpp`, `compositor/src/state.rs`.
