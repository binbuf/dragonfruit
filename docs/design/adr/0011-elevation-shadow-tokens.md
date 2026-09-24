# 0011 — Elevation shadow tokens are the single shadow source

## Status

accepted

## Context

T-04.1a lands real shadows for chrome, SSD titlebars, and floating windows,
shared between the compositor and the QML design system so they cannot drift
(FR-2). Before this, `design-system/components/Shadow.qml` was the only shadow
renderer — a layered-rectangle approximation whose defaults mixed
`component.window.shadowBlur`, `component.shadow.{offsetY,layers}` and the
scheme's `material.shadowOpacity` — and the compositor drew no shadow at all.
The material track's later tasks (T-04.1b rounded corners, T-04.2 backdrop
blur, T-04.4 degrade tiers) all build on this geometry, and T-05/T-11/T-17
consume it, so the surface-to-geometry mapping has to be fixed once.

There are two possible owners for "what shadow does a surface cast": a
role→level table in code on each side (drift risk), or a shared token group.
The design system already generates one QML singleton and one Rust module from
`design-system/tokens/tokens.json`, which is exactly the guardrail the
anti-drift rule needs.

## Decision

- Add `component.elevation.{low,med,high,overlay}` to `tokens.json`, each
  resolving `blur`, `offsetY`, and `layers` from `primitive.elevation.*` and
  `component.shadow.*`. `scripts/gen-tokens.py` emits both `Theme.qml` and
  `design_tokens.rs` from it; hand-editing either generated file remains
  forbidden (`make check-tokens`).
- Both consumers read that group by **name**:
  - `design-system/components/Shadow.qml` takes a `level` string and derives
    its geometry from `Theme.controls.elevation[level]` (default `high`);
    `AppWindow` exposes a `shadowLevel` and passes it through.
  - `compositor/src/window/shadow.rs` (`ShadowLevel`/`ShadowSpec`) resolves the
    same values from `component::elevation::*`, plus the active scheme's
    `color.shadowColor` / `material.shadowOpacity`.
- The compositor's shadow is a stack of translucent solid rectangles (the
  `Shadow.qml` formula in logical pixels), appended **after**
  `window_render_elements` so it composites below its window;
  `WindowInsets::outset` supplies the whole decorated-window rect and the
  per-window `MotionFrame` is reused through `motion_transform`. Rounded-corner
  clipping and a real blur (T-04.1b/T-04.2) replace the approximation without
  changing this token contract.
- A surface's role picks the level: floating/SSD windows are `high`; popups
  and overlays are `med`/`overlay` when T-04.2 wires their compositor chrome.

## Consequences

- Shadow geometry can never drift between the compositor and QML: changing a
  shadow means editing one token and regenerating. `make check-tokens` fails
  on a hand edit.
- The generated `component.elevation` group is part of the public token
  surface; later material tasks extend it (a radius group, a blur group)
  rather than re-defining shadow geometry.
- Until T-04.1b/T-04.2, compositor shadows are rectangular and unblurred; the
  QML approximation remains the app-side renderer. The visual floor is
  deliberately approximate at this slice and the headless tests assert the
  token-derived geometry, not a blurred image.
- The compositor uses the default (dark) scheme until settingsd owns the live
  scheme (T-08); both sides already branch on the scheme, so that is a value
  change, not a geometry change.
