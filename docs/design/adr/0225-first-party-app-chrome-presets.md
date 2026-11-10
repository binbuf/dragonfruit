# 0225 — First-party app chrome uses app-local near-opaque presets

- Status: accepted
- Date: 2026-11-06
- Task: T206

## Context

Settings and Files are first-party CSD windows: they draw their own titlebar,
sidebar and toolbar on a transparent Wayland surface, while the compositor
draws the window shadow and keeps the normal stacking/focus behavior. The
reference measurements give a near-opaque sidebar/toolbar rather than a
sampled glass pass: Light Settings 0.986/0.857 and 0.961/0.965, Light Files
0.985/0.858 and 0.994/0.922, Dark Files 0.935/0.218 and 0.900/0.247. System
Settings Dark has no independent fit. The token transport stores a scalar
alpha and neutral tint; the muted sidebar's small chroma pickup has no channel.

## Decision

- **Four app-local presets.** `settingsSidebar`, `settingsToolbar`,
  `filesSidebar` and `filesToolbar` are `chrome`/`solid` presets with no blur
  and no slider response. The measured values are stored verbatim; Settings
  Dark is explicitly `adapted` from the Finder dark pair. The Files surfaces
  stay independent in both schemes. They are QML-only; the compositor never
  resolves them because it never draws an app's sidebar/toolbar.
- **Client alpha, not a sampled pass.** `AppWindow.chromeTransparent` removes
  the generic opaque frame/content backing only behind the sidebar/toolbar.
  The app's normal transparent surface lets the resolved alpha composite
  against the compositor's lower scene. The titlebar keeps an opaque backing,
  and the detail sheet and Files path bar stay opaque. `ChromeSheet` draws one
  fill with independent corner radii so the sidebar (bottom-left) and
  detail/path bar (bottom-right) round exactly one window corner without
  stacking translucent overlays.
- **One QML complement and explicit fallbacks.** `Material.appChromeFill()`
  and `appChromeBorder()` resolve through `Glass`. The legacy rollout keeps
  the existing opaque `surfaceMuted`/`chrome` fills; a non-composited or
  unsupported client surface (`Glass.appChromeSupported == false`, capture
  seam `DF_APP_CHROME_SOLID=1`) keeps the same opaque plate. Reduce
  Transparency takes the shared opaque accessibility plate. Increase Contrast
  raises the separator ink. Focus is a controls/text state; the material is
  focus-invariant.
- **The window shadow is a ring.** The compositor subtracts the window body
  from its shadow spans, so a translucent sidebar/toolbar cannot sample its
  own shadow and darken the measured result. The QML `Shadow` is hidden on
  this path because the compositor owns the real window shadow. ADR
  [0257](0257-window-corner-cutout-transparency.md) refines the cutout to the
  decorated window bounds so the rounded-corner cutout stays fully
  transparent instead of carrying a faint shadow square.
- **Chroma stays open.** The presets carry alpha/tint only. Captures report
  the rendered channel spread separately; reproducing the reference's small
  sidebar chroma pickup needs a future chroma/tint transport.

## Consequences

- Settings and Files mirror `appearance.colorScheme`, `appearance.accent`,
  `accessibility.reduceMotion`, `reduceTransparency` and `increaseContrast`
  onto their own `Theme`/`Glass`; the Files process gains a small
  `FilesSettings` wrapper over the shared settings client.
- The measured Light and Files Dark values are exact token targets; Settings
  Dark remains an adaptation until a controlled System Settings Dark capture
  exists.
- `make first-party-chrome-check` is in `make gates`; `make
  first-party-chrome-capture` records Light/Dark focused, unfocused, Reduce
  Transparency, Increase Contrast and solid rows over neutral and saturated
  backdrops.
- A future general client backdrop protocol could replace the app-local
  alpha path, but this task deliberately does not add one.
