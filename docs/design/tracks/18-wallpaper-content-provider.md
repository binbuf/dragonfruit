# T-18 — Wallpaper content provider (Wikimedia Featured Pictures)

> **Track, not a single slice.** This file is the design reference. It is
> executed as 4 one-session tasks: [T-18.1a](../../tasks/172-t-18.1a-wallpaper-provider-service.md)
> · [T-18.1b](../../tasks/173-t-18.1b-provider-settings-and-default-source.md)
> · [T-18.2](../../tasks/174-t-18.2-wallpaper-pane-collections-and-skeleton.md)
> · [T-18.3](../../tasks/175-t-18.3-provider-licensing-and-capture.md).
> Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 18 of 18 — live wallpaper content |
| **Area** | `services/wallpaperd/` · `services/settingsd/` (keys) · `shell/` (wallpaper forwarder) · `apps/settings/` (Wallpaper pane) · `design-system/` (skeleton) |
| **Depends on** | T-09 (Wallpaper pane, settings keys, shell forwarder) · T-13 (portal chooser for the Custom row; degrades until it lands) |
| **Blocks** | T-17 (the shipped default background is part of the visual floor) |
| **Design detail** | ADR [0055](../adr/0055-online-wallpaper-content-provider.md) · [08-settings.md](../08-settings.md) · [02-compositor.md](../02-compositor.md) · [licensing.md](../../licensing.md) |

## Demo

```
first run, no wallpaper configured yet
→ open Wallpaper: the Featured rows are grey slow-shimmer placeholders
→ within seconds the tiles fill with Wikimedia Commons Featured Pictures
  across six Dragonfruit categories
→ the out-of-box wallpaper is the top Nature photo; attribution is visible
→ pick another tile; the Space changes live and persists
→ unplug / block the network: cached images still work; a cold cache keeps the
  solid-color fallback and the pane says the pictures will arrive later
→ the Built-in and Custom rows behave exactly as before
```

Capture: `docs/captures/t18-wallpaper.*`.

## Why now

The desktop background is the first thing a user sees, so the *shipped default*
is part of the T-17 visual floor. This track lands after the Wallpaper pane
(T-09.3, done) and before T-16's accessibility/i18n sweep, so the gate validates
the real first-run experience and the attribution/skeleton copy is translated
and accessible rather than retrofitted.

It is **not** an Apple-artwork track: nothing is bundled, nothing is copied from
macOS. Images are fetched at runtime from Wikimedia Commons under their own
licenses, never shipped in the package (ADR [0055](../adr/0055-online-wallpaper-content-provider.md)
and [licensing.md](../../licensing.md)).

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

1. **Provider service** (`services/wallpaperd`): query the six Wikimedia
   Commons featured-picture categories, filter to bitmap files, download
   3840-px thumbnails, cache them and their attribution metadata locally,
   fetch on first run and re-check roughly weekly, degrade cleanly offline,
   expose the catalogue over D-Bus.
2. **Deterministic default**: the top Nature photo becomes the out-of-box
   wallpaper when the user has not chosen one.
3. **Settings + shell wiring**: additive keys and an effective-source rule that
   keeps a user choice authoritative over the fetched default.
4. **Wallpaper pane content**: Featured / Built-in / Custom rows, the
   downloading skeleton, attribution display, and empty/offline/error states.
5. **Licensing and acceptance**: the fetched-content policy, the absent-network
   matrix, i18n/a11y hand-off, the capture, and the track sign-off.

### Out / explicitly deferred

- Favorites, user accounts, and cross-device wallpaper sync (post-gate).
- Video/dynamic/live wallpapers (post-gate).
- Additional content providers beyond Wikimedia (the provider interface is left
  open so one can be added later).
- Bundling any image in the package — never (ADR 0055).

## Acceptance

- [ ] First run with an empty cache populates Featured from Wikimedia and sets
      the default to the top Nature photo.
- [ ] The pane shows shimmer placeholders while fetching; reduced motion makes
      them static.
- [ ] Cached wallpapers work with the network absent; a cold cache keeps the
      solid-color fallback and does not block the session.
- [ ] Attribution (artist + license, linked) is visible for every fetched image.
- [ ] A user-chosen wallpaper always wins over the fetched default and persists.
- [ ] The demo runs and the capture is committed; `make e2e`/`make check` stay
      green.

## Test plan

- Headless: fixture the Commons JSON and assert the query builder, filtering,
  dedup, URL normalization, cache layout, default selection, and offline path.
- QML: pane states (fetching/ready/offline/error), selection, a11y names,
  reduced-motion skeleton; gallery golden for the skeleton component.
- Absent-daemon matrix: no network, no `settingsd`, no portal.
- Nested capture of the real first-run fill and an offline pane.

## Risks

- **Commons response shape or licensing can change.** Pin the parser behind a
  fixture and keep the category map in one constant.
- **Network at first run is not guaranteed.** Absence is a normal state: keep
  the cache, keep the gradient fallback, never block the desktop.
- **License diversity** (public domain, CC BY, CC BY-SA, FAL). Attribution is
  mandatory and share-alike content must not be presented as our own.
- **Download volume.** Cap per-category count, concurrency, and cache size; the
  compositor must never fetch on the frame path.

## Hand-off

- Feeds T-16 (a11y/i18n) and is validated by T-17's visual floor.