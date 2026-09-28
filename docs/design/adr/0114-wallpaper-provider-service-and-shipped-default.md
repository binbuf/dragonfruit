# 0114 — The wallpaper provider service, its seam, and the shipped default

## Status

accepted

## Context

ADR [0055](0055-online-wallpaper-content-provider.md) specified the Commons
query, category map, and attribution contract; ADR
[0094](0094-bundled-default-wallpaper-and-lazy-cache.md) narrowed it: ship the
original `Default.jpg`, warm the cache lazily, and add a `BuiltinDefaultSource`
plus a `Preload` to the wallpaper API. T-18.1a is the implementation slice.

## Decision

- **`services/wallpaperd`** is a restartable session service serving
  `org.dragonfruit.Wallpaper1` at `/org/dragonfruit/Wallpaper1` on the user
  session bus. Properties: `Status` (`idle|fetching|ready|offline`),
  `LastFetch` (Unix seconds), `DefaultSource` (fetched Featured
  default/fallback), `BuiltinDefaultSource` (resolved shipped default), `Items`
  (cached catalogue JSON). Methods `Refresh` and `Preload`; signals
  `ItemsChanged` and `StatusChanged`.
- **The content seam is `ContentSource`**: `catalogue(category)` and
  `download(item)`. `WikipediaSource` implements it over an `HttpClient`
  (`ureq`, MIT/Apache-2.0); `MockSource` is the CI implementation. A second
  provider can be added without changing the D-Bus surface.
- **The shipped default resolves** in this order: `DF_DEFAULT_WALLPAPER` →
  the first existing `$XDG_DATA_DIRS/dragonfruit/wallpapers/Default.jpg`
  (default `/usr/local/share:/usr/share`) → the in-tree
  `assets/graphics/wallpapers/Default.jpg`. It is read in place and is never
  copied into the cache. `make install` / `dragonfruit-session
  --install-session` installs it to `<prefix>/share/dragonfruit/wallpapers/`.
- **The cache** is `$XDG_CACHE_HOME/dragonfruit/wallpapers/`: `index.json`
  plus `<category>/<pageid>.<ext>`. `lastFetch` lives in `index.json`; a cold
  start rebuilds the catalogue from it with no network.
- **Lazy by default, eager on demand**: the service spawns one delayed
  background warm and then answers; `Preload` runs the same refresh
  synchronously for an open pane. Both refresh only when the cache is empty or
  at least a week old. The lock is held only for state transitions, so no D-Bus
  read blocks on the fetch, and `begin_fetch` admits exactly one refresh.
- **Offline keeps the cache**: an empty fetch result sets `offline` without
  replacing items or advancing `lastFetch`; the shipped default always
  resolves.

## Consequences

- The compositor and `df_workspace.set_wallpaper` are unchanged; the provider
  produces a local path like the portal chooser does.
- T-18.1b wires `wallpaper.builtinDefault` and the System Settings/shell
  precedence onto this interface; T-18.2 calls `Preload` when the pane opens;
  T-18.3 verifies licensing and the absence matrix.
- The new HTTP dependency is permissive only; no copyleft is added
  ([licensing.md](../../licensing.md)). Fetched images remain unshipped.