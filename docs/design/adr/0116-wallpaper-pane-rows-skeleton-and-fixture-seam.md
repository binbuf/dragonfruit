# 0116 — The Wallpaper pane's three rows, the Skeleton component, and the provider fixture seam

## Status

accepted

## Context

T-18.1a/T-18.1b shipped `services/wallpaperd` and exposed its catalogue and
effective source through `SettingsBridge` (`providerItems` / `providerStatus` /
`providerDefault` / `wallpaperBuiltinDefault` / `preloadWallpapers`), but the
Wallpaper pane still showed only the six original gradients. T-18.2 must
present the provider content, the shipped default (ADR
[0094](0094-bundled-default-wallpaper-and-lazy-cache.md)), the custom-chooser
row, and the fetching/offline states fixed by ADR
[0055](0055-online-wallpaper-content-provider.md).

The pane is a consumer: it must never fetch from QML and must keep working with
no provider, no `settingsd`, and no portal. The tests run headless with no
session bus, so the provider lifecycle has to be drivable in-process.

## Decision

- **Three source rows.** The pane is restructured into **Featured** (the
  provider catalogue), **Built-in** (the shipped `Default.jpg` "Default" tile
  plus the original gradients), and **Custom** (the portal `Add Photo…` row,
  disabled when the portal is absent). The selection model is unchanged: a
  tile writes `wallpaper.source`; `wallpaper.showOnAllSpaces` and
  `wallpaper.fit` stay where they were.
- **A `Skeleton` design-system component.** A grey rounded rectangle with a
  slow horizontal highlight (a plateau gradient), driven by `Theme.motion.
  skeleton` and themed by the semantic `skeletonBase` / `skeletonHighlight`
  colors. Under `Theme.reducedMotion` the animation is not started at all and
  the highlight rests centered, so the static variant is assertable and never
  animates. The gallery gains a page and light/dark/reduced goldens.
- **Featured states.** `Status=fetching` with an empty catalogue renders six
  `Skeleton` rectangles (accessible name "Downloading wallpapers"); a non-empty
  catalogue renders tiles with `category, title, license` accessible names;
  every other empty state (absent provider, cold cache, offline, error) shows a
  translated "Featured pictures will be available soon." note. Absence is never
  an error.
- **Attribution.** Only a fetched picture shows attribution: the artist, the
  license short name linked to `licenseUrl`, and the file page. The provider's
  HTML is stripped in `SettingsBridge::sanitizeHtmlText` and the labels render
  as `Text.PlainText`, so raw markup can never reach the UI. Built-in tiles and
  user photos carry no attribution.
- **Preload on open.** The pane requests `Preload` in `Component.onCompleted`
  (the shell's Loader instantiates the body on every open); leaving the pane
  destroys it and the provider returns to lazy. This never blocks: the desktop
  already shows the shipped default.
- **A guarded fixture seam.** `DF_WALLPAPER_FIXTURE` (mirroring
  `DF_SETTINGS_FIXTURE`) makes `SettingsBridge` serve a deterministic in-process
  catalogue and count `Preload` requests; the `Q_INVOKABLE setWallpaperFixture`
  seeds `ready` / `fetching` / `offline` / `error`. Without the environment
  variable both are inert, so production behavior is unchanged.

## Consequences

- The pane UI and its absent-provider behavior are asserted headlessly by
  `apps/settings/tests/tst_settings_wallpaper.qml` and
  `tst_settings_absence.qml`; the Skeleton by `tst_design_system.qml` and a
  gallery golden.
- `Skeleton` is the reusable loading placeholder for later panes; its token
  group is `component.skeleton` + `motion.skeleton` + the semantic
  `skeletonBase`/`skeletonHighlight` colors.
- The fixture seam is test-only surface on `SettingsBridge`; a future provider
  consumer must not depend on it.
- The shipped/Built-in tiles intentionally show no attribution; T-18.3 owns the
  licensing/absence matrix sign-off.