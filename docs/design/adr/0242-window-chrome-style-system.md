# 0242 — Window chrome is a named style table shared by QML and the compositor

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-08 |
| **Release** | v0.2.0 |
| **Source** | `docs/design/23-macos-structural-surface-review.md` §2, `docs/design/05-window-decorations.md` |
| **Supersedes** | the single `component.titlebar.height` assumption |

## Context

The 2026-11-08 structural review measured a 0/32/40/48/52/66/88 pt window
chrome taxonomy. The tree had one 40 pt `component.titlebar.height` token used
by the QML `TitleBar` and the compositor SSD renderer, and first-party apps
hard-coded their own layout around it. That made a 32 pt utility window, a
52 pt unified toolbar, and an 88 pt preference window impossible to express
without per-app constants.

## Decision

1. `component.windowStyle` is the one source: each named style
   (`borderless`, `titlebar`, `unifiedCompact`, `expanded`, `unified`,
   `preference`) carries a baseline height plus its `titleRow`/`toolbarRow`
   composition.
2. The final height is derived, not hard-coded:
   `chromeHeight(style, { labelledToolbar, toolbarRowHeight })`. The 66 pt
   icon-and-label toolbar is `unified` + the 14 pt labelled row; an
   app-specific row can make an expanded document 70 pt.
3. `WindowStyle` resolves the table in QML; the compositor has the matching
   `WindowStyle`/`WindowChrome` types generated from the same tokens. A
   first-party `TitleBar` inside an `AppWindow` inherits the window's style.
4. Traffic lights are 16×16 pt, 23 pt centre-to-centre, with a leading inset
   token. The top inset is derived from the final height
   (`(height - diameter) / 2`): 8 pt in a 32 pt bar, 18 pt in a 52 pt bar.
5. Tab/path/status/format bars are content-layer accessory bars
   (`component.accessoryBar`: 28/30/28/20 pt), never part of chrome height.
6. The compositor's SSD default is `titlebar` (32 pt); it draws no toolbar.
   The style can be set per window through the window model and the synthetic
   test seam. The tier-3 dignity rule is unchanged: no second titlebar over a
   CSD client.
7. Fullscreen reveal uses the window's style height. Long-title truncation
   beyond the existing middle elision and RTL titlebar mirroring remain
   `OPEN` (`N-WIN-29`/`N-WIN-30`).

## Consequences

- First-party apps name a style instead of carrying a height. Settings and
  Files are 52 pt `unified` windows; their main toolbar is the titlebar row,
  so the window is not a 40 + 52 pt stack.
- The compositor and QML cannot drift: the same generated table drives both,
  and tests assert every baseline, the 8/18 pt insets, the 23 pt spacing, the
  fullscreen reveal, and that accessory bars do not change chrome height.
- Changing a style at runtime reflows a zoomed client through the existing
  inset/configure path.
- `_NET_FRAME_EXTENTS` remains a separate X11 gap (G-16).
