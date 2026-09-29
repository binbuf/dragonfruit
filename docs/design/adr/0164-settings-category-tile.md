# 0164 — The System Settings category tile is one container; the gradient lives in the pane catalog

## Status

accepted

## Context

T-19.1a vendored Phosphor and gave us `PhosphorIcon`, a bare tintable glyph
with no container (ADR [0163](0163-phosphor-icons.md)). Track 19 then wants the
macOS System Settings look: each category is a small rounded, gradient-backed
tile with an inner top highlight, a soft drop shadow, and a near-white solid
glyph. That container is a **System Settings-only** treatment — the menu bar,
the Dock, and the Applications drawer use bare glyphs (T-19.1c/d).

Two constraints shape the design. First, the category colour must not be baked
into an SVG: the container is the differentiator and only the gradient changes
between categories, so one glyph file serves every hue. Second, the gradient,
highlight, and shadow must render under the headless **software** scene graph
the gallery and QML tests use, where shader effects silently no-op (the same
limit `Shadow.qml` documents).

## Decision

- **`design-system/components/SettingsCategoryIcon.qml` is the one container.**
  It is an `Item` with `source` (a Phosphor name), `gradientStart`,
  `gradientEnd`, `symbolColor` (near-white default), `size`, `radius` (derived
  from `size` through a token ratio, overridable), and an optional `glow`. It
  draws the design-system `Shadow` (token elevation), a rounded `Rectangle`
  with a vertical `Gradient`, a top inner highlight (a white wash that fades
  out by mid-tile), and a centered `PhosphorIcon` in the `fill` weight tinted
  to `symbolColor`. `Rectangle.gradient` and the layered `Shadow` both render
  under software, and the glyph is a `ShapePath` (T-19.1a).
- **The category → (glyph, gradient) table is exactly one place, in the app.**
  `apps/settings/SettingsPanes.qml` holds `categoryStyles`, keyed by each
  pane's existing design-system `icon` identity, and `categoryStyle(pane)`
  resolves it. The literal hues are named constants there — never in a
  component and never in an SVG. The design system stays category-agnostic; the
  style primitive takes resolved values.
- **`Icon.qml` stays the fallback.** A pane whose icon has no Phosphor mapping
  (Trackpad has no vendored mark) resolves `categoryStyle` to null and the
  header/sidebar keep the original `Icon` glyph. Adding a category is a table
  row; adding a glyph is a file drop plus `scripts/gen-phosphor-glyphs.py`.
- **Geometry is tokenized.** `component.settingsCategory.*` (size, radius
  ratio, icon ratio, highlight ratio/opacity, glow opacity) and
  `component.sidebar.categoryIconSize` join `tokens.json`; the generated
  `Theme.qml`/`design_tokens.rs` carry them like every other value.

## Consequences

- `PaneHeader.qml` (the hero tile) and `SettingsShell.qml`'s sidebar rows
  resolve the style from the pane and pass it to the component; the `Sidebar`
  delegate renders `SettingsCategoryIcon` for an item that carries a resolved
  `category` and `Icon` otherwise, so the container exists nowhere else.
- The gallery gains a `SettingsCategory` page and goldens, and the design-system
  QML test covers size/tint/gradient from properties and the unknown-glyph
  blank-not-crash case. The static `check-phosphor-icons.py` scan sees only the
  gallery's literal `source` names; the app-side table is validated by the
  Settings test, which asserts every mapped glyph resolves in the vendored fill
  weight (the sanctioned path for dynamic names, per ADR 0163).
- A future Qt with software-capable shader effects could replace the layered
  shadow/highlight with a blur, but the component is the single place to do it.