# T-19 — Desktop affordances: icon system, Apps drawer, desktop items

> **Track, not a single slice.** This file is the design reference. It is
> executed as 3 one-session tasks: [T-19.1](../../tasks/176-t-19.1-phosphor-icons-and-settings-category-icon.md) ·
> [T-19.2](../../tasks/177-t-19.2-apps-drawer.md) ·
> [T-19.3](../../tasks/178-t-19.3-desktop-items-and-mouse-selection.md). Strict
> order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).
>
> This is a **new** track id. Legacy "T-19" (desktop icons) is absorbed here as
> one unit — see the numbering note in [ROADMAP.md](../../ROADMAP.md#legacy-mapping).

| | |
|---|---|
| **Track** | 19 of 19 — the desktop you can actually inhabit |
| **Area** | `assets/icons/`, `design-system/`, `shell/`, `compositor/`, `apps/files/` |
| **Depends on** | the inherited foundation (private shell protocols, design system, Files `files-core`) |
| **Blocks** | — |

## Why now

The premium gate (T-17) verifies the *window loop*; it does not give the user a
place to **start** from. On a fresh login the desktop is a wallpaper, a menu
bar, and a Dock. macOS's first-use feel comes from three affordances we
explicitly deferred to the post-gate backlog and that are now cheap enough to
pull forward:

1. **A category icon language** for Settings (and later the drawer) — the
   rounded, gradient-backed tile that makes a settings list read at a glance.
2. **An Applications drawer** — scan everything installed and lay it out
   alphabetically, Launchpad-style, as the primary way to start an app.
3. **Files owned by the desktop** — `~/Desktop` as an icon view with normal
   mouse selection, so the desktop is a workspace, not a picture.

These run before the real-session bring-up (Phase 16.5) so the first login on
real hardware already feels like a desktop.

## T-19.1 — Phosphor-backed category icons

We adopt the **Phosphor** icon set (MIT) for *foreground glyphs only*, and keep
the Dragonfruit-specific container — rounded tile, category gradient, inner
highlight, drop shadow — in QML. Glyph and styling stay separate so the theme,
dark/light, sizing, and the icon set itself can change independently.

Why a third-party glyph set at all: the inherited [`Icon.qml`](../10-design-system.md)
draws original geometry for a small, fixed vocabulary; it is the right call for
menu-bar and Dock marks, but hand-drawing a large, consistent category
vocabulary does not scale. Phosphor is permissively licensed and its `fill` /
`duotone` weights carry the visual weight the reference has. The
[original-assets rule](../14-risks.md) forbids Apple's assets, not third-party
open assets; the decision to vendor an icon set is recorded in an ADR by the
unit.

## T-19.2 — The Applications drawer

The Dock's **Add Application** picker
([04-shell.md](../04-shell.md#adding-and-removing-apps)) is a management
surface, not a launcher; Launchpad and Spotlight-equivalent search were left
post-gate. This track adds the **launcher**: a full-screen `Applications`
overlay fed by the app-index corpus, alphabetical by default, with a search
field, category filter pills, and a 7-column icon grid of themed app icons.
Activation uses the same launch path as the Dock (`buildLaunchCommand` +
`appLaunchEnvironment` + launch origin). The Spotlight-equivalent *search* pane
remains deferred; the drawer's search field filters its own list only.

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

## Reference UI/UX

Local captures only, and they never ship:

- [macos/AppDrawer.md](../../reference/macos/AppDrawer.md) — the Applications
  grid: title, search, category pills, 7-column tiles, truncated labels.
- [macos/Desktop1.md](../../reference/macos/Desktop1.md) — desktop items,
  label under the icon, free grid placement.
- [macos-ui-inventory.md](../../reference/macos-ui-inventory.md) — distilled
  summaries ("Applications grid (Launchpad)", "Desktop").

## Acceptance

- [ ] Phosphor is vendored, licensed, and reachable from QML; a category icon
      renders as container + gradient + glyph and is used in Settings.
- [ ] The Applications drawer lists every launchable app alphabetically,
      filters by category and query, launches on activate, and degrades
      explicitly when app-index is absent.
- [ ] `~/Desktop` renders on the desktop layer with mouse selection
      (click, Cmd/Shift, rubber-band) and open; killing the Files desktop does
      not disturb the browser or the session.
- [ ] `make e2e` stays green and each unit commits a nested capture.