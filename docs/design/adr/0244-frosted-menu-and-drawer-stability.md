# 0244 — Frosted menu and drawer stability

Status: accepted (2026-11-09)

This corrects the menu/drawer material ownership in ADR 0243 following live
feedback. Reference: the sibling `macos-design-review/docs/REPORT.md` §3.5
describes a near-white menu tint of about 0.55 alpha over strong, approximately
10 pt frosting, and opening within one 60 Hz frame. `SURFACE-REVIEW.md` §4
requires a scrollable, search-first Apps collection; its material fit is open.

## Rendering

- Read menu material regions after QML's open transaction settles, and refresh
  them when submenu/layout bindings change. The previous synchronous read in
  `opened()` produced an empty region list for the entire open menu. Convert
  the published bar coordinates to popup-surface coordinates before committing
  them; otherwise the compositor adds the surface origin twice.
- Attached corners use one per-corner rectangle fill. Stacking another
  translucent rectangle over the top rounded corners doubled their alpha.

- The compositor supplies frosting for attached, detached and status menus;
  QML supplies the resolved tint once, with the content's opacity. An opaque
  compositor tint underneath separately animated client content could flash
  ahead of the content and retain an independent outline during transitions.
- Frosted cards downsample in panel space, like the existing Dock path.
  Stretching a small panel to output-sized blur textures both allocated excess
  GPU storage and weakened the effective blur by the panel/output ratio.
- A separate logical-sigma blur specification targets the measured 10 pt
  frosting, using five Kawase levels and scale-adjusted taps. It remains an
  approximation of the reference blur, not a newly measured optical fit.
  Cached textures and source-damage invalidation remain in use. Other material
  families retain their existing blur model.
- Menus open immediately. Their 183 ms close policy remains. Repeated service
  replies do not replace identical menu models; changed replies are deferred
  until the open menu closes, so a Repeater cannot destroy the active popup.
- Apps uses the existing 0.88 card-opacity token over the frosted backdrop,
  adjusted relatively by the transparency slider. This is a deliberate
  readable adaptation while the reference Apps material remains unmeasured.
  Its full-output scrim is disabled on the sampled path to avoid a second dark
  wash over the card. The card no longer scales while opening, avoiding repeated
  material geometry changes and texture reallocations.
- Reduce Transparency and unsupported backends keep their shared near-opaque
  plates. Increase Contrast draws the stronger edge with the client tint.

## Interaction

- The drawer dismisses on outside press with any mouse button. A background
  receiver inside the card preserves empty-space clicks without interfering
  with the search field, category controls or scrolling grid.
- The grid explicitly consumes pixel or angle wheel deltas. The shell converts
  fractional Wayland deltas before rounding, so small touchpad updates survive.
- Keyboard focus loss dismisses after one event-loop turn, allowing a genuine
  focus handoff to finish. Dismissal only releases keyboard focus if the drawer
  still held it. Control Center, Mission Control and the drawer retire one
  another when opened.

## Verification

Regression checks cover menu tint/opacity, immediate opening, drawer wheel
scrolling, all-button outside dismissal, keyboard selection visibility,
fallback states, and logical blur sigma at 1x/1.5x/2x. Nested GPU captures
verify menu frosting and the drawer card. Compositor
synthetic-input checks also scroll the live grid and dismiss it with an outside
right-button press. These checks do not certify native high-refresh frame
pacing or touchpad hardware behavior.
