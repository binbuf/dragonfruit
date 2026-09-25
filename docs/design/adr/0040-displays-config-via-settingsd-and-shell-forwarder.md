# 0040 — Displays config: settingsd owns the selection, the shell forwards to `df_output`

## Status

accepted

## Context

T-09.5 is the Settings app's Displays-basic pane. Its reference UI is the
macOS-style scaled-resolution tile list (`Larger Text`…`Default`…`More Space`)
plus rotation (`System_Preferences.md`). The design routes Displays to "the
compositor output API" ([08-settings.md](../08-settings.md)): the private
`df_output` protocol has `set_mode`/`set_scale`/`set_transform` and is
**token-gated to trusted session processes only** — the Settings app is an
ordinary first-party client and cannot bind it. The shell is the trusted output
client, and ADR [0034](0034-compositor-policy-via-shell-bridge.md) already makes
it the forwarder from the one settingsd owner to the compositor applier for
motion/input; T-09.3 did the same for wallpaper.

The compositor reports only the *current* mode, not the list of supported
modes, and the app has no channel to read shell state. Exact pixel-mode
selection therefore cannot be presented honestly in Wave 1; the reference
pane's resolution tiles are themselves scaled-resolution choices, which the
compositor expresses as an output scale.

## Decision

- **`display.scale` (double, 0.5–2.0) and `display.rotation` (enum
  `normal|90|180|270`) are settingsd keys** (`schema.rs`, `SCHEMA_VERSION` 3),
  written by `apps/settings`, so the selection persists and every consumer
  observes it through the one `Changed` signal. The reference "Resolution" list
  maps to `display.scale` (scaled resolution), matching the reference exactly;
  true mode enumeration is a T-16 item.
- **The shell is the forwarder**: `shell/src/displayspolicy.*` is a pure mapping
  (`display.scale` → `df_output.set_scale`, `display.rotation` →
  `df_output.set_transform`), applied by `ShellController::applyDisplayPolicy`
  on every settingsd `changed`/`refreshed`. `ShellProtocol` remembers the
  announced `df_output` handles and applies the policy after the manager replay,
  so a policy that predates the output list is not lost.
- **The compositor is the sole applier** and the output state remains
  compositor-owned; the shell keeps no second settings store. Wave 1 has no
  per-display selection, so the policy reaches every announced output;
  per-output targeting is T-16.

## Consequences

- The Settings app never touches the private output protocol directly; the
  trust boundary (chrome/output management is a privilege of the shell) is
  unchanged.
- `df_output.set_mode` is available but unused by Wave 1; the pane's
  `Resolution` rows are scale choices, and a real mode list requires a
  `df_output` modes event (T-16).
- Brightness, color, night light, VRR, and `Arrange…` stay out of the pane:
  they have no provider yet, and the no-half-panes rule ships only working
  controls.