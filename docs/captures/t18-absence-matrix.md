# T-18.3 — Wallpaper absence/state matrix

Reviewed matrix for the T-18 wallpaper provider (ADRs
[0055](../design/adr/0055-online-wallpaper-content-provider.md) /
[0094](../design/adr/0094-bundled-default-wallpaper-and-lazy-cache.md)).
The headless reproduction transcript is
[`t18-absence-matrix.txt`](t18-absence-matrix.txt), regenerated with
`make t18-absence-matrix`. The live stills are the T-18.2 captures.

The ruled invariant: **absence is a normal state.** No network, no provider,
no `settingsd`, or no portal must ever block the desktop, and the shipped
original `Default.jpg` always resolves.

## Matrix

| # | State | Expected | Reproduction | Result |
|---|---|---|---|---|
| 1 | Lazy launch (cold cache, pane never opened) | Service is `idle`, no network/disk fetch, shipped default resolves | `dragonfruit-wallpaperd --print-status` with an empty `XDG_CACHE_HOME` | `status:"idle"`, `builtinDefaultSource` set, `items:[]` |
| 2 | Cold cache + no network (`Preload`) | Eager fetch fails; service is `offline`, serves no items, still resolves the shipped default; nothing blocks | `HTTPS_PROXY=http://127.0.0.1:1 dragonfruit-wallpaperd --preload` | `status:"offline"`, `items:[]`, `builtinDefaultSource` set; cache holds only `index.json` |
| 3 | Warm cache + no network | Cached items survive; a failed fetch keeps `lastFetch` | `read_path::an_offline_refresh_keeps_the_cache` | pass |
| 4 | Provider restart | Fresh provider rebuilds the catalogue from the cache with no network | `read_path::a_cold_start_rebuilds_the_catalogue_from_the_cache_without_network` | pass |
| 5 | Weekly re-check | Within `WEEK_SECS` no fetch; stale cache re-checks; failed re-check is not a fetch | `read_path::a_fresh_cache_is_not_refetched` | pass |
| 6 | API error / rate limit | Any non-2xx (incl. HTTP 429) or transport error is a category failure; all failing keeps the old cache and goes `offline` | `provider::tests::an_empty_fetch_result_keeps_the_cache_and_goes_offline`, `provider::tests::the_service_state_warm_is_offline_safe` | pass |
| 7 | Lazy launch vs open-pane `Preload` | Load touches neither network nor disk; `Preload` does exactly one refresh, fills the cache, selects the deterministic Nature default | `read_path::preload_fetches_fills_the_cache_and_selects_the_featured_default` | pass |
| 8 | Session bus (properties, signals, offline) | `Preload`/`Refresh`/properties round-trip; `ItemsChanged`/`StatusChanged` fire; absent network answers `offline` with the shipped default; a second `Preload` is a no-op | `session_bus` (3 tests) | pass |
| 9 | `settingsd` restart / absent | The Settings shell tolerates an absent `settingsd`; published values re-sync when it returns | `tst_settings_absence`, `tst_wallpaperpolicy`, `tst_settings_live` | pass |
| 10 | Portal absent | Only the Custom (FileChooser portal) row is disabled; the shipped default and built-in rows stay live without an error | `tst_settings_absence::test_wallpaper_pane_disables_only_the_absent_portal_row` | pass |
| 11 | Package artifact | The only bundled image is the shipped original `Default.jpg`; fetched images are never packaged | `make install DESTDIR=… PREFIX=/usr` | only `share/dragonfruit/wallpapers/Default.jpg`, byte-identical to the asset |

## Live halves (real network / real gallery)

These are the parts the headless path cannot claim; they are recorded from the
live capture (`make t18-wallpaper-capture` on a host Wayland session).

- **Cold cache + dead network → shipped default renders.**
  `docs/captures/t18-wallpaper-offline.png`: the nested demo opened at the
  Wallpaper pane with a scratch-cache provider behind a dead proxy
  (`HTTPS_PROXY=http://127.0.0.1:1`, observed `status: offline`). The desktop
  background region (left 420 px strip, away from the Settings window) is a
  rich photograph — 172,128 unique colours, luminance σ = 70.4 — identical to
  the `fetching` still captured before any successful fetch, i.e. the shipped
  `Default.jpg`, not a solid colour. The Featured row region (820,430)–(1280,560)
  is uniform (343 unique colours), the "Featured pictures will be available
  soon." note.
- **Open pane triggers `Preload` and the first-run fill.**
  `docs/captures/t18-wallpaper-fetching.png` (provider held in `fetching` by a
  hanging CONNECT proxy) shows the Featured skeletons; restarting the provider
  without the proxy reaches `status: ready` and
  `docs/captures/t18-wallpaper-filled.png` shows the filled Featured row with
  attribution. The Featured region then has 43,972 unique colours (photographic
  tiles vs. 343 in the offline still).
- **A fetched wallpaper is applied live.** The capture selects the provider's
  deterministic Nature default (`wallpaper.source` → cached local path) and the
  filled still's desktop background is the fetched picture (138,117 unique
  colours). The provider is not required for the desktop to keep rendering it:
  the compositor holds the local path, and the shipped default remains the
  fallback.

The vision model was rate-limited (HTTP 429) for this run; the pixel statistics
above are the evidence, the same substitution T-18.1b recorded.

## Verdict

All eleven states behave as designed. The shipped original default is the
cold-cache, absent-network, and absent-provider background; fetched content is
cache-only, attributed, and never bundled. Deferred to T-17: human visual-floor
sign-off.