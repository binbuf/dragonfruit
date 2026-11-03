# Design System

## Summary

We build the design system **before** building ten applications. It is the
single place where the desktop's visual identity lives — corner radii,
translucency, shadows, animations, typography, padding, focus rings, hover
behavior, reduced-motion behavior, and dark/light mode — and every
first-party application consumes it.

Qt Quick is particularly appropriate for this kind of custom animated
interface: Qt treats animation and transitions as first-class concepts and
provides its own scene graph and rendering engine.

## Component library

```text
Window
TitleBar
TrafficLights
Sidebar
Toolbar
SplitView
SettingsRow
SettingsGroup
Toggle
SegmentedControl
Slider
Select
Popup
ContextMenu
MenuBarMenu
ShortcutHint
SearchField
SourceList
Icon
TablerIcon
PhosphorIcon
SettingsCategoryIcon
Dialog
Sheet
Popover
Tooltip
ScrollView
```

`Tooltip` (T-14.7i) is the passive hover label: pointer-anchored,
non-focusable, token-driven, and used first by the Dock name labels (ADR
[0093](adr/0093-tooltip-joins-the-design-system.md)). Since T-14.7aa it is a
pill with an opt-in pointer tail (`tailVisible`, the Dock's shipping form) and
a token inner rim; the tail aims at the anchor centre and rides the owner's
overlay rect (ADR
[0162](adr/0162-dock-hover-zoom-profile-and-label-tail.md)).

`TablerIcon` (ADR [0185](adr/0185-tabler-line-icons.md)) is the foreground
line-glyph primitive: [MIT-licensed Tabler Icons](https://tabler.io/icons)
3.50.0 (commit `6904efd2…`) is vendored under `assets/icons/tabler/`, exposed
at `qrc:/icons/tabler/<name>.svg` (`<name>-filled.svg` for the small solid
set), and rendered as a tintable stroked/filled `ShapePath` sized from
`name`/`color`/`size`/`weight`/`strokeWidth`. A glyph is a *resource* — no
color or container is baked in — while the tint and any tile stay in QML; a
referenced glyph that is not vendored fails `make check-tabler`.
`Icon.qml` is now a compatibility facade over `TablerIcon`: it keeps the
inherited vocabulary (`folder`, `chevron-right`, `wifi`, …) used by Files,
Settings, and shell chrome and maps each name to a Tabler outline glyph, so the
migration did not touch every call site. New code uses `TablerIcon` directly.

`PhosphorIcon` (T-19.1a) remains the **filled** primitive for surfaces that
need a solid near-white mark on a colored container: the System Settings
category tile. (The first-party app artwork uses Tabler's filled
`folders`/`settings` marks.) Phosphor 2.0.8 (MIT) is vendored under
`assets/icons/phosphor/` and rendered as a tintable `ShapePath` sized from
`name`/`color`/`size`/`weight`; a referenced glyph that is not vendored fails
`make check-phosphor` (ADR [0163](adr/0163-phosphor-icons.md)). The line/fill
split is recorded in ADR [0185](adr/0185-tabler-line-icons.md).

`SettingsCategoryIcon` (T-19.1b) is the one gradient-backed category tile: a
rounded container with a token drop shadow, a vertical gradient, a top inner
highlight, and a centered near-white `PhosphorIcon` in the `fill` weight. Only
**System Settings** uses the container — the menu bar, Dock, and drawer use
bare `TablerIcon` glyphs. The component is category-agnostic
(`source`/`gradientStart`/`gradientEnd`/`symbolColor`/`size`/`radius`/`glow`);
the category → (glyph, gradient) table lives in the Settings pane catalog
(`SettingsPanes.categoryStyles`, keyed by the pane's `icon` identity) and a
pane without a mapping keeps the Tabler-backed `Icon` glyph. Geometry comes from
`component.settingsCategory.*`; the literal category hues are named constants in
that one table and are never baked into an SVG (ADR
[0164](adr/0164-settings-category-tile.md)).

## Menu layout language

Both menu surfaces — the top bar's dropdowns (`MenuBarMenu`) and pointer
context menus (`ContextMenu`) — use one row anatomy, implemented once in
`MenuRow.qml`:

```text
[check/icon gutter] label (flex, one line, elides) [shortcut hint] [chevron gutter]
```

- The **check and chevron gutters are reserved on every row**, even when the
  row is unchecked or has no submenu. Labels and shortcut hints therefore line
  up across rows and between the two menu components; a submenu chevron never
  shifts its row's hint.
- The **label is the only flexible cell**. It is single-line and elides before
  it can touch the hint; the hint is right-aligned in its own trailing cell.
- Rows are `component.contextMenu.rowHeight` (26) tall inside
  `component.contextMenu.padding` (6); menu-bar dropdowns use the matching
  `component.menuBarMenu.*` values. A separator is one `padding` tall.
- `component.shortcutHint.*` owns the hint's glyph size, weights, and the gap
  between keys. The bar itself keeps `component.menuBar.*` (28 px high, 8 px
  horizontal padding, 15 px status glyphs, 2 px status gap) and a 24 px title
  slot inside `component.menuBarMenu.barHeight` with 10 px side padding. The
  system menu is the brand mark, the application menu is bold, and app menus
  are medium.

### Shortcut hints

`ShortcutHint` is the one renderer for a row's `shortcut` field. It never
prints `Ctrl+Alt+Delete`: the chord is parsed into a canonical row of key
glyphs. The vocabulary follows the desktop's keymap roles
([keymap.md](../keymap.md)):

| Role | Glyph | Accepted input |
|---|---|---|
| Super (Command role) | `⌘` | `Super`, `Cmd`, `Command`, `Meta`, `Win`, `⌘` |
| Control | `⌃` | `Ctrl`, `Control`, `⌃` |
| Option / Alt | `⌥` | `Alt`, `Option`, `Opt`, `⌥` |
| Shift | `⇧` | `Shift`, `⇧` |
| Function | `fn` | `Fn`, `Function` |
| Delete | `⌫` | `Delete`, `Del`, `Backspace`, `⌫` |
| Return | `↵` | `Return`, `Enter`, `↵`, `⏎` |
| Escape | `⎋` | `Escape`, `Esc`, `⎋` |
| Tab | `⇥` | `Tab`, `⇥` |
| Space | `␣` | `Space`, `Spacebar`, `␣` |
| Caps Lock | `⇪` | `CapsLock`, `Caps`, `⇪` |
| Page Up / Down | `⇞` / `⇟` | `PageUp`/`PgUp`, `PageDown`/`PgDn` |
| Arrows | `←` `↑` `→` `↓` | `Left`/`ArrowLeft`, … |

- Modifiers are reordered to the fixed **Fn, Ctrl, Alt, Shift, Super** order,
  followed by the key. The input's order and `+` separators never leak into
  the UI, so every row reads the same way.
- Letters are uppercased; punctuation and function keys stay as typed
  (`F3`, `,`).
- A string that is not a chord is rendered verbatim in the same right-aligned
  trailing cell. The Dock's window rows use `shortcut` for a Space name
  (`Space 2`), so the hint never guesses a chord where there is none.
- Key names are **data, not prose**: first-party menu models carry canonical
  English names (`Super`, `Delete`, `Return`) and are never passed through
  `qsTr()`. Only the row label is localized. A translated key word would not
  parse and would fall back to plain text instead of a symbol.
- Modifier glyphs use `textTertiary`, the key label uses `textSecondary`, and
  a highlighted row uses `accentContent` for both. The hint is decorative to
  AT-SPI; the menu item's accessible description carries the raw chord.
- There are **no keycap boxes**: the Tahoe reference shows plain right-aligned
  symbols, and boxes add noise at a 26 px row height. The glyphs come from the
  bundled Inter face, so they tint and scale like the rest of the row and need
  no additional icon asset.

The decision and its alternatives are recorded in ADR
[0187](adr/0187-menu-row-anatomy-and-shortcut-glyphs.md).

## Token architecture

```text
primitive tokens    radius, color, elevation, spacing, type family and scale, duration
        │
semantic tokens    color-scheme roles (surface, elevated, accent, "on" colors),
        │           material roles (translucency, blur amounts)
        │
component tokens    per-component sizes, states, animation curves
```

Tokens are defined **once** in `design-system/tokens` and consumed
everywhere: QML singletons for the shell and apps, and a generated Rust
module for the compositor. Sharing the source is what makes
compositor-drawn SSD titlebars and first-party `TitleBar`s unable to drift
apart (see [05-window-decorations.md](05-window-decorations.md)).

The `component.elevation.{low,med,high,overlay}` group is the shared shadow
source (T-04.1a): each level resolves the primitive elevation scale into the
`blur`/`offsetY`/`layers` that both the `Shadow` QML component and the
compositor's window-shadow pass read, so surface shadow geometry is identical
across the process boundary by construction (see
[ADR 0011](adr/0011-elevation-shadow-tokens.md)).

The semantic `material` group (`chromeOpacity`/`chromeBlur`,
`dockOpacity`/`dockBlur`, `popupOpacity`/`popupBlur`, `shadowOpacity`) is the
shared source for surface materials (T-04.2): the QML `Theme.material` group
tints the shell chrome and the compositor's backdrop pass reads the same
values, per color scheme, so the two sides cannot drift (see
[ADR 0013](adr/0013-backdrop-blur-pass.md)). The Dock's own `dock*` tokens keep
its frost tunable independently of the menu bar (T-14.7j, ADR
[0102](adr/0102-dock-material-role-and-qml-glass-layers.md)).

## Typography and the system font

The desktop's type is **Inter** (SIL OFL 1.1, `assets/fonts/Inter/`), declared as
`primitive.font.family`. Every first-party process calls
`Dragonfruit::installSystemFont()` from `libs/system-font` after
`QGuiApplication` exists and before QML loads: the bundled variable faces are
registered from Qt resources and the family becomes the application default,
so QML `Text` and stock controls inherit it exactly as AppKit text inherits
SF Pro. Components keep owning size and weight through the type-scale tokens
and never name a family.

Development is self-contained: the faces are compiled into the first-party
binaries, so the shell, apps, and the gallery goldens render identically on
any host instead of following the host's default font. Packaging will
additionally install the faces system-wide (fontconfig) so third-party
Qt/GTK/XWayland apps inherit Inter too; that half is deliberately not wired
up yet.

## Motion

Named curves and durations (`motion.spaces-switch`, `motion.dock-magnify`,
`motion.menu-open`) live in the token layer, not in component code. Two
rules:

- Every animation maps to a **reduced-motion variant** that removes
  translation/scale while keeping the state change legible. A headless sweep
  enumerates every motion token and every animation site in the shell,
  design-system, and apps and fails if one lacks a variant
  ([adr/0155](adr/0155-compositor-magnifier.md)).
- Gesture-driven transitions are **progress-based and interruptible**; a
  discrete "instant" code path is a bug (see
  [03-workspaces.md](03-workspaces.md)).
- Dock magnification smooths the pointer with `motion.dock-magnify-track` (a
  short low-pass with **no** overshoot) and recomputes geometry every frame
  from the smoothed position, so it stays progress-based and interruptible.
  `motion.dock-magnify`'s overshoot is reserved for discrete changes (the icon
  size and the reveal); the hover engage/release amplitude uses the
  `motion.dock-hover` ease (T-14.7aa), whose gentler ramp keeps the whole-row
  translation from popping. The
  plate/margin values it feeds are dock spacing tokens,
  never literals. See [ADR 0089](adr/0089-dock-plate-geometry-and-live-panel-rect.md),
  [ADR 0111](adr/0111-dock-magnification-tracking-stability.md), and
  [ADR 0162](adr/0162-dock-hover-zoom-profile-and-label-tail.md).

## Quality gates

- A component **gallery app** renders every component in every state,
  scheme, and motion variant; visual regression tests run against it.
- Icon registries are build gates: `make check-tabler` rejects an unvendored
  line glyph (including the `Icon.qml` facade map and the dynamic service
  vocabulary), and `make check-phosphor` does the same for the filled tile/app
  set.
- Keyboard navigation and AT-SPI roles are verified **per component**, not
  re-proven per application. The live cross-process check is layered on top:
  the first-party apps are dumped and walked keyboard-only through AT-SPI, and
  every compositor-owned global flow is proven keyboard-only in the headless
  conformance suite. The shell's offscreen chrome is not exported to the
  accessibility bus (see [ADR 0154](adr/0154-atspi-and-keyboard-audit-boundary.md)).
- Dark/light and reduced-motion variants are part of the definition of done
  for every component.
- The T-17 premium gate signs off the **live** visual floor against the goldens
  with `scripts/capture-t17-visual-floor.sh` (`make t17-visual-floor-capture`):
  one nested session captures dark, light, and dark+reduced-motion chrome and
  pairs it with the `window_*`/`ssd_*` goldens. The reviewed verdict and the
  recorded token-driven-approximation waiver are in
  [../captures/t17-visual-floor.md](../captures/t17-visual-floor.md) and
  [ADR 0176](adr/0176-t17-visual-floor-sign-off.md); a true GPU blur sampler
  remains a post-gate item behind the same tokens.
- **Every user-visible string is externalized** with Qt's `qsTr()`/`tr()` and
  checked by the string-extraction gate (`scripts/i18n-extract.py`, wired into
  `make lint`/CI). The checked-in `.ts` catalogs are loaded at runtime by
  `dragonfruit-i18n`, which also sets the default `QLocale` so dates, times,
  and numbers format for the active language
  ([ADR 0156](adr/0156-localization-catalogs-and-locale-formatting.md)).

## Rules

- **Every first-party application consumes these components.** No app rolls
  its own titlebar, menu, or settings row.
- **One source of visual truth.** Tuning corner radii, translucency,
  shadows, spacing, or animation curves happens here, once, and propagates
  everywhere.
- **Reduced motion and accessibility are not afterthoughts.** The component
  library owns reduced-motion behavior, keyboard navigation, focus rings, and
  AT-SPI-compatible semantics (`QAccessible`).
- **Dark/light mode is a component-level concern,** not an app-level one.
- **Localization is a component-level concern too.** Strings are externalized
  where they are written (never concatenated into user-visible text), and a
  translated string must fit the layout — prefer short catalog terms over a
  literal translation that elides in the sidebar or a segmented control.
- **No libadwaita.** First-party apps must have a strongly non-GNOME visual
  identity; libadwaita would put us in a recurring fight against another
  desktop's design language.

## Visual direction

Our visual baseline is **ours**, deliberately. "macOS-like" applies to the
interaction quality and mental model — animation timing, spatial organization,
control placement — not to Apple's bitmap output. Apple's own design keeps
moving (window shapes, menu-bar iconography, materials), so building against
"whatever macOS currently looks like" would make Apple an involuntary upstream
design dependency. See [14-risks.md](14-risks.md) for the IP constraints on
assets and branding.

Third-party **open** icon sets are vendored resources under their own MIT
licenses — Tabler for the line glyphs and Phosphor for the filled Settings
tiles — not copies of Apple's artwork. The line/fill split and the credit
package are recorded in ADR
[0185](adr/0185-tabler-line-icons.md).

## What the design system provides to siblings

- Compositor-drawn server-side decorations reuse the same visual tokens as
  first-party `TitleBar`s (see [05-window-decorations.md](05-window-decorations.md)).
- The `MenuBarMenu` component is the first-party path into the menu-broker
  (see [06-global-menu.md](06-global-menu.md)). Its normalized, JSON-serializable
  entry shape (`menuModel`) is the native publication contract: an app declares
  its menus once in that shape, and the shell's `MenuBar` consumes
  `applicationMenuItems` + `appMenuModel` unchanged (ADR
  [0041](adr/0041-native-menu-model-publication-shape.md)). Settings' single
  source is `apps/settings/SettingsMenu.qml`.
- Settings and Files are built entirely from these components (see
  [08-settings.md](08-settings.md), [09-files.md](09-files.md)).
