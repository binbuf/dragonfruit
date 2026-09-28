# 0115 — The wallpaper effective source and its absence behavior

## Status

accepted

## Context

ADR [0094](0094-bundled-default-wallpaper-and-lazy-cache.md) fixed the
precedence (user choice, then the shipped default, then the fetched Featured
fallback) and ADR
[0114](0114-wallpaper-provider-service-and-shipped-default.md) shipped
`services/wallpaperd` with `BuiltinDefaultSource` / `DefaultSource` properties
and the additive `wallpaper.provider*` keys. T-18.1b is the wiring slice:
settingsd keys, the shell forwarder, and System Settings.

The provider is a session service that may be absent, restart, or resolve no
default; `settingsd` is likewise independently restartable. The out-of-box
background must therefore not depend on either being up.

## Decision

- **Additive keys.** Revision 10 declares `wallpaper.provider`,
  `wallpaper.providerAutoFetch`, `wallpaper.providerLastFetch`,
  `wallpaper.providerSource`, and `wallpaper.builtinDefault`, all owned by
  `wallpaperd`. `wallpaper.source` stays the user override owned by
  `apps/settings`; the provider never writes it.
- **One pure precedence.** `shell/src/wallpaperpolicy.*` resolves the effective
  source as `wallpaper.source`, else `wallpaper.builtinDefault`, else
  `wallpaper.providerSource`, else the Space's solid color. `fit` and
  `showOnAllSpaces` are unchanged; the compositor and
  `df_workspace.set_wallpaper` are unchanged.
- **Two inputs, no hard dependency.** The shell reads both the settingsd keys
  and the provider's live `BuiltinDefaultSource` / `DefaultSource` properties
  (subscribed with restart resync). A non-empty settingsd key wins; otherwise
  the provider property fills in. If both are missing, the shell resolves the
  shipped asset itself (`DF_DEFAULT_WALLPAPER` →
  `$XDG_DATA_DIRS/dragonfruit/wallpapers/Default.jpg` → the in-tree asset,
  mirroring `wallpaperd`). A provider restart keeps the last known values; an
  absent provider is invisible.
- **One accessor for panes.** `SettingsBridge` (ADR 0036) gains
  `providerItems`, `providerStatus`, `providerDefault`, and
  `wallpaperBuiltinDefault`, plus `preloadWallpapers()` over the provider's
  `Preload`. Absence yields empty items/status and a still-resolved shipped
  default, never an error. Panes never touch D-Bus.

## Consequences

- The first-run and offline desktop renders the shipped `Default.jpg` with no
  service on the bus.
- T-18.2 consumes the bridge properties and calls `preloadWallpapers()` when
  the Wallpaper pane opens; T-18.3 verifies the licensing and absence matrix.
- A future provider that persists its defaults to settingsd needs no shell
  change: the same keys already win over the live properties. `wallpaperd`
  currently publishes them as D-Bus properties only; the settingsd keys are the
  persisted mirror when it chooses to write them.