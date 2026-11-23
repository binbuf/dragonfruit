# 0327 — Shell idle effects: one controller, a dim overlay, and the logind path

## Status

Accepted (2026-11-23). Implements the shell half of
[ADR 0313](0313-idle-dpms-ownership.md); the timer/DPMS ownership and policy
keys are unchanged. The dim-overlay decision is superseded in part by
[ADR 0329](0329-panel-brightness-through-logind.md): with a real backlight the
panel write is the dim effect and the overlay is only the nested/headless
fallback.

## Context

ADR 0313 split the idle chain: the compositor owns the inactivity timer and
per-output DPMS, while the shell owns the user-facing effects. T315 shipped the
timer and the `set_idle_policy` request; T316 shipped `df_output.set_dpms`.
Nothing forwarded `idle.*` from settingsd, nothing handled the emitted
`idle-*` actions, and `power.preventSleep` had no consumer. The panel
backlight is not wired until T319, so a dim stage cannot yet change hardware
brightness.

The shell already has one lock entry point (`lockScreen()`), one logind power
path (`performSessionAction`), one settingsd client, and a full-output
`overlay` layer surface factory. A second timer, a new daemon, or a second lock
implementation would duplicate an existing owner.

## Decision

1. **One pure controller owns the stage transitions.** `shell/src/idlepolicy.*`
   reads the five keys, normalizes them to `IdleCompositorPolicy` (forcing
   suspend to `0` while `power.preventSleep` is set), and maps the five action
   strings to effects through a small `IdleEffects` callback set. It holds no
   protocol, bus, or QML handle, so `tst_idle` drives it with recording mocks.
2. **The shell forwards the policy on startup and on every change.** The
   controller sends the first snapshot even when it equals the compositor's
   defaults, and only re-sends when the effective policy differs. Disabled
   stages (`0`) do nothing; repeated stage actions are idempotent.
3. **Dim is a translucent full-output overlay until T319.** The shell maps an
   `overlay` layer surface named `idle-dim` with an empty input region and no
   keyboard, and commits a 45% black ARGB buffer. It also drops the forwarded
   `display.brightness` to half so the value is ready for the backlight writer.
   T319 replaces the overlay with the logind backlight write; the pre-dim value
   is restored on `idle-awake`.
4. **Blank, lock, and suspend use the existing paths.** `idle-blank` calls
   `df_output.set_dpms(false)` and records the pre-blank DPMS state;
   `idle-lock` calls `lockScreen()`; `idle-suspend` calls the existing logind
   `Suspend`; `idle-awake` restores the pre-dim brightness and the recorded
   DPMS state, not an assumed default.
5. **No HUD.** The OSD is not shown for an idle stage, and no new surface is
   focusable or takes input.

## Consequences

- The shell grows one pure module, one overlay surface, and five action
  branches; the compositor and the protocol do not change.
- `tst_idle` covers the mapping, the policy dedupe, `preventSleep`, the full
  stage sequence, disabled stages, idempotence, and the wake restore.
- The settings-consumer table marks `idle.dim`, `idle.blank`, `idle.lock`,
  `idle.suspend`, and `power.preventSleep` verified.
- The dim overlay is a temporary visual path: T319 must remove it when the
  real panel brightness write lands, or the screen would dim twice.
- The compositor still applies blank/wake itself; the shell's `set_dpms` call
  is idempotent and keeps the effect owner explicit.
