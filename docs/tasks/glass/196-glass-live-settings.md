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

- [x] Add `appearance.transparency` (double 0–1, default 0.5255, clearer toward 1), `accessibility.reduceTransparency`, and `accessibility.increaseContrast` (both bool, default false). Use the existing settingsd schema/default/persistence paths.
- [x] Add accessible Appearance and Accessibility controls with keyboard operation and immediate preview/feedback. Coalesce live drag writes and flush the final value, following existing Settings write/refresh conventions. Give the preview the same effective preset resolver as the rendered menus. Make affected surfaces clear in the control text.
- [x] Carry the current effective scheme and material preferences through the existing trusted shell-to-compositor policy bridge. If a request is needed, append and version-gate it. Apply a coherent snapshot at startup, on live updates and after service/shell reconnect. Repaint only affected surfaces and invalidate the matching compositor paint state.
- [x] Ensure users can activate the accessibility overrides independently of the appearance rollout switch. Preserve the existing Accent, Auto scheme, Reduce Motion and Control Center Dark Mode semantics.

## Out of scope

- Surface optics tuning and any unsupported Dock/window slider curve. Menu Bar and Desktop & Dock controls retain their existing schemas.

## Design notes

Use settingsd as the durable owner. Resolve Auto to the same effective scheme in shell/compositor; the bridge must react to host scheme change while Auto is active. Follow the existing typed validation and offline write behavior. Use schema defaults for missing keys and coherent refresh after service return. Keep legacy IPC semantics and ensure a rejected or unsupported update yields a safe local material. A visual preference must never reconfigure work area or schedule persistent repaint.

## Done when

- [x] Add regression coverage in settingsd schema/persistence, Appearance/Accessibility QML, `tst_themebinding`, `tst_compositorpolicy`, and a headless shell/compositor policy round trip. Verify old settings, restart, service absence, endpoints, keyboard, active material policy invalidation, and both overrides together.
- [x] `make gates` passes. Update `docs/settings-keys.md` and the policy documentation. Fill in Hand-off.

## Hand-off

**Landed.** settingsd revision 18 adds `appearance.transparency` (double
0–1, default 0.5255, clearer toward 1), `accessibility.reduceTransparency`
and `accessibility.increaseContrast` (both bool, default false) through the
existing schema/default/persistence path. The shared client defaults mirror
them; a schema-17 file migrates losslessly and a rejected write leaves the
store untouched.

The Appearance pane adds an accessible attached-menu transparency slider with
a 60 ms coalescing timer and a `committed` flush; its preview calls the same
`Glass.resolve("menuAttached", …)` resolver as the rendered surfaces. The
Accessibility pane adds keyboard-operable Reduce Transparency and Increase
Contrast toggles plus a resolver-backed preview. The control text names the
affected surfaces (light attached menus; Dock/menu bar/windows/panels keep
their own materials). Both overrides apply in the legacy and replacement
looks, together and independently of the rollout; Reduce Transparency wins
over the slider and Increase Contrast still boosts the opaque plate. Accent,
Auto scheme, Reduce Motion and Control Center Dark Mode are unchanged.

**Bridge.** `ThemeBinding` keeps the shell's `Glass` singleton in step;
`compositorpolicy` maps the three values with a safe fallback; the shell sends
one additive v9 `df_toplevel_manager.set_appearance_policy` snapshot (resolved
scheme, slider in millionths, both overrides) on startup, every
`changed`/`refreshed`, and host scheme changes. The compositor applies it
atomically, ignores unknown schemes, clamps/falls back on malformed sliders,
and advances `material_revision`/`needs_redraw` only on a real change. The v5
motion/input requests stay for older compositors. `query policy` and `query
material` expose the live snapshot for headless tests.

**Checks.** `cargo test -p dragonfruit-settingsd` PASS (41 unit + restart +
schema-doc + 5 session-bus); `cargo test -p dragonfruit-compositor --lib` 76
PASS; the new `appearance_policy_round_trips_and_invalidates_only_on_change`
and the existing motion/input conformance cases PASS (full suite 49 in gates);
`cargo clippy` for both crates clean; `make check-tokens`,
`make check-design-tokens`, `make glass-presets-check` (151) PASS; `make
qml-test` 83/83 PASS; `make check-i18n` PASS after refreshing
`translations/dragonfruit.ts` with the new pane strings; final `make gates`
PASS (glass 120, presets 151, e2e, QML 83/83).

**Docs.** `docs/settings-keys.md`, `docs/design/10-design-system.md`,
`docs/design/04-shell.md`, `docs/design/08-settings.md`, ADR 0034 and new ADR
[0215](../../design/adr/0215-glass-live-settings-and-appearance-policy.md).
Reusable facts/follow-ups are in
`docs/tasks/glass/plan/progress/T196.md`.

**Deviations / remaining.** No blockers. The transparency slider is only an
optical input for the light attached-menu preset (per T195); while the default
rollout remains legacy it is a durable preference with a mode/percent preview
readout rather than a visible change. T197+ still owns consuming the resolved
preset at the render boundary; T208 still flips the rollout default.
