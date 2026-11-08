# 0240 — Wallpaper collection rows, Abstract/Space sources, and the apply state

## Status

accepted — amends [0055](0055-online-wallpaper-content-provider.md) and
[0114](0114-wallpaper-provider-service-and-shipped-default.md)

## Context

The T-18 pane ships the provider catalogue as one three-column "Featured"
grid inside a settings card. The macOS reference (measured on a 740×820
window with a 508-px detail column) is a flat gallery: a 160×100 hero with the
current name, a variant popup, "Show on all Spaces", and two deep-link buttons;
then sections with a title and "Show All (N)"/"Show Less", four 108×100 tiles
per row, 10-px gaps, and an inline four-column expansion. Clicking a dynamic
wallpaper switches the name immediately, replaces the popup with a 16×16
progress ring, keeps the old preview until the new image is ready, then swaps
the preview and restores the popup.

The current pane has no category rows, no Show All, no per-tile apply state,
and no Abstract/Space. It also shows a solid-color placeholder when
`wallpaper.source` is empty even though the effective background is the shipped
default.

The provider currently has six Wikimedia Commons categories. The obvious
"top-tier" stock APIs are not usable for a wallpaper picker: Unsplash's API
guidelines prohibit wallpaper applications and require hotlinking/download
tracking; Pexels says wallpaper apps are not supported; Pixabay prohibits
redistribution on wallpaper platforms and restricts hotlinking. Wikimedia
Commons and NASA are the permissive sources that fit the existing local-cache
and attribution model.

## Decision

### Pane

- **Flat hero.** A 160×100 preview, current name, source/attribution line, a
  `Fit` popup, a "Show on all Workspaces" toggle, and **Screen Saver…** /
  **Clock Appearance…** buttons. The hero is not a settings card.
- **Collection rows.** Abstract, Nature, Water, Cityscapes, Earth, Space,
  Scenery, Underwater, then Built-in and Your Photos. Abstract is the special
  first row (the macOS "Dynamic Wallpapers" equivalent); Space is the NASA
  source.
- **Four-column responsive tiles.** 3:2 thumbnails with captions, a selected
  ring/check, hover, arrow-key navigation, and a section-level
  **Show All (N)** / **Show Less** toggle. Built-in is expanded by default so
  the shipped default and every gradient are visible.
- **Your Photos.** The Add Photo tile is first, followed by the current custom
  photo when the user's choice is not a provider/built-in item.
- **Effective selection.** The pane selects the effective source (user choice,
  then shipped default, then fetched fallback) so the out-of-box Default tile
  is selected even while `wallpaper.source` is empty.
- **Apply state.** Clicking a tile keeps the old preview, shows a tile scrim +
  spinner and a 16×16 hero ring in place of the Fit popup, downloads a deferred
  full image when needed, then writes `wallpaper.source` and swaps the preview
  only once the image is ready and a short minimum dwell has elapsed. Failure
  keeps the previous wallpaper and shows a translated error.
- **Absence.** While the provider is fetching, empty categories show skeleton
  rows. A cold/offline/error catalogue shows the available-soon note and a
  **Try Again** button; cached items remain visible with a refreshing/offline
  status row. Absence is never an error.

### Sources

- **Keep Wikimedia Commons Featured Pictures** for the existing six categories
  and add **Abstract** from `Category:Featured abstract photos`.
- **Add NASA Image and Video Library** for **Space**. The search endpoint is
  keyless; the parser keeps landscape images of at least 1920×1080 with a NASA
  center/creator, filters logo/poster/chart/graphic records, credits
  `secondary_creator` or the center, and links the NASA media guidelines.
- **Deferred full-resolution download for NASA.** The catalogue fetch downloads
  only the medium thumbnail; the original is fetched by the new D-Bus
  `FetchItem(pageid)` when the user applies it. Wikimedia keeps its existing
  eager 3840-px download, so its tiles apply without a network round-trip.
- **Stable numeric ids.** NASA's string `nasa_id` is hashed with FNV-1a to a
  positive 31-bit cache key; the title/page URL keep the original id.
- **Force retry.** `Refresh` now forces a fetch even when the cache is fresh;
  `Preload` and the launch warm keep the empty/stale rule.

### API / cache

- `Items` entries gain `provider` (`wikimedia`/`nasa`), `fullUrl`, `thumbPath`
  (grid image), and keep `localPath` (full image, empty while deferred). Older
  indexes default `provider` to Wikimedia and fall back to `localPath` for the
  thumbnail.
- The cache adds `<category>/<pageid>-thumb.<ext>` for deferred thumbnails;
  `<category>/<pageid>.<ext>` remains the full image. Full files downloaded on
  demand survive a weekly refresh when the same id is still present.
- The D-Bus surface adds `FetchItem(pageid)` (JSON `{ok, pageid, source|error}`)
  and emits `ItemsChanged` on success. `Refresh` is forced; `Preload` is not.
- `SettingsBridge` exposes `thumbUrl`/`previewUrl`, `fetchWallpaperItem`,
  `refreshWallpapers`, and `wallpaperFetchFinished`.

### Deliberately out of scope

- Dynamic/light-dark wallpaper pairs, video/live wallpapers, favorites, and
  cross-device sync.
- Unsplash, Pexels, Pixabay, Wallhaven, and Flickr as sources (API/licensing
  restrictions or unclear per-image rights).
- The Met/AIC (CC0 art) and Openverse (aggregated CC) are reasonable future
  additions but are not part of this slice.

## Consequences

- The pane is no longer a card stack; it reads as a gallery while the rest of
  Settings keeps its settings-row anatomy.
- The macOS reference is interaction/layout only; no Apple artwork or dynamic
  wallpaper asset is copied. Abstract/Space are fetched at runtime and never
  bundled.
- NASA content is credited to NASA/center and linked to the NASA media
  guidelines; the parser filters logos/insignia and non-NASA creators. A future
  source must supply the same attribution contract.
- `SettingsMap`/search gain one shipped row per category; `SettingsMap.qml` and
  the business-logic map stay in sync.
- The Rust provider, session-bus surface, `SettingsBridge`, QML pane, fixtures,
  translations, and captures all move together. The headless QML suite drives
  the ready/categories/many/deferred/deferred-fail/fetching/offline/error
  states with generated local images, so the apply/download flow is testable
  without a bus or network.
