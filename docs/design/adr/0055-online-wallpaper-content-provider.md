# 0055 — Wallpaper content is fetched at runtime from Wikimedia Commons, never bundled

## Status

accepted

## Context

T-09.3 gave the Wallpaper pane per-Space image selection and fit, backed by
`wallpaper.source` (settingsd) and the compositor's image-wallpaper path
(T-05.4). The shipped collections are original generated gradients plus the
portal "Add Photo…" chooser. There is no online content source and no HTTP
client anywhere in the tree.

The product wants a first-run desktop background set that is rich and varied
without shipping (or copying) anyone else's artwork. The macOS reference
("Dynamic Wallpapers", "Landscape", "Cityscape") is interaction-model only; its
bitmap output must never be reproduced ([14-risks.md](../14-risks.md)). Wikimedia
Commons Featured Pictures is a permissively-licensed, attributed corpus we can
fetch instead of bundling.

## Decision

- **Fetch, never bundle.** A session service `services/wallpaperd` downloads
  Featured Pictures at first run and re-checks roughly weekly. No image is
  shipped in the package; the cache lives under `$XDG_CACHE_HOME/dragonfruit/`.
- **Six Dragonfruit categories, one Commons category each** (the values were
  validated against the live API):

  | Dragonfruit | Wikimedia category |
  |---|---|
  | Nature | `Category:Featured pictures of nature` |
  | Water | `Category:Featured pictures of bodies of water` |
  | Cityscapes | `Category:Featured pictures of cityscapes` |
  | Earth | `Category:Featured pictures of Earth from space` |
  | Scenery | `Category:Featured pictures of landscapes` |
  | Underwater | `Category:Featured pictures taken underwater` |

- **One query shape.** `action=query`, `generator=categorymembers`,
  `gcmtitle=<category>`, `gcmnamespace=6`, `gcmlimit=10`,
  `gcmsort=timestamp`, `gcmdir=desc`, `prop=imageinfo`,
  `iiprop=url|size|mime|extmetadata`, `iiurlwidth=3840`, `format=json`,
  `formatversion=2`, following `continue`. `gcmtype=file` alone is not trusted
  (it returned `Category:` pages); post-filter `ns==6` and `mime^=image/`.
  Strip the `utm_*` query Wikimedia appends to `url`/`thumburl` before caching.
- **Deterministic default.** The out-of-box wallpaper is the first entry of
  `Category:Featured pictures of nature` under timestamp-desc, downloaded at
  3840 px, when the user has not chosen a wallpaper. "Top naturescape" is this
  rule, not a heuristic — it is reproducible in a headless test from a fixture.
- **User choice wins.** `wallpaper.source` keeps its meaning (a user-chosen
  path, owned by `apps/settings`). The provider publishes a separate resolved
  default under new additive keys (`wallpaper.provider*`, owned by
  `wallpaperd`). The shell's effective source is `wallpaper.source` when
  non-empty, otherwise the provider source. Nothing overwrites a user choice.
- **Absence is normal.** No network, no API, or no `settingsd` must never block
  the desktop: keep the cache, keep the compositor's solid-color fallback, show
  the pane's offline/empty state.
- **Attribution is mandatory.** The provider stores `Artist`, `LicenseShortName`,
  `LicenseUrl`, the file page URL, and the description; the pane displays them.
  Licenses span public domain, CC BY, CC BY-SA, and FAL; share-alike/FAL images
  are never represented as original Dragonfruit work.
- **Placeholder while fetching.** Before the first catalogue arrives, Featured
  tiles render as grey slow-animated-gradient (shimmer) rectangles with a
  reduced-motion static variant and an accessible name.

## Consequences

- The provider is a new restartable service with a new permissively-licensed
  HTTP dependency; `df-ipc` stays dependency-free and no copyleft dependency is
  added ([licensing.md](../../licensing.md)).
- The compositor and `df_workspace.set_wallpaper` are unchanged: the provider
  produces a local path, exactly like the portal chooser does today.
- The pane gains three source rows (Featured / Built-in / Custom) and a
  design-system skeleton component with a gallery golden.
- T-16's i18n/a11y sweep covers the new copy; T-17's visual floor validates the
  real first-run default instead of a gradient stand-in.
- Attribution and license links become user-visible product surface, so any
  future provider must supply the same metadata contract.