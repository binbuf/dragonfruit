# 0204 — The shell applies role material through one policy object

## Status

accepted

## Context

T189a made `df_output.material_role_state` (v4) carry an effective material per
`MaterialRole` and added the parsed role table to `ShellProtocol`. The Dock and
menu bar still applied the legacy v3 global `material_state` snapshot to both
surfaces, so a transient overlay could push the persistent chrome down and the
role table was unused. T189c will add the popup family, so the application
rule needs one owner rather than a copy in each QML component.

## Decision

- `shell/src/materialpolicy.{h,cpp}` is the shell's one application policy.
  `MaterialPolicy::applied(role)` returns the state a surface applies:
  a known v4 role wins, otherwise the legacy v3 global snapshot is used for
  compatibility.
- `ShellController` owns one `MaterialPolicy`. The menu bar always asks for
  `Chrome` and the Dock for `Dock`; no QML surface re-derives the degrade
  policy or picks a fallback token.
- The policy resolves the milky complement from the state: any explicit
  fallback reason (`no-sampled-blur`, `software`, `multi-gpu`, `floor-missed`)
  forces `blurActive=false`, which selects the existing token-driven fallback
  fill in `MenuBar.qml`/`Dock.qml`. Token values are unchanged.
- The shell keeps a defensive persistent-floor guard: a persistent role that
  reports `Minimal` with no fallback reason is applied as
  `tier=reduced`, `blur_active=false`, `fallback=floor-missed`. The compositor
  remains authoritative for tier selection; this only prevents a malformed or
  legacy snapshot from turning the Dock/menu bar into a flat Minimal slab.
- The v3 event stays connected. It seeds the compatibility path and never
  overwrites a role that has received a v4 state.

## Consequences

- The Dock and menu bar share one tested mapping from role state to QML
  properties, and the milky fallback is a deliberate state rather than an
  accident of `blur_active`.
- T189c should apply `Popup`/`Drawer` through the same `MaterialPolicy` object
  instead of adding a second mapping.
- The capture fixture converts its logical output rect to physical pixels
  before calling `captureScreenshot`, matching the real shell capture path at
  fractional scale.

## References

- `docs/tasks/0.1.1/reference/189-t-21.2c-persistent-chrome-material-floor.md`
- ADR 0203 (role-keyed material floor and v4 state)
- `shell/src/materialpolicy.h`, `shell/src/shellcontroller.cpp`
- `shell/tests/tst_materialpolicy.cpp`, `shell/tests/tst_dock.qml`,
  `shell/tests/tst_menubar.qml`
