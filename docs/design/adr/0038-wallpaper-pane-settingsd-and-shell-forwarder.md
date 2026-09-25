# 0038 — The Wallpaper pane writes settingsd keys; the shell forwards them per Space

## Status

accepted

## Context

T-09.3 is the Settings app's Wallpaper pane. The app is a normal Wayland client,
not a trusted private-protocol client (the token roles are `shell` and
`desktop-icons`), and T-05.4 put the per-Space `Wallpaper` (`source` + `fit`) in
the compositor. The design routes the pane to "settingsd + compositor (per-Space
wallpaper)" ([08-settings.md](../08-settings.md)), so the pane needs both a
durable owner and a live applier, and it must obey the T-09.1b rule that a pane
never opens its own bus or protocol connection.

The schema had no wallpaper keys, and the shell already has the pattern for
`settingsd → shell → compositor`: `CompositorPolicy` forwards the motion/input
keys over `df_toplevel_manager` and the compositor is the sole applier
(ADR [0034](0034-compositor-policy-via-shell-bridge.md)). The `df_workspace`
interface already carries `set_wallpaper`.

## Decision

- **Three additive settingsd keys, since schema 2**: `wallpaper.source`
  (text, empty = solid color), `wallpaper.fit` (text enum `fill|fit|stretch|
  center`), `wallpaper.showOnAllSpaces` (bool, default true). The Settings app
  owns them (writes through the shared client); settingsd persists them.
- **The shell is the forwarder.** `shell/src/wallpaperpolicy.*` maps the keys to
  a pure `WallpaperSettings` (the `CompositorPolicy` analogue);
  `ShellProtocol::setWallpaper` sends `df_workspace.set_wallpaper` to every Space
  when `showOnAllSpaces`, otherwise to the active Space only. The request is
  remembered and re-sent after the manager's initial replay, so a selection that
  predates the Space list is not lost.
- **`df_workspace.set_wallpaper` changes only `source`/`fit`** and keeps the
  Space's solid fallback color, so a NULL source returns a Space to its default
  color without the shell knowing that color. The wire `color` argument is
  retained for the lockstep contract.
- **Built-in artwork is ours**: `SettingsBridge` renders six original gradients
  to stable PNGs under the user's app-data directory and exposes them as
  `Settings.wallpaperPresets`; the same file backs the pane preview and the
  compositor's decode. "Add Photo…" uses the xdg-desktop-portal FileChooser and
  disables when the portal is absent (the no-half-panes absent-provider state).

## Consequences

- Adding a wallpaper key means three places as before: `schema.rs`, the mirrored
  `settingsSchemaDefaults()`, and `docs/settings-keys.md` (the doc test enforces
  it).
- Distinct per-Space selections are live within a session when
  `showOnAllSpaces` is off, but only the last selection is persisted (one
  source/fit pair, not a map keyed by Space). Per-Space persistence needs a
  stable Space identity exposed to settingsd; it stays a T-16 follow-up.
- The compositor never sees a wallpaper it cannot decode fail the frame: an
  undecodable source falls back to the Space color (T-05.4/ADR 0019).
- A later Displays/output pane (T-09.5) reuses the same "settingsd key or
  explicit forwarder, compositor applier" shape rather than letting the app bind
  the private protocol.