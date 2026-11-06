# 0212 — Rounded blur spans receive span-local damage

## Status

accepted (2026-11-05, v0.1.1 native Dock artifact fix)

## Context

After the T-21.5 material-stability change, the native Dock still showed
hard-edged patches of the raw window/wallpaper coming through the glass while
the pointer moved across tiles. The artifact was independent of magnification
and of the rectangular hover wash: with both disabled, a tile/cursor-sized
region still lost its frost for a frame.

The blur panel is a rounded rectangle drawn as a stack of horizontal spans
(`rounded_rect_spans`). Smithay's damage tracker passes `element.draw` a damage
list in **element-local** coordinates. The element then draws each span with
`render_texture_from_to`, whose `damage` argument is **local to that span's
destination rectangle**. `BackdropBlurElement::draw` was forwarding the
panel-local damage unchanged to every span.

The central span starts one corner-radius below the panel origin, and the
corner spans start at different x offsets. Passing panel-local damage through
unchanged shifted the scissor by that offset. For a small damage rectangle
around a hovered tile, the shifted scissor no longer covered the real damaged
pixels, so the blur panel did not paint them. The translucent Dock surface
then composited over the raw scene underneath, which is exactly the
"background coming through" patch users saw.

## Decision

- `BackdropBlurElement::draw` computes the panel's physical rect once and
  translates the element-local damage into each span's local coordinates
  before calling `render_texture_from_to`.
- The translation is `panel.loc - span.loc`, clipped to the span bounds. A
  damage rectangle outside a span draws nothing for that span.
- The pure `damage_in_span` helper is unit-tested for the central span offset,
  out-of-span clipping, and full-panel damage.
- The span's source mapping and the panel's blur cache are unchanged. This is
  a scissor-coordinate fix, not a material or geometry change.

## Consequences

- A small Dock surface commit (hover state, tooltip, badge) now repaints the
  blur under exactly the damaged pixels instead of leaving a raw-scene patch.
- Full-panel damage still covers every span; the rounded corners keep their
  existing source mapping.
- The fix applies to every rounded material panel (Dock, menu bar, popups,
  drawers), because they all use `BackdropBlurElement`.

## References

- `compositor/src/window/blur.rs` (`BackdropBlurElement::draw`,
  `damage_in_span`)
- `compositor/src/window/corner.rs` (`rounded_rect_spans`)
- ADR [0211](0211-persistent-chrome-stability-and-blur-cache.md)
- `docs/captures/t21-dock-flicker-fix.txt`
