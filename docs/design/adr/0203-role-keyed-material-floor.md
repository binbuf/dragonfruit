# 0203 — Role-keyed material floor and the v4 role state

## Status

accepted; persistent-floor selection amended by
[ADR 0211](0211-persistent-chrome-stability-and-blur-cache.md). Persistent
chrome still never selects below `Reduced`, but a global `Minimal` selection
now keeps `blur_active=1` instead of emitting `floor-missed`.

## Context

T-04's degrade ladder selected one global tier and applied it to every chrome
surface. T-21.2c needs the Dock and menu bar to be **persistent chrome** that
does not silently lose its material when a transient overlay (Mission Control,
the Applications drawer, an OSD) pushes the global selection to `Minimal`.
The compositor already had `MaterialRole::{Chrome, Dock, Popup, Drawer}` and a
v3 `df_output.material_state` event carrying one global snapshot. The open
questions were how to classify roles, how a persistent role falls back when
even `Reduced` misses, and how to extend the protocol without changing the v3
event's meaning or emitting per frame.

## Decision

- `MaterialRole::is_persistent()` is the single classification: `Chrome` and
  `Dock` are persistent; `Popup` and `Drawer` are transient. `wire()` fixes the
  stable role values 0=chrome, 1=dock, 2=popup, 3=drawer.
- `EffectiveMaterial::resolve_for_role()` applies the floor:
  - persistent roles never select below `Reduced` while the sampled path is
    available;
  - transient roles follow the ladder to `Minimal`;
  - when the controller selected `Minimal`, persistent chrome keeps
    `tier=reduced`, turns `blur_active` off, and reports the new
    `MaterialFallback::FloorMissed` (`fallback=4`, `floor-missed`) so the
    shell draws the deliberate milky plate;
  - the software/headless path stays `Minimal` with `fallback=software` for
    every role.
- The render path resolves every chrome panel through its role's effective
  state (`render::chrome_backdrop_render_elements` /
  `render::chrome_blur_panels`), so the floor applies to the actual sampled or
  feather pass. A role with `blur_active=0` emits no backdrop elements and
  owns no damage.
- `df_output` is v4. The additive `material_role_state` event carries
  `role/tier/blur_active/fallback`; it is emitted per output only for roles
  whose effective state changed. The legacy v3 `material_state` keeps its
  global snapshot and change-only rule for older clients.
- The pure policy (`DegradeTier`, `MaterialRole`, `EffectiveMaterial`) is
  testable without a compositor; the synthetic harness gains
  `set material-path gpu|software` and `set material-blur-supported on|off` so
  the headless protocol conformance test can exercise the sampled-path floor.

## Consequences

- The Dock/menu bar keep `Reduced` (or the explicit milky fallback) while
  transient overlays may reach `Minimal`; the shell gets the same decision the
  renderer uses, so it never has to re-derive the policy in QML.
- The v3 event remains byte-compatible in meaning. Shells that have not been
  updated keep their conservative global behavior; v4 clients get the role
  table and the `floor-missed` reason.
- The remaining T-21.2c work is shell application (Dock/menu bar, then the
  popup family) and refresh-aware hysteresis; this ADR fixes the state shape
  those tasks consume.
- A forced `Minimal` is no longer a universal "blur off": persistent chrome
  reports the floor-missed milky fallback, which is the intended diagnostic
  result rather than a policy override.

## References

- `docs/tasks/0.1.1/reference/189-t-21.2c-persistent-chrome-material-floor.md`
- `compositor/src/window/degrade.rs`, `compositor/src/window/backdrop.rs`
- `compositor/src/render.rs`, `compositor/src/shell/mod.rs`
- `protocols/dragonfruit-toplevel.xml`
- ADR 0015 (degrade tiers), ADR 0189 (effective material and fallbacks)
