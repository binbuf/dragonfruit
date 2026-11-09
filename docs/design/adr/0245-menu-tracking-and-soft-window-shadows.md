# 0245 — Menu tracking and soft window shadows

Status: accepted (2026-11-09)

The sibling `macos-design-review/docs/REPORT.md` §3.5 describes immediate menu
opening, strongly frosted menu cards, and soft window shadows extending about
30 pt. Its Apps material remains unmeasured. This pass builds on ADR 0244;
the drawer opacity and submenu grace interval below are product choices,
not new measurements of macOS.

## Menu interaction and rendering

- An open menu session switches top-level titles on the next event-loop turn.
  The previous dropdown finishes its exit immediately, so its translucent
  content cannot overlap the new menu. Ordinary dismissal retains the existing
  close animation and releases input immediately.
- Both menu components use the existing 150 ms submenu dwell as a crossing
  grace interval. The parent stays selected while crossing another row toward
  its child. Entering the child cancels the pending sibling selection; staying
  on a sibling commits it. Leaving a row before its initial dwell cancels
  opening. Disabled rows and separators clear stale highlights.
- Dock menu and submenu cards publish separate material regions, translated
  relative to the integer overlay crop. The gap and unused area in their
  bounding rectangle receive no glass. Fractional bounds round endpoints
  outward, matching the menu-bar path.
- Popup geometry and material metadata are sent only when changed, with caches
  reset on protocol teardown. Hover repaints no longer repeatedly configure
  an unchanged Dock overlay.
- Menu layout can settle during polish, after the overlay crop is chosen.
  Queue a new presentation when the Dock's popover bounds or material regions
  change, even when ordinary re-entrant scene updates are suppressed. Hidden
  software render-control scenes also size their content item explicitly when
  adding or removing popover headroom.

## Materials

- The sampled Apps card's baseline alpha is 0.78, down from 0.88. The existing
  slider mapping, frosted backdrop, contrast edge and opaque accessibility/
  unsupported-renderer fallback remain in place.
- Shared shadows use 24 weighted layers with a truncated Gaussian falloff,
  replacing eight equal-opacity bands. Source-over integration reaches the
  configured opacity; rounded endpoints keep the spread symmetric. QML and
  compositor use the same function and elevation tokens. The compositor keeps
  cached geometry, excludes the translucent window body, and shares physical
  edges at fractional scales. This is a soft-shadow approximation, not a new
  measured optical fit.

## Verification

Pointer-driven QML regressions cover diagonal submenu crossings, sibling dwell,
disabled rows, cancelled hover, and immediate top-level retargeting. Dock tests
cover independent panel regions and crop coordinates. Rust tests cover shadow
falloff, source-over alpha, fractional-scale edges and body exclusion.

Nested captures check menu materials and window rendering. These checks do not
measure native-session high-refresh performance or certify touchpad hardware.
