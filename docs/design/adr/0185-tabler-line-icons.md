# 0185 — Tabler is the line-glyph library; `Icon` is a compatibility facade

## Status

accepted

## Context

The inherited `design-system/components/Icon.qml` draws original geometry for a
small, fixed vocabulary. Track 19 replaced the foreground vocabulary with
Phosphor and introduced the tintable `PhosphorIcon` primitive (ADR
[0163](0163-phosphor-icons.md)), the Settings category tile (ADR
[0164](0164-settings-category-tile.md)), the menu-bar marks (ADR
[0165](0165-menubar-phosphor-marks.md)), and the first-party app artwork (ADR
[0166](0166-first-party-app-icons.md)). That migration left two gaps:

- The **line** surfaces — Files toolbars and sidebar, the menu bar, the Dock
  launcher, the Applications drawer, the OSD, and the small chrome marks — were
  still drawn by the hand-authored `Icon.qml` bars/Canvas, or by Phosphor's
  solid weight where an outline reads better.
- The Tahoe material pass (ADR [0182](0182-tahoe-liquid-glass-material-pass.md))
  raised the visual floor of the chrome, but the glyph language did not move
  with it: the hand-drawn marks are visibly less consistent than the rest of
  the surface.

Tabler Icons is an MIT-licensed, 5,000+ glyph set with a consistent 24 px
outline grid, round caps/joins, and a small filled set. Its outline weight is
the right match for a Tahoe-like line language, and its permissive license
fits the repo-wide MIT policy (see [licensing.md](../../licensing.md)).

Qt 6.11 has no `QtSvg` module in this toolchain, and the two obvious tint
mechanisms are unreliable: `Image` over an SVG does not tint, and
`MultiEffect` colorization is a GPU shader effect that silently no-ops on the
headless software scene graph the gallery and QML tests run on. The tint
mechanism therefore has to render under **both** scene graphs, exactly as
`PhosphorIcon` already does.

## Decision

- **Tabler Icons 3.50.0 is vendored** under `assets/icons/tabler/`, pinned to
  `v3.50.0` (commit `6904efd245391ad396df17654738f52f6da8c40e`). Files keep
  upstream names: `<name>.svg` for the `outline` weight and
  `<name>-filled.svg` for the pinned `filled` marks. The set is curated to the
  glyphs shipped surfaces use, not the full upstream catalogue, so the
  existence gate is meaningful. The asset README records the upstream, version,
  commit, source paths, and license; the license text ships at
  [LICENSES/Tabler.txt](../../../LICENSES/Tabler.txt) and in
  [NOTICE](../../../NOTICE).
- **`TablerIcon` is the line-glyph primitive.** It renders one `ShapePath`
  (stroked for `outline`, filled for `filled`) with bound `strokeColor` /
  `fillColor`, using path data generated at build time into the
  `TablerGlyphs` singleton (`scripts/gen-tabler-glyphs.py`). It keeps the
  glyph's square aspect (Tabler's `viewBox="0 0 24 24"`), exposes `name`,
  `color`, `size`, `weight`, `strokeWidth`, and an optional `accessibleName`,
  and draws no container. A rare outline glyph with a filled dot (for example
  `accessible`) gets a second fill-overlay `ShapePath`.
- **`Icon.qml` becomes a compatibility facade over `TablerIcon`.** It keeps the
  public API (`name`, `color`, `size`) and the inherited vocabulary, and maps
  each legacy name to a vendored Tabler outline glyph in one table. Files,
  Settings, the menu bar, the Dock, and the design-system components therefore
  adopt the new glyphs without a mass rename. New code uses `TablerIcon`
  directly; the facade is the migration seam, not the preferred API.
- **Phosphor remains the filled tile set.** The System Settings category tile
  needs a solid near-white mark on a gradient container, and Tabler's filled
  set does not cover every category. `SettingsCategoryIcon` therefore keeps
  `PhosphorIcon` in the `fill` weight; the first-party app icons are generated
  from Tabler's filled `folders`/`settings` marks. The split is deliberate:
  **Tabler = line glyphs, Phosphor = filled Settings-tile glyphs.**
- **The menu bar, Dock launcher, Applications drawer, OSD, Control Center, and
  first-party app artwork migrate now.** The menu-bar battery keeps its
  continuous level by composing the Tabler `battery` outline with a token
  `Rectangle` fill overlay inside the cell (the software renderer does not clip
  a child `Shape` to an ancestor); the fill geometry follows the Tabler cell
  interior, not the old Phosphor geometry.
- **A referenced glyph that does not exist fails the build.** `make
  check-tabler` runs `gen-tabler-glyphs.py --check` (the registry must match
  the SVG directory), `gen-app-icons.py --check` (the first-party artwork must
  match the pinned filled glyphs), and `scripts/check-tabler-icons.py` (every
  literal `TablerIcon`/`Icon`/`Button` glyph name must resolve, every alias
  target must be vendored, and the dynamic vocabulary emitted by the
  services/Files models must be mapped). The runtime component additionally
  warns and renders blank for an unknown name, but a typo never reaches a
  shipped build. The existing `make check-phosphor` gate remains for the filled
  set.
- **No Apple artwork is vendored or reproduced.** Tabler and Phosphor are
  third-party MIT resources; the original-assets rule in
  [14-risks.md](../14-risks.md) forbids Apple's assets, not third-party open
  assets.

## Consequences

- The line surfaces gain a consistent, Tahoe-like outline language without
  touching every call site: the `Icon` facade is the one migration point, and
  the dynamic names from Files/Control Center keep working through the same
  map.
- A new line glyph is a file drop into `assets/icons/tabler/` plus
  `scripts/gen-tabler-glyphs.py`; a new legacy name is one row in the
  `Icon.qml` map. A new filled tile mark is still a Phosphor file drop plus
  `scripts/gen-phosphor-glyphs.py`; a new filled app mark is a Tabler file drop
  plus `scripts/gen-tabler-glyphs.py`.
- The menu bar's battery fill is the one mark that is not pure Tabler path
  geometry; if Qt later clips a child `Shape` under software, the overlay can
  switch to a clipped `battery` fill without changing the public shape.
- The generated `TablerGlyphs.qml` is committed and checked like the token and
  Phosphor artifacts; the SVGs remain the single source of truth.
- The visual-regression gallery gains a `Tabler` page (outline, filled, sizes)
  and the `Icons` page now renders the Tabler-backed facade. The goldens are
  regenerated with `make gallery-snapshot`.
- The licensing/credit surface is explicit: `NOTICE`, `LICENSES/README.md`,
  `LICENSES/Tabler.txt`, the asset README, this ADR, and the design-system
  docs all name the version, commit, source, and license.
