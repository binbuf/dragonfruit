# 0250 — Menu-bar reveal motion, measured overflow, and per-display policy

- **Status:** Accepted (T233)
- **Date:** 2026-11-09
- **Context:** [02-compositor.md](../02-compositor.md),
  [04-shell.md](../04-shell.md), [06-global-menu.md](../06-global-menu.md),
  ADR [0220](0220-menu-bar-auto-hide-visibility.md),
  ADR [0248](0248-status-item-registry-and-clock.md)

## Context

T201 made `menu.autoHide` real per output, but the reveal was an immediate
visibility flip with no hide grace or motion. T231 built one status-item
registry with a priority overflow rule, but the slot budget was an injected
number: the live bar never measured its own width, so a narrow display could
clip the row, and essential items could exceed a tiny budget. The multi-display
contract also stopped at per-output visibility: nothing named which display
owns the application menu or whether status items follow the active display.

## Decision

1. **The compositor owns the measured transition.** `MenuBarVisibility`
   (`compositor/src/shell/menubar.rs`) stores a per-output reveal intent, a
   pending delay, a running `Tween`, and the rendered `progress`. The tokens
   are `menuBar.revealDelay` (0 ms: the top edge is a hard pointer barrier),
   `menuBar.hideDelay` (350 ms of pointer-leave grace), and the shared
   `motion.menuBarReveal` (160 ms, `[0.2, 0, 0, 1]`). Reduced motion collapses
   both delays and the tween to zero, so the same machine settles in one step.
2. **The surface slides; the strip stays the hit target.** The compositor
   translates the menu-bar render location (and its panel/region metadata) by
   `(1 - progress) * surface height`. Input hit-testing keeps the top strip at
   the output edge, so the pointer that reveals the bar is already over its
   input region. The reservation is present while the target is visible or the
   slide-out is still running, and disappears only at progress zero: a
   concealed bar reserves and hits nothing.
3. **The budget is measured, not injected.** `MenuBar.qml` lays out an
   offscreen measurement row of the same `StatusItem` delegate over the
   registry's ordered list, then derives capacity from the live width minus
   the app-menu row, padding, the notch/safe-area inset, and the overflow
   slot. A finite `statusOverflowCapacity` remains a test/capture override.
   The registry removes third-party items first, then optional system items,
   and finally the lowest-priority essential items on a very narrow display.
   Every removed item is listed in the overflow menu; the row never clips, and
   a zero budget renders only the affordance.
4. **The notch is a width inset, not a hit target.** `notchWidth` only
   reduces the status row's available width, so a camera housing pushes items
   into overflow instead of clipping them.
5. **Per-display is a pure policy.** `MenuBarDisplayPolicy.qml` resolves one
   bar state per online display: the active display (the focused window's
   display, falling back to the primary) owns the application menu; status
   items follow `all` or `active`; an offline display has no bar. The
   compositor continues to composite the one all-output surface per output;
   this task does not add a second surface or change the layer protocol.
6. **The QML `revealProgress` is an introspection seam.** It mirrors the
   compositor's progress for scene tests and captures; production motion is
   compositor-owned, like the existing `visibleIn()` rule.
7. **The Menu Bar pane lists every live key.** The controls group now covers
   Wi-Fi, Bluetooth, Battery, Sound, Search, Notifications, Focus, Date &
   Time, Accessibility, and Quick Settings, and explains that overflow items
   move to the menu instead of being clipped.

## Consequences

- `query menubar` reports `progress`, the surface `offset`, and `input-top`
  per output alongside the existing mode/reveal/visible/reservation fields.
- A fullscreen output still direct-scanouts while the bar is concealed; a
  revealed bar is a real composition requirement and keeps its strip.
- The measured capacity follows a live width change, a localized clock, a
  tray label, and a notch; there is no second hard-coded slot count.
- Multi-display ownership is testable without a second surface. A future
  per-display surface implementation consumes `MenuBarDisplayPolicy` instead
  of re-deriving the rule.
- `tst_menubar` covers the measured budget/notch/tiny-bar fallback, the
  display policy and hotplug, and the reveal progress; the compositor unit
  tests cover delay/tween/reduced motion/per-output independence; the
  conformance test asserts the translated concealed bar has no input region
  and the settled bar restores both.
