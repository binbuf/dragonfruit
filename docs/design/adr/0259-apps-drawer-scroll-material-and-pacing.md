# 0259 — Applications drawer: committed input, direct wheel path, and one near-opaque glass card

Status: accepted (2026-11-12)

> The near-opaque 0.97 alpha in this ADR is superseded by
> [ADR 0260](0260-apps-drawer-translucent-glass.md); the committed input,
> direct wheel path and stable-grid decisions remain in force.

This corrects the live Applications drawer regressions reported after ADR
[0244](0244-frosted-menu-and-drawer-stability.md): the first open/reopen could
not scroll or click, the card was a flat translucent slab when the material
degraded, and the grid reflowed while the user typed. The reference remains the
Spotlight **Applications** surface in
`macos-design-review/docs/SURFACE-REVIEW.md` §4: a search-first, 7-column,
scrollable collection that is visibly a large light glass panel.

## Decision

- **The input region is committed with the open, not "with the next frame".**
  `ShellProtocol::setAppsDrawerInputRegion` now issues a state-only
  `wl_surface_commit` immediately after `wl_surface_set_input_region`. A
  double-buffered region that waits for the next buffer commit can miss the
  first open (or the reopen after `hideAppsDrawer` set the region empty), which
  is exactly the "scroll and clicks fall through to the window below" failure.
  `showAppsDrawer` also applies the region before its first render, and the
  configure handler re-applies it when the drawer opened before the first
  configure.
- **The shell owns the drawer's wheel delta.** The drawer's offscreen
  `QQuickRenderControl` window delivers pointer presses to the QML scene but
  does not deliver wheel events to the QML pointer handlers. `onChromePointerAxis`
  converts the one Wayland axis value to the QML pixel/angle pair and calls
  `scrollByWheelAt(x, y, pixelY, angleY)` directly. The QML `WheelHandler`
  remains the QML/test path and shares the same `scrollGridByDelta` helper. The
  method ignores a wheel over the transparent output margin outside the card.
- **One drawer optical preset, compositor-owned.** A new `appsDrawer` glass
  preset (`role: drawer`, `lightRelative` slider) replaces the borrowed
  `menuDetached` preset. It carries the measured popover tint/rim model with a
  14 pt light / 16 pt dark frost, a restrained corner lens, and the near-opaque
  **0.97** alpha requested for this large browsing surface. `Material.qml`
  returns a transparent card fill while the compositor renders the sampled
  glass; the 0.97 milky plate remains the Reduce Transparency /
  no-sampled-blur / Minimal fallback. The QML border, top specular line and
  gradient become complements rather than a second material. The floating
  shadow is cut out of the card interior on the sampled path, so it cannot
  darken the glass the compositor draws underneath the surface.
- **The grid keeps its measured width while filtering.** `usedColumns` no
  longer follows `visibleApps.length`; a query that narrows the result set must
  not reflow the card horizontally on every keystroke. Seven columns (or fewer
  on a narrow output) stay fixed, results are left-aligned, and the card keeps
  one width.
- **Only visible rows decode artwork.** A tile requests its icon only when it
  is within one row of the grid viewport; a large app index no longer decodes
  every icon the moment the drawer opens.
- **Pacing follows the scene, not raw pointer samples.** Pointer motion,
  buttons, keys and wheel no longer schedule an unconditional full-output
  render. The QML scene's `sceneChanged` drives the existing chrome pacer;
  empty-space motion and no-op events cost no frame. The corpus projection is
  published only when it actually changed, so reopening the drawer does not
  rebuild every tile (and re-request every icon).

## Consequences

- The first and every subsequent open receive pointer and wheel input without
  requiring a prior pointer motion.
- The sampled card is near-opaque and legible over busy wallpaper while still
  carrying the compositor's frost, rim, lens and adaptive tint. The fallback
  path stays a deliberate plate.
- Filtering no longer moves the card or its columns; only the row count
  changes.
- The drawer does less work per pointer sample and per reopen. The full-output
  surface remains the render target; a content-sized drawer surface is a
  separate architectural change, not part of this correction.
- Tests cover the stable grid width, the compositor-owned card fill, the new
  preset's optics, and the existing wheel/keyboard/launch paths. The live
  nested check exercised the direct wheel path and a tile launch on the GPU
  transport.
