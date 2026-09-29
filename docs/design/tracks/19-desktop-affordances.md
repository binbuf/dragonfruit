# T-19 — Desktop affordances: icon system, Apps drawer, desktop items

> **Track, not a single slice.** This file is the design reference. It is
> executed as 6 one-session tasks:
> [T-19.1a](../../tasks/176a-t-19.1a-phosphor-vendor-and-icon-primitive.md) ·
> [T-19.1b](../../tasks/176b-t-19.1b-system-settings-category-style.md) ·
> [T-19.1c](../../tasks/176c-t-19.1c-menu-bar-icon-migration.md) ·
> [T-19.1d](../../tasks/176d-t-19.1d-dock-and-app-icon-migration.md) ·
> [T-19.2](../../tasks/177-t-19.2-apps-drawer.md) ·
> [T-19.3](../../tasks/178-t-19.3-desktop-items-and-mouse-selection.md). Strict
> order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).
>
> This is a **new** track id. Legacy "T-19" (desktop icons) is absorbed here as
> one unit — see the numbering note in [ROADMAP.md](../../ROADMAP.md#legacy-mapping).

| | |
|---|---|
| **Track** | 19 of 19 — the desktop you can actually inhabit |
| **Area** | `assets/icons/`, `design-system/`, `shell/`, `compositor/`, `apps/` |
| **Depends on** | the inherited foundation (private shell protocols, design system, Files `files-core`) |
| **Blocks** | — |

## Why now

The premium gate (T-17) verifies the *window loop*; it does not give the user a
place to **start** from. On a fresh login the desktop is a wallpaper, a menu
bar, and a Dock. macOS's first-use feel comes from three things we explicitly
deferred to the post-gate backlog and that are now cheap enough to pull forward:

1. **A consistent icon language** — Phosphor for foreground glyphs, with the
   Dragonfruit-specific container where it belongs.
2. **An Applications drawer** — scan everything installed and lay it out
   alphabetically, Launchpad-style, as the primary way to start an app.
3. **Files owned by the desktop** — `~/Desktop` as an icon view with normal
   mouse selection, so the desktop is a workspace, not a picture.

These run before the real-session bring-up (Phase 16.5) so the first login on
real hardware already feels like a desktop.

## T-19.1 — The Phosphor icon language

We adopt the **Phosphor** icon set (MIT) for *foreground glyphs*, and keep the
Dragonfruit-specific container in QML. Glyph and styling stay separate so the
theme, dark/light, sizing, and the icon set itself can change independently.

Why a third-party glyph set at all: the inherited [`Icon.qml`](../10-design-system.md)
draws original geometry for a small, fixed vocabulary; it is the right call for
a handful of chrome marks, but hand-drawing a large, consistent vocabulary does
not scale. Phosphor is permissively licensed and its `fill` weight carries the
visual weight the reference has. The [original-assets rule](../14-risks.md)
forbids Apple's assets, not third-party open assets; the decision is recorded in
an ADR (T-19.1a).

T-19.1a vendors the pinned `regular` and `fill` weights (Phosphor 2.0.8) and
renders them through `PhosphorIcon`, a tintable `ShapePath` primitive. A glyph
is a *resource* with no baked color or container; the tint, size, and any tile
live in QML, and a referenced glyph that is not vendored fails the build
(`make check-phosphor`).

The icon language is one foundation (T-19.1a) and three migrations:

- **T-19.1a** vendors Phosphor, exposes it to QML, and adds the `PhosphorIcon`
  primitive + ADR.
- **T-19.1b** adds `SettingsCategoryIcon` — the rounded, gradient-backed tile
  with an inner highlight, drop shadow, and near-white glyph — and uses it for
  **System Settings only**. Everywhere else a migrated icon is a plain glyph
  sized for its place. The component is category-agnostic; the
  category → (glyph, gradient) table lives in `SettingsPanes.categoryStyles`
  and a pane without a mapping keeps the original `Icon` glyph (ADR
  [0164](../adr/0164-settings-category-tile.md)).
- **T-19.1c** migrates the menu-bar status marks (`shell/menubar/StatusGlyph.qml`)
  to plain Phosphor glyphs, preserving every state variant and the battery
  level. The battery composes the Phosphor `battery-empty` outline with a
  token level fill overlay inside the cell (ADR
  [0165](../adr/0165-menubar-phosphor-marks.md)).
- **T-19.1d** gives our first-party apps Phosphor artwork: the Dock's **Files**
  (our Finder) tile and the **System Settings** app icon (which carries the
  gradient container). Each app ships one SVG generated from Phosphor
  (`assets/icons/apps/org.dragonfruit.{Files,Settings}.svg`) that is both the
  bundled QML resource (`qrc:/icons/apps/…`) and the installed
  `share/icons/hicolor/scalable/apps/` file, with the `.desktop` `Icon=` naming
  the same id; `DockGlyph` maps the desktop id to the bundled tile, keeping
  app-index's themed `iconPath` as the third-party fallback (ADR
  [0166](../adr/0166-first-party-app-icons.md)). The Dragonfruit system-menu
  logo (`DragonfruitLogo.qml`) is kept; the Dock's folder-stack, Trash, and
  overflow artwork are untouched.

## T-19.2 — The Applications drawer

The Dock's **Add Application** picker
([04-shell.md](../04-shell.md#adding-and-removing-apps)) is a management
surface, not a launcher; Launchpad and Spotlight-equivalent search were left
post-gate. This track adds the **launcher**: a full-screen `Applications`
overlay fed by the app-index corpus, alphabetical by default, with a search
field, category filter pills, and a 7-column icon grid of themed app icons
(including the T-19.1d first-party icons). Activation uses the same launch path
as the Dock (`buildLaunchCommand` + `appLaunchEnvironment` + launch origin). The
Spotlight-equivalent *search* pane remains deferred; the drawer's search field
filters its own list only.

**Built (T-19.2).** The pure `buildAppsDrawerList` helper
(`shell/src/appsdrawer.{h,cpp}`) owns the sort/dedupe/category-mapping rules;
the `Dragonfruit.AppsDrawer` QML module renders the overlay; a new full-output
`apps-drawer` layer surface follows `createOverviewSurface`. Activation reuses
the Dock's launch path, and app-index absence renders an explicit row. One
greppable freedesktop → pill mapping lives in `appsdrawer.cpp`'s
`kCategoryMap`. See [ADR 0167](../adr/0167-applications-drawer.md).

## T-19.3 — Files-owned desktop items

The macOS model: **the file manager owns the desktop.** `~/Desktop` is an icon
view with the same core and semantics as a Files window, running as its own
process (`dragonfruit-files --desktop`) on a compositor **desktop-layer**
surface admitted by launch token — the shell and this surface, nothing else
([02-compositor.md](../02-compositor.md#private-shell-protocols),
[09-files.md](../09-files.md#desktop-icons-owned-by-files)). The compositor
currently composites only the `top` and `overlay` chrome layers and renders its
own wallpaper; the desktop layer lands between them. This track's first slice
covers rendering, mouse selection (click, Cmd/Shift multi-select, rubber-band),
and open — mutations, inline rename, drag-to-Trash, spring-loading, and Desktop
Reveal icon exposure are follow-ups.

**Built (T-19.3).** The compositor half landed: the `background` layer
composites between the wallpaper and windows, layer creation is role-scoped
(`desktop-icons` owns `background`, the shell owns `top`/`overlay`), the
desktop's token is provisioned via `desktop:`-tagged
`DRAGONFRUIT_LAUNCH_TOKENS` / `DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN`, and
hit-testing/focus follow the paint order (ADR
[0168](../adr/0168-desktop-layer-and-role-scoped-layers.md)). The process
landed: `dragonfruit-files --desktop` is a self-contained private-protocol
client rendering the `DesktopSurface` QML offscreen onto that layer, with
click/Cmd/Shift/rubber-band selection, double-click open, and an
`Open in Files` / `New Folder` background menu (ADR
[0169](../adr/0169-desktop-process-client.md)). `FilesArguments` parses
`--desktop` and resolves `~/Desktop` via xdg-user-dirs; the dev tool provisions
the token and launches it, and a conformance test kills the desktop client and
asserts the shell/browser/session survive.
**Remaining:** systemd session-manager provisioning of the desktop token, the
live nested capture, and the deferred desktop operations (rename,
drag-to-Trash, Reveal) — see the T-19.3 hand-off.

## Reference UI/UX

Local captures only, and they never ship:

- [macos/AppDrawer.md](../../reference/macos/AppDrawer.md) — the Applications
  grid: title, search, category pills, 7-column tiles, truncated labels, and the
  gradient category-icon language.
- [macos/Desktop1.md](../../reference/macos/Desktop1.md) — desktop items,
  label under the icon, free grid placement.
- [macos-ui-inventory.md](../../reference/macos-ui-inventory.md) — distilled
  summaries ("Applications grid (Launchpad)", "Desktop").

## Acceptance

- [ ] Phosphor is vendored, licensed, reachable from QML, and the ADR is
      recorded.
- [ ] Menu-bar marks, the Files Dock tile, and the System Settings app icon use
      Phosphor; the System Settings category tiles use the gradient container;
      the Dragonfruit logo is unchanged.
- [ ] The Applications drawer lists every launchable app alphabetically,
      filters by category and query, launches on activate, and degrades
      explicitly when app-index is absent.
- [ ] `~/Desktop` renders on the desktop layer with mouse selection
      (click, Cmd/Shift, rubber-band) and open; killing the Files desktop does
      not disturb the browser or the session.
- [ ] `make e2e` stays green and each unit commits a nested capture.