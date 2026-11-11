# 0167 — The Applications drawer: a pure app-index list helper, a full-output launcher overlay, and one category mapping

## Status

accepted

## Context

Track 19's launcher (T-19.2) is the shell's primary "start an app" affordance.
The Dock's Add Application picker (ADR
[0090](0090-dock-app-management-picker-and-drops.md)) is a management surface,
not a launcher. The drawer must list every launchable app the app-index corpus
knows about, alphabetically, filter by freedesktop category and a search query,
render a fixed-size tile grid, and launch through the same path as the Dock.

Three decisions have to be made once, because later tasks (the deferred
Spotlight-equivalent search pane) will build on them:

1. Where the filtering/category mapping lives.
2. How the overlay surface is created and toggled.
3. What "the freedesktop `Categories` list" maps to as pills.

## Decision

- **The list is a pure helper, not QML logic.** `buildAppsDrawerList(entries,
  category, query)` and its companions (`appsDrawerCategoryKeys`,
  `appsDrawerCategoryFor`, `appsDrawerCategoriesFor`,
  `appsDrawerPresentCategories`) live in `shell/src/appsdrawer.{h,cpp}`, in the
  Wayland-free `dragonfruit-shell-dockcore` library. It drops `noDisplay` and
  non-launchable records, de-dupes by desktop id (first wins), maps categories,
  sorts by localized name with a desktop-id tiebreak, and filters by category
  and query. This mirrors `buildAppPickerList` (ADR
  [0090](0090-dock-app-management-picker-and-drops.md)) but **drops the pin
  semantics**: the drawer starts apps, it does not manage Dock membership. The
  view narrows the pushed list by the same predicates for its live search,
  exactly as `DockAppPicker` does.
- **One greppable freedesktop → pill mapping.** `appsdrawer.cpp`'s
  `kCategoryMap` maps each freedesktop main category to one canonical key and
  drops unknown and `GTK`/`Qt` implementation categories. The canonical keys are
  ordered by `appsDrawerCategoryKeys()`; `"all"` is the implicit first pill and
  is never a data category. The display labels live in `AppsDrawer.qml` (so they
  are translatable): `developer-tools`, `productivity`, `utilities`,
  `entertainment`, `games`, `social`, `creativity`, `information`.
- **The overlay is a new full-output layer surface following the overview.** 
  `ShellProtocol::createAppsDrawerSurface()` creates a `df_shell` layer surface
  on `DF_SHELL_LAYER_OVERLAY`, namespace `"apps-drawer"`, full-output,
  `exclusive_zone = -1`, `KEYBOARD_INTERACTION_ON_DEMAND`, and commits a null
  buffer. `ShellController` owns the toggle (`showAppsDrawer`/`hideAppsDrawer`),
  a reduced-motion-aware fade/scale in QML, Escape and click-outside dismissal,
  pointer/keyboard routing, and a `DF_APPS_DRAWER_FIXTURE` capture seam.
- **The launcher is the Dock's launch path.** Activation calls `openApp`, which
  activates a running window or launches the installed entry through
  `buildLaunchCommand` + `appLaunchEnvironment`; the acted-on tile rect is
  recorded in `m_dockTiles` so `set_launch_origin` hands the compositor the real
  icon exactly as the Dock does (ADR
  [0105](0105-dock-launch-origin-tile-handoff.md)). A failed launch raises the
  shell's existing failure notice, never a silent no-op.
- **One global shortcut and one permanent Dock tile.** A new
  `InputAction::ShowApps` ("show-apps") is bound to F4 in
  `default_system_bindings()`; the shell injects a permanent Applications
  launcher into the Dock's entry model (`appsLauncher: true`, immediately after
  the first pinned tile, so it sits second from the left on a default Dock).
  The Dock renders it as a grid tile and raises `appsDrawerRequested()`. It is a
  fixed affordance: it cannot be dragged, reordered, or removed, and its
  context menu's **Remove from Dock** row is disabled. Both the shortcut and the
  tile route to `toggleAppsDrawer()`, and the drawer is centered over the
  output. (An earlier revision placed the affordance in the menu bar as an
  `applications` status item; that item was removed.)
- **app-index absence is explicit.** When the service is unreachable the overlay
  renders `Application index unavailable`, never a blank panel, and the open
  drawer refreshes on app-index's coalesced `Changed` signal.

## Consequences

- The category mapping and the list rules are headless-unit-tested in
  `tst_dockcore`; the overlay's tiles, filters, launch signal, and absence state
  are QML-tested in `tst_appsdrawer`. The deferred Spotlight-equivalent search
  pane can reuse the helper's query predicate but must not replace the drawer's
  local filtering.
- Adding a pill is one row in `kCategoryMap` plus one entry each in
  `appsDrawerCategoryKeys()` and `AppsDrawer.categoryLabels`; a new category
  glyph is not part of this task (T-19.1's icon treatment is separate).
- The drawer's scene coordinates equal output coordinates (the surface maps 1:1
  to the output), which is why the recorded tile rect can feed
  `set_launch_origin` unchanged.
- The grid follows the measured Spotlight Apps geometry: 115×92 pt cells,
  77 pt icons, an 18 pt single-line label, a 115 pt column pitch and a 102 pt
  row pitch. A column count capped at seven and a viewport capped at five rows
  keep wide outputs at seven per row; narrow outputs wrap to fewer instead of
  overflowing. The caps (`maxColumns`, `maxRows`) mean the keyboard row stride
  always matches the real layout.
- The card is a translucent frosted plate (0.72 alpha on the sampled path, a
  0.86 milky plate on the fallback path) with a 28 pt radius, a soft floating
  shadow and a subtle top-to-bottom light falloff. Its top row is the measured
  Spotlight chrome: a 34 pt Back button, a 58 pt rounded search field, and a
  34 pt More button (Refresh Applications / Close). The category row is a set
  of 26 pt translucent capsule chips rather than a segmented group. The card
  hugs the content with `panelPadding` (20 pt) on the sides and
  `panelPaddingVertical` (20 pt) above and below, clamped to the output so a
  narrow screen shrinks the card instead of clipping it.
- Vertical wheel input scrolls the grid from anywhere over the card, not only
  when the pointer is over the grid's column band. The grid Flickable spans the
  card width; a vertical wheel over the search row or chip row is forwarded to
  the same scroll path, while a horizontal wheel still scrolls the chips.
- Every open is a fresh launcher session: the query, category selection,
  keyboard selection, and grid/pill scroll offsets reset, and the search field
  takes active focus. The tiles are not focusable — a `Repeater` rebuild must
  never steal focus from the field mid-typing. Escape dismisses outright (the
  next open starts clean); Return on a filtered list opens the top hit when no
  arrow selection is active.

## Visual refinement (2026-11-10)

The card now follows the measured popover transparency more closely instead of
the earlier 0.72 readability compromise: `panelOpacity` is 0.58 in light and
`panelOpacityDark` is 0.68 in dark on the sampled path, with the 0.90 milky
plate kept for the no-blur fallback. The card gains the reference's 1 px top
specular line and a tighter 16 pt vertical inset; its shadow uses the drawer's
own 40 pt / 0.22 / 6 pt offset instead of the generic overlay elevation.

The top row is unchanged in geometry (34 pt Back, 58 pt field, 34 pt More),
but the field now uses 16 pt type, a 16 pt radius and an inner top sheen, and
the Back/More controls are bare glyphs whose capsule appears only on
hover/focus. Category capsules keep the measured 26 pt height but use 13 pt
labels and 16 px horizontal padding (the measured chip widths imply 16 px, not
12 px), and the selected capsule is the light surface plate — white in light,
the elevated dark surface in dark — rather than an inverted ink pill. The
freedesktop category set, list helper, launch path, and keyboard model are
unchanged.
