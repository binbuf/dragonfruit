# T-18 — Wallpaper content provider (shipped default + Wikimedia Featured Pictures)

> **Track, not a single slice.** This file is the design reference. It is
> executed as 4 one-session tasks: [T-18.1a](../../tasks/172-t-18.1a-wallpaper-provider-service.md)
> · [T-18.1b](../../tasks/173-t-18.1b-provider-settings-and-default-source.md)
> · [T-18.2](../../tasks/174-t-18.2-wallpaper-pane-collections-and-skeleton.md)
> · [T-18.3](../../tasks/175-t-18.3-provider-licensing-and-capture.md).
> Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md); this
> track runs immediately after the Dock experience addendum (T-14.7a–k).

| | |
|---|---|
| **Slice** | 18 of 18 — live wallpaper content |
| **Area** | `services/wallpaperd/` · `services/settingsd/` (keys) · `shell/` (wallpaper forwarder) · `apps/settings/` (Wallpaper pane) · `design-system/` (skeleton) · `assets/graphics/wallpapers/` (shipped default) |
| **Depends on** | T-09 (Wallpaper pane, settings keys, shell forwarder) · T-13 (portal chooser for the Custom row) |
| **Blocks** | T-16 (a11y/i18n sweep) · T-17 (the shipped default background is part of the visual floor) |
| **Design detail** | ADRs [0055](../adr/0055-online-wallpaper-content-provider.md) / [0094](../adr/0094-bundled-default-wallpaper-and-lazy-cache.md) · [08-settings.md](../08-settings.md) · [02-compositor.md](../02-compositor.md) · [licensing.md](../../licensing.md) |

## Demo

```
first run, empty cache, no network
→ the desktop is already the shipped original default
   (assets/graphics/wallpapers/Default.jpg) — no wait, no fetch
→ open Wallpaper: the pane requests an eager Preload; Featured rows show grey
   slow-shimmer placeholders
→ within seconds the tiles fill with Wikimedia Commons Featured Pictures
   across six Dragonfruit categories; attribution is visible
→ pick another tile; the Space changes live and persists
→ unplug / block the network: the cached/shipped image still works; a cold
   cache keeps the shipped default and the pane says the pictures will arrive
   later
→ the Built-in row offers the shipped default plus the original gradients; the
   Custom row behaves exactly as before
```

Capture: `docs/captures/t18-wallpaper.*`.

## Why now

The desktop background is the first thing a user sees, so the *shipped default*
is part of the T-17 visual floor. The track was moved forward to run
immediately after the Dock experience addendum (T-14.7a–k) and before the T-15
breadth phase, so the first-run background and its API/cache wiring settle
before T-15 and T-16 build on the desktop, and T-16's accessibility/i18n sweep
and T-17's visual floor validate the real first-run experience rather than a
gradient stand-in.

It is **not** an Apple-artwork track: the shipped default is our own original
asset and nothing is copied from macOS. The Featured pictures are fetched at
runtime from Wikimedia Commons under their own licenses and are never shipped in
the package (ADR [0055](../adr/0055-online-wallpaper-content-provider.md) /
[0094](../adr/0094-bundled-default-wallpaper-and-lazy-cache.md) and
[licensing.md](../../licensing.md)).

## Inherited and reused

- The Wallpaper pane and per-Space selection model (T-09.3).
- `settingsd`'s additive key schema and the shell's `wallpaperpolicy`
  forwarder (T-08/T-09).
- The compositor's image-wallpaper decode/cache/slide path (T-05.4) — the
  provider only supplies a local file path; the compositor is unchanged.
- The adapter/absent-daemon discipline from T-07 and the Settings app shell
  from T-09.
- Design-system components, tokens, and gallery goldens.

## Scope

### In

1. **Shipped default**: install and resolve the original
   `assets/graphics/wallpapers/Default.jpg` as the out-of-box, cold-cache, and
   offline background (ADR 0094).
2. **Provider service** (`services/wallpaperd`): query the six Wikimedia
   Commons featured-picture categories, filter to bitmap files, download
   3840-px thumbnails, cache them and their attribution metadata locally,
   fetch on first run and re-check roughly weekly, degrade cleanly offline,
   expose the catalogue over D-Bus.
3. **Cache policy**: warm lazily in the background on session launch (off the
   compositor/shell frame path) and eagerly via `Preload` when the Wallpapers
   pane is open.
4. **Settings + shell wiring**: additive keys and an effective-source
   precedence — user choice, then the shipped default, then the fetched
   fallback — exposed to System Settings through `SettingsBridge`.
5. **Wallpaper pane content**: Featured / Built-in / Custom rows (Built-in
   includes the shipped default), the downloading skeleton, attribution
   display, and empty/offline/error states.
6. **Licensing and acceptance**: the shipped-vs-fetched policy, the
   absent-network/preload matrix, i18n/a11y hand-off, the capture, and the
   track sign-off.

### Out / explicitly deferred

- Favorites, user accounts, and cross-device wallpaper sync (post-gate).
- Video/dynamic/live wallpapers (post-gate).
- Additional content providers beyond Wikimedia (the provider interface is left
  open so one can be added later).
- Bundling any *fetched/third-party* image in the package — never (ADR 0055).
  The original shipped default is intended and licensed (ADR 0094).

## Acceptance

- [x] The out-of-box, cold-cache, and offline background is the shipped original
      default, with no network dependency at first run.
- [x] The provider warms lazily on launch and eagerly when the Wallpapers pane
      is opened; neither path blocks the session.
- [x] First run with an empty cache populates Featured from Wikimedia; the
      deterministic top Nature photo is the Featured default/fallback.
- [x] The pane shows shimmer placeholders while fetching; reduced motion makes
      them static.
- [x] Cached wallpapers work with the network absent; a cold cache keeps the
      shipped default and does not block the session.
- [x] Attribution (artist + license, linked) is visible for every fetched image.
- [x] A user-chosen wallpaper always wins over the shipped and fetched defaults
      and persists.
- [x] The demo runs and the capture is committed; `make e2e`/`make check` stay
      green.

Evidence: the T-18.3 reviewed matrix
[`../../captures/t18-absence-matrix.md`](../../captures/t18-absence-matrix.md)
(cold/warm/offline/restart/weekly/error/`Preload`/portal/package) with its
headless transcript, and the capture stills
`t18-wallpaper-{offline,fetching,filled}.png`.

## Test plan

- Headless: fixture the Commons JSON and assert the query builder, filtering,
  dedup, URL normalization, cache layout, Featured-default selection, the
  offline path, shipped-default resolution/precedence, and lazy-vs-`Preload`
  behavior.
- QML: pane states (fetching/ready/offline/error), selection, a11y names,
  reduced-motion skeleton, the shipped default tile, and the `Preload` on pane
  open; gallery golden for the skeleton component.
- Absent-daemon matrix: no network, no `settingsd`, no portal.
- Nested capture of the real first-run fill and an offline pane.

## Risks

- **Commons response shape or licensing can change.** Pin the parser behind a
  fixture and keep the category map in one constant.
- **Network at first run is not guaranteed.** Absence is a normal state: the
  shipped default renders immediately, the cache persists, and the desktop never
  blocks.
- **License diversity** (public domain, CC BY, CC BY-SA, FAL). Attribution is
  mandatory and share-alike content must not be presented as our own. The
  shipped default is the only bundled image and is our own.
- **Download volume.** Cap per-category count, concurrency, and cache size; the
  compositor must never fetch on the frame path, and the lazy launch warm must
  not compete with bring-up.

## Hand-off

- Feeds T-16 (a11y/i18n) and is validated by T-17's visual floor.

### T-16.7 (localization) — strings the sweep must cover

All in `apps/settings/WallpaperPane.qml` (new in T-18.2) unless noted:

- Section titles: "Featured", "Built-in", "Custom".
- Empty/offline/error note: "Featured pictures will be available soon."
- Skeleton accessible label: "Downloading wallpapers"
  (`design-system/components/Skeleton.qml`; callers pass `accessibleName`).
- Attribution row: "Photo by %1", "License: %1", "View file page".
- Existing copy touched by the row split: "Current wallpaper", "Default",
  "Solid color", "Using %1", "Add Photo…", "Add wallpaper photo", "Choose an
  image file for this Space.", "No file chooser is available.", "Show on all
  Spaces", "Apply this wallpaper to every Space.", and the fit labels
  Fill/Fit/Stretch/Center.
- The plugin/category labels are capitalized from a lowercase provider slug at
  runtime, not translated words; a future provider must localize its own.

### T-16.6 (AT-SPI / keyboard) — surfaces to audit

- `Skeleton.qml` is `Accessible.Graphic` and is `Accessible.ignored` when it
  has no `accessibleName`; the Featured row passes "Downloading wallpapers".
- Featured tiles are `Accessible.RadioButton` (checkable, focusable) with a
  category-derived name; they are keyboard-selectable and the `Preload` on pane
  open is not required for keyboard use.
- Attribution links are `Accessible.Link` with press actions; verify they are
  reachable and operable keyboard-only (the panel only appears for fetched
  items, so T-16.6 needs a fixture or a warm cache to see it).
- Built-in and Custom tiles keep their existing a11y roles; the shipped
  `Default` tile's name is the translated "Default".

Deferred to T-17: human visual-floor sign-off on the shipped default
background (batched at the track boundary).