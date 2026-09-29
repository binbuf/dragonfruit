# 0168 — The desktop layer composites between wallpaper and windows; layers are role-scoped

## Status

accepted

## Context

Track 19's T-19.3 makes `~/Desktop` a Files-owned icon view on a compositor
**desktop-layer** surface. The compositor previously composited only the
above-window `top`/`overlay` chrome layers and drew the per-Space wallpaper
itself; `chrome_render_elements` / `chrome_surfaces` explicitly skipped every
layer `< 2` ("a T-10/T-11 concern"). The trust model admitted both trusted
roles through the same `df_shell` factory, so any trusted client could request
any layer.

Four things had to be decided once, because later desktop-operations slices
(rename, drag-to-Trash, Reveal icon exposure) build on them:

1. Where the desktop surface composites relative to the wallpaper and windows.
2. Which role may create which layer.
3. How pointer input reaches the desktop surface without stealing from windows.
4. Whether a click on empty desktop clears window focus.

## Decision

- **The desktop band is `Wallpaper < Desktop < Windows < Chrome`.** A
  `background` (layer 0) `df_layer_surface` composes above the per-Space
  wallpaper and below every window. The pure `shell::layer::StackBand` enum and
  its derived `Ord` are the one ordering contract; `render::desktop_render_elements`
  emits the background surfaces and the backends append them *between*
  `window_render_elements` and `wallpaper_render_elements` in the front-to-back
  element list. `chrome_render_elements` keeps handling only above-window
  `top`/`overlay`.
- **Layers are role-scoped.** `TrustedRole::may_create_layer` grants the
  `background` layer to `DesktopIcons` (the Files `--desktop` process) and the
  above-window layers to `Shell`; neither crosses into the other's band. A
  trusted client asking for the wrong band is refused with the same
  `ERROR_ACCESS_DENIED` path as an untrusted `bind`. The role is carried by the
  launch token: `DRAGONFRUIT_LAUNCH_TOKENS` now accepts `[role:]hex` entries,
  `DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN` is the single-token fallback, and the
  desktop token is written to `<socket>.desktop-launch-token` (a `desktop:`
  entry mints `DesktopIcons`).
- **Hit-testing follows the paint order.** `input::chrome_under` only considers
  above-window layers, the window `Space` is tested next, and
  `input::desktop_under` is the fall-through, so a window always wins over the
  desktop and the desktop wins over empty space. No desktop surface mapped means
  the fall-through is `None` and pre-T-19.3 behavior is unchanged.
- **Focusing the desktop clears the active window.** `focus_changed` treats only
  *above-window* chrome focus as chrome-preserving (`is_above_window_chrome_surface`);
  focusing a `background` surface resolves to no window, so `active_window`
  clears and the front window is deactivated. The desktop surface itself can
  still take keyboard `OnDemand`, and `chrome_has_keyboard_focus` deliberately
  excludes it so a desktop click is not immediately cleared as "click-away".

## Consequences

- The ordering is asserted by `shell::layer::tests::layer_bands_composite_wallpaper_desktop_windows_chrome`
  and the role matrix by `shell::trust::tests::layer_creation_is_role_scoped`;
  the live role admission is exercised by
  `shell_protocol_conformance::desktop_and_shell_roles_own_disjoint_layers`.
- An output with no desktop surface behaves exactly as before; the feature is
  invisible until `dragonfruit-files --desktop` connects.
- The desktop process is one more trusted role to provision (session manager and
  dev tool follow-ups). The `bottom` layer (1) remains unimplemented and is
  aliased into the desktop band so it can never leak above windows.
- Rubber-band selection is shared view logic (`FilesIconView.marqueeSelected`),
  not compositor logic; the compositor only routes input.