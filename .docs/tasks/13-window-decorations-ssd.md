# T-13 — Window Decorations: SSD Titlebars and Traffic Lights

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `compositor/` (SSD renderer) + `design-system/` (visual spec) |
| **Depends on** | [T-02](02-compositor-core.md) · [T-04](04-window-model.md) · [T-08](08-design-system.md) |
| **Blocks** | [T-30](30-compatibility-bridges.md) (decoration themes) · Phase-2 exit (traffic lights in loop) |
| **Estimate** | L |
| **Design docs** | [05-window-decorations.md](../design/05-window-decorations.md) · [10-design-system.md](../design/10-design-system.md) |

## Summary

Server-side decoration: `xdg-decoration` negotiation with **SSD as the
compositor default**, a compositor-drawn titlebar with left-side traffic
lights rendered from design-system tokens, the shared behavior spec (hover
reveal, double-click zoom, window menu, fullscreen hover reveal), and the
documented three-tier policy for third-party apps.

## Background

Wayland's `xdg-shell` makes clients responsible for their own decoration by
default; we cannot reliably replace part of an application's pixels
([05-window-decorations.md](../design/05-window-decorations.md)). The
product promise is bounded: **"traffic lights wherever the application
permits native desktop decoration."** The decoration visual design is owned
by the design system so SSD titlebars and first-party app titlebars look
identical.

## Scope

### In scope

1. **Negotiation** (per
   [05-window-decorations.md](../design/05-window-decorations.md)):
   - Client requests SSD → compositor-drawn titlebar (Tier 2).
   - No decoration request → **compositor default is SSD**
     (cooperative-by-default; CSD only when the client asks).
   - Expected third-party outcomes (documented + tested):
     - Qt apps typically request SSD (very good).
     - GTK apps draw their own headerbars (Tier 3; steerable left via GTK
       settings we document and configure — ship the settings snippet).
     - Electron/Chromium and most SDL games are CSD (Tier 3, best effort).
     - Xwayland windows reliably land in Tier 2 (with T-06).
2. **SSD renderer** in the compositor: titlebar + left traffic lights
   (close/minimize/zoom) rendered from the **generated Rust token module**
   (T-08) so it cannot drift from first-party `TitleBar`s; shadows,
   translucency, rounded corners per tokens.
3. **Behavior spec** (identical wherever our decorations appear):
   - Traffic-light glyphs **reveal on hover of the button cluster**;
     colorless otherwise.
   - **Double-click on the titlebar zooms** (T-04 Zoom state).
   - **Right-click opens the window menu**: Move to Space, Minimize, Zoom,
     Close ([03-workspaces.md](../design/03-workspaces.md)).
   - **Fullscreen windows hide the titlebar; a hover reveal keeps controls
     reachable.**
4. **Snap/geometry integration**: resize edges, double-click zoom, and
   drag-the-titlebar-to-move (pointer + touch), honoring translucent input
   regions of the client beneath the bar.
5. **Tier-3 dignity rule**: we never inject a fake compositor titlebar
   above a CSD application — enforce as an architectural rule (no code path
   that draws SSD over a client-decorated surface).

### Out of scope

- First-party `TitleBar`/`TrafficLights` QML components (T-08 — this ticket
  matches them visually).
- Third-party decoration *themes* (compatibility phase, T-30).

## Requirements

- FR-1: Negotiation matrix passes: SSD-requesting client (Qt test app), no
  request (default SSD), CSD-requesting client (we never draw a titlebar),
  Xwayland window (always SSD).
- FR-2: Screenshot-diff test: SSD titlebar vs design-system `TitleBar` at
  identical tokens — pixel-identical (the "unable to drift" rule).
- FR-3: Behavior suite: hover reveal, double-click zoom, right-click menu
  contents, fullscreen hover reveal — pass on all SSD windows.
- FR-4: Titlebar drag moves the window; drag to screen edge behavior
  decided (macOS does nothing special by default — keep it boring), and
  resize handles work on all edges.
- FR-5: Traffic-light glyphs are our own original artwork (IP rule,
  [14-risks.md](../design/14-risks.md)) — not Apple's.
- FR-6: The window menu's "Move to Space" lists Spaces from T-05 and works
  for Xwayland windows too.

## Acceptance criteria

- [ ] Tier matrix test green (including GTK-under-CSD left-button GTK
      settings guidance published).
- [ ] Pixel-diff drift test in CI (SSD vs QML TitleBar).
- [ ] Traffic lights appear correctly in the core interaction loop
      (Phase-2 exit step).

## Test plan

- Headless protocol tests for xdg-decoration negotiation.
- Nested visual tests: hover/reveal animation, dark/light, blur behind
  translucent titlebars.
- Fuzz: malformed xdg-decoration requests never crash.

## Risks / open questions

- Rounding/antialiasing differences between compositor GL and Qt scene
  graph could break the pixel-diff — if so, diff with tolerance and keep
  the visual identity via shared tokens + spec, not literal equality;
  document the decision.
- Decide the SSD design language (translucency amount, corner radii) with
  art direction before implementation; tokens make changes cheap later.
