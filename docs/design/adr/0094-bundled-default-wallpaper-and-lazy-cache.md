# 0094 — The shipped default wallpaper and the lazy provider cache

## Status

accepted — amends [0055](0055-online-wallpaper-content-provider.md)

## Context

The Wallpaper pane and the compositor's image-wallpaper path (T-09.3, T-05.4)
take a local file path. ADR [0055](0055-online-wallpaper-content-provider.md)
decided the desktop would **fetch, never bundle** Wikimedia Featured Pictures,
and that the out-of-box background would be the deterministic top Nature photo.
That made the first-run desktop depend on a network round-trip and on the
provider having populated its cache.

The project now has its own original default wallpaper,
`assets/graphics/wallpapers/Default.jpg`. The wallpaper track is being moved
forward to run immediately after the Dock experience addendum (T-14.7a–k), so
the first-run background and its API wiring must settle before the T-15 breadth
phase and T-16/T-17. Loading the whole Featured catalogue at session launch
would also stall bring-up for a surface the user has not opened.

## Decision

- **Ship the original default.** `assets/graphics/wallpapers/Default.jpg` is
  installed to a stable share path and is the out-of-box, first-run, and
  cold-cache/offline background. 0055's "fetch, never bundle" is narrowed to
  **never bundle fetched or third-party images**; original project assets may
  ship (consistent with the "original assets only" rule).
- **Effective-source precedence.** `wallpaper.source` (user choice) when
  non-empty, otherwise `wallpaper.builtinDefault` (the shipped default),
  otherwise `wallpaper.providerSource` (the deterministic Nature entry, now a
  fallback), otherwise the Space's solid color. 0055's deterministic Nature
  default stays as the Featured row's default and the fallback when the shipped
  asset is unavailable — it is no longer the out-of-box background.
- **Lazy cache, eager on demand.** `services/wallpaperd` warms its cache in the
  background on session launch, off the compositor/shell frame path; the
  desktop already renders the shipped default, so nothing blocks. Opening the
  Wallpapers pane in System Settings calls a new `Preload` method for an eager
  foreground load; with no open pane the service stays lazy.
- **One API for consumers.** `org.dragonfruit.Wallpaper1` gains
  `BuiltinDefaultSource` (resolved shipped path) and `Preload`; System Settings
  consumes the interface through `SettingsBridge`, and the shell's
  `wallpaperpolicy` resolves the precedence. The compositor and
  `df_workspace.set_wallpaper` stay unchanged.

## Consequences

- The out-of-box desktop never depends on the network; a provider failure or
  absence is invisible at first run.
- Packaging must install `Default.jpg` and list it (MIT/`NOTICE`); fetched
  images are still never bundled ([licensing.md](../../licensing.md)).
- T-18.1a installs and resolves the default and implements lazy/`Preload`;
  T-18.1b adds `wallpaper.builtinDefault` and the System Settings/shell wiring;
  T-18.2 calls `Preload` when the pane opens; T-18.3 verifies the licensing and
  absence matrix.
- A future provider must still supply the attribution contract; the shipped
  default carries none because it is our own asset.