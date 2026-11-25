# ADR 0349 — Wallpaper provider reads and luma decode are asynchronous

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T338
- **Supersedes:** none

## Context

The shell read `wallpaperd`'s `DefaultSource` and `BuiltinDefaultSource`
properties with two blocking `QDBusConnection::call()` requests (the default
25 s timeout) and decoded the whole active wallpaper on the GUI thread to
compute the menu bar's top-band luma. A slow provider or a 4K wallpaper change
therefore froze the shell during startup and every Space/wallpaper switch. The
luma only needs a contrast decision, not the full-resolution image.

## Decision

1. `WallpaperProviderClient` owns one asynchronous, coalesced
   `org.freedesktop.DBus.Properties.GetAll` with a 2 s bound. A burst of
   provider `PropertiesChanged`/`ItemsChanged` signals becomes at most one
   follow-up read. The resolved values are cached in the client; a timeout,
   error, or absent provider keeps the last known values. A provider restart
   re-reads; an unregistration does not clear the desktop.
2. `WallpaperLuma` decodes a scaled thumbnail (320 px maximum) on a dedicated
   worker thread. The default decoder asks `QImageReader` for a scaled decode
   and scales again if the plugin ignores the request, so a full 4K buffer
   never reaches the luma pass. The result is cached by source path + mtime +
   size + fallback colour, so a Space switch on the same file is a cache hit
   and a replaced image is decoded again.
3. The shell applies a luma result only when its cache key is still the key on
   screen; a stale worker result cannot overwrite a newer wallpaper. An empty
   source resolves to the solid fallback immediately, without a worker.

## Consequences

- Startup and wallpaper changes never wait on `wallpaperd` or a full image
  decode. The menu bar keeps its previous luma until the thumbnail arrives;
  that is the intended "last known value" behavior.
- The cache key includes mtime and size, so editing a wallpaper in place is
  re-decoded. A same-mtime, same-size replacement is the one case that keeps
  the old luma.
- `tst_wallpaperluma` covers the scaled top band, the off-GUI-thread decode
  (slow decoder seam), the mtime cache, and a slow/coalesced/timed-out provider
  mock on a private session bus.
