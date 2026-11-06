# 0211 — Persistent chrome stays glass; every downgrade latches; panel blur is cached

## Status

accepted (2026-11-05, v0.1.1 native Dock flicker fix)

## Context

The native DP-1 4K/144 session showed the Dock switching between sampled
glass and the milky fallback every few seconds. The global degrade controller
was visiting `Full`, `Reduced`, and `Minimal`; the persistent-chrome floor
turned a `Minimal` selection into `tier=reduced, blur_active=false,
fallback=floor-missed`, and the shell immediately repainted the Dock with the
milky complement. That was a deliberate quality-floor policy, but on a
144 Hz 4K output the controller visited `Minimal` often enough that the swap
read as flicker.

At the same time, the material pass was rebuilding every panel's Kawase chain
on every material frame. The Dock's own hover surface is not part of the
sampled scene, so a pointer sweep changed the chrome but not the blur source:
the GPU work was repeated for identical pixels.

## Decision

1. **Persistent chrome never trades the sampled pass for a tier selection.**
   `EffectiveMaterial::resolve_for_role` keeps `Chrome` and `Dock` at
   `Reduced` or better and keeps `blur_active=true` whenever the backend
   supports the sampled pass. A global `Minimal` selection may still be
   reported by `degrade stats`, but it no longer changes the Dock/menu-bar
   material identity. `FloorMissed` remains in the wire enum for compatibility
   and for an explicit future failure path; a tier selection does not emit it.
   The milky complement is still used for a genuinely absent sampled pass
   (`NoSampledBlur`) or the software path.

2. **Every downgrade latches its boundary.** The previous policy latched only
   a downgrade that followed a recent upgrade, so the first Full → Reduced →
   Full cycle and the Reduced ↔ Minimal cycle remained. Now a downgrade always
   latches the boundary it crossed. Recovery across a latched boundary needs
   `EMA < 0.4 × budget` for 10 s. The normal 3 s / 0.75 × budget recovery still
   applies when no boundary is latched (for example after a forced tier is
   released).

3. **The upgrade gate remembers the failed tier.** Each tier keeps a smoothed
   cost estimate. A target whose last measured EMA is above `1.05 × budget` is
   not promoted into while the current tier is already using more than `0.5 ×
   budget`. A genuinely cheap current tier can still recover normally; a
   steady sweep whose lower tier already consumes most of the budget cannot
   climb back into the tier that just missed the deadline.

4. **Panel blur is cached, not rebuilt every frame.** `PanelBlurKey` covers the
   scene texture generation and commit, the panel's scene source rectangle,
   the blur spec, the chain size, and the output scale. A cache hit reuses the
   last Kawase texture; a miss clears the key before rebuilding and advances
   the panel's own `CommitCounter`. The panel commit is the damage contract:
   a scene change that alters the blurred pixels now damages the panel even
   when its geometry did not move, closing the stale-edge gap.

5. **Observability.** `material stats` adds `blur_skips`; `blur_passes` and
   `blur_draws` keep their existing meaning. A steady Dock sweep over an
   unchanged scene should advance `blur_skips` without advancing the rebuild
   counters.

> **Follow-up (2026-11-05).** These changes removed the tier-driven glass ↔
> milky swap but did not fix the remaining hard-edged raw-background patches
> seen while moving over the Dock. That artifact was a rounded-span damage
> coordinate bug; see
> [ADR 0212](0212-rounded-span-damage-translation.md). The material
> stability and blur cache remain necessary for the stable native material.

## Consequences

- The Dock and menu bar remain glass under transient load. They can move from
  Full to Reduced blur quality, but they do not flash to the milky plate.
- The controller may stay at `Reduced` longer after a real downgrade; this is
  intentional. A genuinely lighter load still recovers after the latch window.
- A scene change behind a stationary panel now repaints that panel once, which
  is the correct damage. A Dock-only hover reuses the cached blur.
- The full-output diagnostic (`DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1`) and the
  material rollback (`DRAGONFRUIT_DRM_MATERIAL=0`) are unchanged.
- Region tiling (separate top/bottom scene textures) remains a follow-up. The
  blur cache removes the repeated hover cost; tiling is the durable win for
  scene changes and multiple glass panels.

## References

- `compositor/src/window/degrade.rs`
- `compositor/src/window/material_cache.rs`
- `compositor/src/backend/{nested,drm}.rs`
- `docs/design/adr/0203-role-keyed-material-floor.md`
- `docs/design/adr/0206-refresh-aware-hysteresis.md`
- `docs/design/proposals/dock-flicker-native-material.md`
