# 0019 — Image wallpapers decode once into a cached memory buffer and slide with their Space

## Status

accepted

## Context

T-05.4 makes a Space's wallpaper an image (`Wallpaper.source` + `WallpaperFit`),
not only a solid color, and slides it with its Space during a switch. Every
backend composites through Smithay render elements and a clear color; there was
no image decoding dependency, no per-Space background element, and the
`Wallpaper` model shipped in T-05 only as a color. The design constraints are
that large images must not be decoded per frame and that no image configured (or
a broken one) must keep the existing color behavior.

## Decision

- **Decode with the `image` crate (PNG + JPEG only), once, into a cached
  Smithay `MemoryRenderBuffer`.** `compositor/src/wallpaper.rs::WallpaperCache`
  keys by `source` path and caches both successes and failures, so the render
  loop only imports an already-decoded buffer; a missing/undecodable source is
  never retried per frame and falls back to the solid `color`. `to_rgba8` bytes
  are uploaded as `Fourcc::Abgr8888` (GL `RGBA8`/`UNSIGNED_BYTE`).
- **The fit mapping is pure geometry.** `sample_wallpaper(source_size, fit,
  target)` returns the source crop and the output-local destination for
  `fill`/`fit`/`stretch`/`center`, unit-testable with no GPU.
- **One render seam.** `render::wallpaper_render_elements` (a
  `WallpaperRenderElement` enum: `Image`/`Solid`) is appended at the *bottom* of
  each backend's front-to-back element list, so the wallpaper composites behind
  every window and chrome. An image is drawn over its own solid fallback.
- **One slide formula.** `wallpaper::slide_offset` is shared with
  `DfState::apply_overview_scene`, and `DfState::wallpaper_slots` reports only
  the partly-on-screen Spaces, so the background translates exactly like the
  live window surfaces and a settled scene shows only the active Space.

## Consequences

- A new pinned workspace dependency (`image = 0.25.10`, `png` + `jpeg`).
- Wallpaper memory is one decoded raster per source per session; T-16's
  settings/picker and per-output polish reuse this cache and can call
  `WallpaperCache::clear` when a source is replaced.
- The wallpaper is a normal render element with a stable buffer id, so damage
  tracking and the T-05.6 capture/frame-budget work see it like any other
  element; desktop-icons (T-19) draw above it.
- Per-output wallpaper animation polish stays T-16; this ADR fixes only the
  decode/cache and the slide offset.