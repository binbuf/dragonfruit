# 0183 — The chrome material rollout is per-surface role/panel-rect configuration

## Status

accepted

## Context

ADR [0182](0182-tahoe-liquid-glass-material-pass.md) landed the GPU sampled
blur (T-20.1) and the Tahoe liquid-glass pass (T-20.2) behind a
`MaterialRole`, but only the menu bar and the Applications drawer visibly used
it. Every other chrome surface — the Dock, menu-bar dropdowns, context
menus/popovers, the OSD, notification banners, and Control Center — either
still painted an opaque QML fill (hiding the material) or resolved its panel
rect/radius incorrectly. The pass and the role resolution already exist, so
the rollout is **configuration**, not new machinery: a surface that is not
frosted is a bug in its role, panel rect, or corner radius.

## Decision

- **Every chrome namespace resolves through one role.** The menu bar
  (`menubar`) is `Chrome`; the Dock (`dock`) is `Dock`; the Applications
  drawer (`apps-drawer`) is `Drawer`; and every overlay — menu-bar dropdowns
  (`menubar-popup`), Dock context menus/popovers (`dock-popup`), the OSD
  (`osd`), notification banners (`notification`), and Control Center
  (`control-center`) — is `Popup`. `MaterialRole::from_layer_namespace` is the
  single source; a full-output scene overlay (Mission Control, the switcher)
  stays excluded by `is_backdrop_panel`.
- **Panel rects follow the visible panel, not the surface.**
  `chrome_surfaces` derives the panel from a declared rect first, then the
  reserved strip narrowed by the input region, then the full geometry. The
  Dock plate and the drawer card declare their live rect via
  `df_layer_surface.set_panel_rect`; overlay popups are tight to their overlay
  surface; the OSD declares its visible card because its surface is larger than
  the card to fit the shadow (otherwise the frost would cover the transparent
  shadow margin).
- **The backdrop corner is the surface's own component radius.** A new
  `MaterialRole::corner_radius(namespace)` selects it: the OSD rounds from
  `component.osd.radius` (20 px), every other popup from
  `component.popup.radius` (14 px), the menu bar/Dock/drawer from their own
  component tokens. `spec_for`/`blur_spec_for` carry it through, so the frosted
  shape matches the QML card.
- **The QML surface is the material's scrim, not the material.** The
  popup-family QML fills carry `material.popupOpacity` (retuned to 0.62 light /
  0.70 dark), so the sampled backdrop and glass show through; the
  `border` hairline stays opaque as the rim complement. The same token feeds
  the feather fallback path, so the deterministic approximation cannot drift
  from the GPU path.

## Consequences

- `design-system/tokens/tokens.json` `material.popupOpacity` is retuned
  (0.96/0.92 → 0.62/0.70); `Theme.qml`/`design_tokens.rs` regenerate with
  `make check-tokens`.
- The one-pass guard is unchanged: the rollout only adds panels to the existing
  per-output pass, so `backdrop_skipped` stays zero.
- Per-surface captures (light + dark) are committed under `docs/captures/`;
  the menu-bar dropdown capture uses a `DF_MENUBAR_MENU_FIXTURE` seam because
  the right-anchored status row's x drifts with the clock width.
- Multi-GPU and the headless `Minimal` determinism remain T-20.4.