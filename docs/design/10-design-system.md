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
AppWindow
TitleBar
TrafficLights
WindowResizeGrips
Sidebar
Toolbar
SplitView
SettingsRow
SettingsGroup
Toggle
Button
SegmentedControl
Slider
Select
Popup
ContextMenu
MenuBarMenu
MenuRow
ShortcutHint
SearchField
SourceList
Icon (compatibility facade)
TablerIcon
PhosphorIcon
SettingsCategoryIcon
Dialog
Sheet
Popover
Tooltip
ScrollView
Shadow
FocusRing
Skeleton
DragonfruitLogo
```

The inventory is the QML module's actual surface, not a wish list. `MenuRow`
and `ShortcutHint` are the shared menu anatomy; `Shadow`, `FocusRing`, and
`Skeleton` are shared primitives rather than app-local copies;
`WindowResizeGrips` is the shared resize affordance used by first-party
windows.

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
`dockOpacity`/`dockBlur`, `popupOpacity`/`popupBlur`, `shadowOpacity`, plus
the `chromeFallbackOpacity`/`dockFallbackOpacity`/`popupFallbackOpacity`
milky complements) is the shared source for surface materials (T-04.2): the
QML `Theme.material` group tints the shell chrome and the compositor's
backdrop pass reads the same values, per color scheme, so the two sides cannot
drift (see [ADR 0013](adr/0013-backdrop-blur-pass.md)). The Dock's own `dock*`
tokens keep its frost tunable independently of the menu bar (T-14.7j, ADR
[0102](adr/0102-dock-material-role-and-qml-glass-layers.md)). The
`Material` singleton (T-21.2c, ADR
[0205](adr/0205-popup-family-material-fallback.md)) is the one QML complement:
it returns the sampled-glass scrim or the milky fallback for the popup family
and names the recorded reason, so components do not each re-derive a fill.

### Glass material presets and resolution

The top-level `glass` token group (T195, ADR
[0214](adr/0214-glass-role-presets-and-resolution.md)) is the optical
vocabulary layered above the existing `MaterialRole` wire values and
`DegradeTier` ladder. It defines 17 named presets: `dock`, `menuBarVeil`,
`menuAttached`, `menuDetached`, `statusMenu`, `tooltip`,
`controlCenterOuter`, `controlCenterModule`, `controlCenterExpanded`,
`notificationBanner`, `notificationStack`, `notificationLowerPanel`,
`windowChrome`, `settingsSidebar`, `settingsToolbar`, `filesSidebar`, and
`filesToolbar`. The last four are app-local client-alpha presets consumed by
QML only; the compositor never resolves them because it never draws an app's
sidebar or toolbar. Each preset has a light and a dark calibration with a
`neutralAlpha`/`neutralTint` fit, a separate perceptual fit and weight, blur,
edge/foreground contrast, provenance, confidence, and sRGB colour space.
Resolution mixes the neutral and perceptual fits once rather than applying both
as overlays.

`scripts/gen-tokens.py` validates the group (finite numbers, ranges, required
presets, provenance/confidence, the three measured anchors) and generates
`Theme.glass` plus `design_tokens::glass`. The runtime resolvers are
`design-system/Glass.qml` and `compositor/src/material_preset.rs`; they return
explicit optics and one of four rendering modes:

| Mode | Meaning |
|---|---|
| `sampled-glass` | The sampled backdrop/glass pass runs. |
| `no-blur-veil` | A deliberately veil-only surface (the menu bar); never a missing pass. |
| `solid` | An intentional solid surface, or a safe opaque plate for an unsupported sampled path. |
| `software-fallback` | The software/headless path has no readback; the opaque plate is the contract. |

The menu-bar `no-blur-veil` preset (T200, ADR 0219) is a depth LUT over the
sharp live scene, not a blur. Light is a white lift measured at 0.451 at 5 pt
and zero by 95 pt, scaled by the measured background-luminance response
(`1.0/1.06/0.91/0.48/0.0` at `L=0/0.25/0.5/0.78/1.0`) so it disappears over
white; Dark is the documented black baseline with the same depth shape and an
identity adaptation. A bounded local luminance probe supplies the adaptation
and may raise alpha inside the 39 pt reference strip for foreground
readability, but the displayed backdrop is always the sharp centre sample.
The QML surface paints no fill on the
replacement rail; the compositor veil sits below the foreground and popups.
When the veil cannot run, QML keeps the scheme-appropriate milky fallback.

Resolution precedence is: Reduce Transparency (opaque scheme plate, sampling
and lensing off, slider ignored) → rollout look (`legacy` or `replacement`) →
the light `menuAttached` slider → Increase Contrast edge/foreground boost →
backend/tier rendering mode. Reduce Motion is orthogonal. The light
`menuAttached` preset is the only slider response, interpolated piecewise
through the measured 0.0 / 0.5255 / 1.0 File anchors; dark and every other
preset keep a named baseline. The Dock (and other persistent chrome) keeps its
sampled floor at `Minimal`; transient popups may become solid.

### Menu families (T202)

The menu family uses three of the shared presets: `menuAttached` for the
menu-bar dropdown, `menuDetached` for the tray, Dock and app-owned context
menus, and `statusMenu` for the Wi-Fi/Bluetooth/volume/battery popovers.
Submenus are detached cards even when their parent is attached. The attached
surface has square top corners and the measured ~12.5 pt bottom pair; detached
and status cards are all-rounded. The menu shadow uses the 20 pt elevation
token and the close motion is the measured 183 ms with a 0.95 terminal scale
toward the anchor. The logical close and input release are immediate; a
reopen/retarget before the exit timer cancels the pending unmap. Reduce Motion
collapses the duration and terminal scale.

`Material.menuFill()`/`menuResolution()` are the QML complement. Trusted shell
surfaces (`MenuBarMenu`, the tray context menu, Dock popovers and the status
menus) resolve the sampled preset; app-owned surfaces (`ContextMenu` in Files,
the desktop and Select) pass `materialBackdrop: "local"` and resolve the opaque
scheme plate, never a translucent slab over unfrosted content. The shell
declares each open panel as an independently keyed material region on the
`menubar-popup`/`dock-popup` overlay, with the attached per-corner geometry and
the detached/status all-rounded geometry. Light `menuDetached`/`statusMenu`
preserve their own default fits and follow the measured attached File curve by
a documented relative adjustment (alpha ratio, tint delta); dark presets do
not follow the light slider. The dark attached baseline is calibrated against
the valid dark mid/black context/status samples; its white row remains open
for a controlled recapture. ADR
[0221](adr/0221-menu-family-presets-and-exit.md) records the mapping and the
fallback boundary.

### Control Center materials (T203)

The Control Center is three distinct materials, not one Popup plate with a
second alpha layered on it. The outer plate uses `controlCenterOuter`
(Light 0.265/0.373, Dark 0.489/0.677). Each visible tile is its own
`controlCenterModule` capsule. The measured Wi-Fi Light fit 0.391/0.317 is the
*effective* stacked response; because the compositor's flat-band output is
opaque, the module preset carries that effective fit directly and the module
region is the final composite in its own area (the outer plate stays visible
around the capsules). The unmeasured dark module is an adapted effective
0.65/0.44 so it reads darker than the dark outer. An open Wi-Fi/Bluetooth list
adds a `controlCenterExpanded` region: the dark list is near-opaque 0.96/0.13
(background-independent luma ~0.13); the light list is a readable 0.94/0.96
plate. All three carry the measured 8 pt blur, 0.30 rim and 6 pt / 24 pt
capsule-end lens family.

The shell declares the regions on the `control-center` layer surface and clips
them to the scroll viewport; keys are stable (`outer`, `module-*`,
`expanded-wifi`/`expanded-bluetooth`) and the list is commit-latched with the
panel image. `MAX_PANEL_REGIONS` is 32 to cover the 19 modules plus the outer
plate and two lists. On the sampled path QML paints no outer/module/list fill;
on solid, software-fallback and Reduce Transparency it paints the same resolved
tint/alpha, and Increase Contrast raises the resolved rim. Light sampled
modules switch their foreground to the light-on-glass palette because the
reference modules are dark glass even in Light appearance. ADR
[0222](adr/0222-control-center-regions-and-module-materials.md) records the
region and decomposition decision.

`make control-center-materials-capture` records collapsed/expanded
Light/Dark, Increase Contrast, Reduce Transparency and an absent-adapter case
over neutral and grid backdrops; `scripts/t203-control-center.py` reports the
outer and nested composite error separately and checks that the expanded list
moves the visible material without a stale region.

### Notification materials (T205)

The banner and the Notification Center are three materials, not one Popup
plate. The transient banner uses `notificationBanner` (Light 0.549/0.976); the
Center stack uses `notificationStack` (Light 0.551/0.706, Dark 0.507/0.550);
the separately toned lower panel uses `notificationLowerPanel` (Light
0.407/0.852). The Dark banner (0.56/0.30) and Dark lower panel (0.40/0.30) are
uncovered as independent fits; their token confidence is `adapted` and capture
rows label them underconstrained.

The compositor's replacement rail builds all three with the same measured
flat-band family as the Dock/Control Center, consuming the resolved
alpha/tint directly. The `notification` surface declares one `banner` region
sized to the visible card (96 pt, or 132 pt with actions), so the transparent
strip below a no-action banner is never frosted. The `notification-center`
surface declares two disjoint regions, `stack` and `lower`, separated by the
panel gap and commit-latched with the image. When a banner is active the shell
insets the stack below the banner card, keeping both trusted surfaces
co-visible without overlap or double-frosting. QML is transparent on the
sampled path and paints the same resolved tint/alpha on solid,
software-fallback and Reduce Transparency; the legacy rollout keeps
`Material.popupFill()`. The QML card shadow is disabled on the sampled path so
it cannot darken the opaque compositor flat band.

The Center has a deliberate empty state in the stack and a summary in the
lower panel; the history list scrolls with a visible thumb and a
"Scroll for more" hint when rows overflow. ADR
[0224](adr/0224-notification-banner-stack-lower-panel.md) records the region
and adaptation decision. `make notification-materials-capture` records
Light/Dark empty, banner-only, populated, concurrent and scrollable states plus
Reduce Transparency, Increase Contrast and the software path over neutral/grid
backdrops; `scripts/t205-notification-materials.py` reports independent banner,
stack and lower-panel fit errors.

### First-party app chrome (T206)

Settings and Files draw their own window chrome. Their sidebars and toolbars
use four app-local presets — `settingsSidebar`, `settingsToolbar`,
`filesSidebar`, `filesToolbar` — while the main content sheet and the Files
path bar stay opaque. The measured Light Settings sidebar/toolbar are
0.986/0.857 and 0.961/0.965; the measured Light Files pair is 0.985/0.858 and
0.994/0.922; the measured Dark Files pair is 0.935/0.218 and 0.900/0.247.
Settings Dark has no independent fit and is an explicit adaptation from the
Finder dark pair. The Files surfaces stay independent in both schemes.

These are client-alpha presets, not compositor-sampled passes. The app window
is a normal transparent Wayland surface, so the resolved tint/alpha is
composited by the compositor against the normal lower scene.
`Material.appChromeFill()`/`appChromeBorder()` are the one QML complement; the legacy
rollout keeps the existing opaque `surfaceMuted`/`chrome` fills, and a
non-composited/unsupported client surface (`DF_APP_CHROME_SOLID=1` in the
capture rail) keeps the same opaque plate rather than painting a translucent
slab over unfrosted content. Reduce Transparency resolves to the shared opaque
accessibility plate. Increase Contrast raises the separator ink; focus stays a
controls/text state (the titlebar/controls dim, the material does not).

`AppWindow.chromeTransparent` removes the generic opaque frame/content
backing, keeps an opaque titlebar backing, and the app shells paint an opaque
detail sheet below the toolbar with the correct bottom-right radius. The new
`ChromeSheet` component draws a single fill with independent corner radii so
the sidebar (bottom-left), detail sheet (bottom-right), and path bar
(bottom-right) round exactly one window corner without stacking translucent
overlays. The compositor's window shadow is emitted as a ring outside the
window's rounded body (`shadow_elements_outside`), so it cannot show through
the translucent sidebar/toolbar and darken the measured material; the QML
`Shadow` is hidden on this path because the compositor owns the real shadow.

The preset transport stores a scalar neutral alpha/tint. The reference's small
sidebar chroma pickup is therefore **not** carried: captures report the
rendered channel spread separately, and the missing chroma channel is an
explicit follow-up rather than a claimed colour match. ADR
[0225](adr/0225-first-party-app-chrome-presets.md) records the preset,
fallback, and chroma decision. `make first-party-chrome-check` is in
`make gates`; `make first-party-chrome-capture` records Light/Dark focused,
unfocused, Reduce Transparency, Increase Contrast and solid-fallback rows over
neutral and saturated backdrops, with `scripts/t206-first-party-chrome.py`
scoring the sidebar, toolbar, opaque detail sheet and path bar independently.

### Dock optics fit (T198)

The Dock preset carries the measured optical fit beyond the flat band:
`sharpFraction` 0.12, `edgeTail` 6 pt, and the corner lens
`cornerLensPeak`/`Distance`/`Tail` 9.6/15/50 pt, in both schemes. The
compositor's `window::dock_optics` model mirrors the renderer: a convex mix of
the sharp scene (12%) and Kawase levels 1, 2 and 5 (26/40/22%) at 2/4/8/16/32 pt
checker periods. The 1x capture matches every resolved period within 0.05, the
rim peak 0.399 vs 0.397, half-amplitude 0.75 pt and tail 5.5 pt. The shader
applies the measured flat band (alpha 0.253 light / 0.149 dark, neutral tint
0.604/0.207), the exponential rim and the corner-only lens; Increase Contrast
raises the resolved rim peak/width after the fit. The QML plate layers are
hidden on the sampled path, so tint, rim and outer shadow are painted once;
unsupported paths and Reduce Transparency keep the shell-painted plate.

`make dock-optics-capture` records the 1x light/dark/Reduce
Transparency/Increase Contrast runs, the 1.5x/2x captures and a Dock sweep with
the material-cache counters. The 1x reference layout passes; the 1.5x/2x MTF
rows remain OPEN on the nested scene-texture scale path and are recorded in
`docs/captures/t198-dock-optics.md` (ADR [0217](adr/0217-dock-optics-fit.md)).

The one internal rollout switch is `DRAGONFRUIT_GLASS_ROLLOUT=legacy|replacement`.
The shipped default is `replacement`; `DRAGONFRUIT_GLASS_ROLLOUT=legacy`
selects the legacy rail for A/B and rollback. The shell controller applies the
selection to the `Glass` singleton and the compositor resolver reads the same
variable, so nested and native fixtures select either look reproducibly. An
unset or unknown value takes the shipped default.

The durable inputs are settingsd keys (T196): `appearance.transparency`
(double, default 0.5255, clearer toward 1) drives the light attached-menu
slider, `accessibility.reduceTransparency` forces the opaque scheme plate and
disables sampling, and `accessibility.increaseContrast` boosts the edge and
foreground after the fill decision. The shell's `ThemeBinding` writes the same
values into the `Glass` singleton; the Settings app mirrors them for its pane
previews. The Appearance and Accessibility panes call the same
`Glass.resolve` function as the rendered surfaces, so the preview cannot drift
into a second slider curve. The compositor receives one coherent v9
`set_appearance_policy` snapshot (resolved scheme, slider, both overrides) and
repaints only when the snapshot really changes. The two accessibility
overrides are independent of the rollout switch and of each other; Reduce
Transparency wins over the slider, and Increase Contrast still applies on the
resulting opaque plate. Reduce Motion remains a separate motion-only key.

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
  size and the reveal); the hover engage/release amplitude uses
  `motion.dock-hover`, the measured **135 ms smoothstep** (`3u² − 2u³`, encoded
  as cubic-Bezier control points `(1/3, 0)` and `(2/3, 1)`) so a quick reversal
  retargets from the visible state. The spatial profile is the controlled
  raised cosine over resting tile centres with a 125.1 pt influence radius
  (ADR [0218](adr/0218-dock-motion-raised-cosine-and-tooltip.md)); the
  plate/margin values it feeds are dock spacing tokens, never literals. The
  Dock tooltip waits the controlled 1.4 s `component.tooltip.dwell` and is a
  label-driven pill 8 pt above the plate. See
  [ADR 0089](adr/0089-dock-plate-geometry-and-live-panel-rect.md),
  [ADR 0111](adr/0111-dock-magnification-tracking-stability.md),
  [ADR 0162](adr/0162-dock-hover-zoom-profile-and-label-tail.md), and
  [ADR 0186](adr/0186-dock-reference-retune-profile-fit-and-states.md).

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
- The glass preset schema and resolution policy are checked by
  `make glass-presets-check` (also part of `make gates`): schema ranges and
  finite values, the three measured light anchors, unchanged unmeasured slider
  roles, distinct dark attached/detached presets, the explicit veil mode, safe
  unsupported paths, and the replacement rollout default with the legacy rail
  selectable. The Rust and QML resolvers pin the same cases in
  `compositor/tests/material_presets.rs` and the design-system QML tests.
- First-party app chrome is checked by `make first-party-chrome-check` (also
  part of `make gates`): the four measured/adapted presets, the scalar
  fallback, both app-local Theme/Glass bindings, the transparent frame and
  opaque detail/path-bar surfaces, and the shared `Material.appChromeFill`
  helper. `make first-party-chrome-capture` scores the live Light/Dark,
  focused/unfocused, Reduce Transparency, Increase Contrast and solid rows.
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
