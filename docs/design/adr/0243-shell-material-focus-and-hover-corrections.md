# 0243 — Shell material, focus and hover corrections

> Menu/drawer rendering and dismissal are corrected by [ADR 0244](0244-frosted-menu-and-drawer-stability.md).

Status: accepted (2026-11-09)

The component review uses the sibling `macos-design-review` repository's
`docs/LINUX-HANDOFF.md`, `docs/SURFACE-REVIEW.md` and material schema. Those
notes distinguish the sharp menu-bar veil, sampled shell cards and near-opaque
app chrome. The Apps material fit remains an open reference row; the detached
menu material is an adaptation for the drawer, not a new measured fit.

## Decisions

- **Menu-bar polarity follows the wallpaper, independently of app appearance.**
  Prefer pure white text/glyphs and the existing dark wash. A top-band sRGB
  luminance of at least 0.78 selects black text/glyphs and the light lift.
  The shell sends `veil-light` or `veil-dark` as the material region key with
  the foreground commit; the compositor resolves the matching scheme. The
  measured sharp depth LUTs remain unchanged. Opaque accessibility/backend
  fallbacks use their own scheme's foreground. This supersedes ADR 0219's
  appearance-driven choice of veil, not its optical measurements.
- **One compositor tint per sampled menu.** Attached, detached and status menus
  use the resolved flat-band alpha, tint and blur; their QML fill is transparent
  while sampling is available. This removes the generic adaptive tint plus
  client tint combination. Geometry and accessibility fallbacks remain shared.
- **The Apps drawer is sampled glass.** Its default preset is `menuDetached`,
  replacing the intentionally solid `windowChrome` preset. The client uses
  the same transparent/fallback rule as shell menus. Reduced transparency,
  missing sampling and minimal quality retain an opaque readable card.
- **Desktop and app context menus differ.** The Files desktop uses the detached
  menu's alpha over the wallpaper. This is an alpha-only local fallback, not a
  sampled blur: the desktop process cannot read the compositor scene. App
  content menus retain the opaque local plate. The desktop independently binds
  appearance, transparency and accessibility settings. This refines ADR 0221.
- **Focused app names come from the localized desktop entry.** The broker strips
  `.desktop` before deriving a fallback name; it must not turn that suffix into
  “Desktop”. Empty focus is Files and never a window title. Synthesized menu
  labels use the same display name as the bar.
- **Menu exporters do not run on the Dock's UI thread.** Focus publishes the
  local identity immediately. A single bounded worker resolves service menus;
  generation checks discard stale replies and coalesce intervening changes.
  It only reads captured values and queues publication to the UI thread.
  Identical running-window projections skip redundant work. The existing GPU
  transport and damage-aware material caches remain in use; this does not
  claim process isolation for every shell feature.
- **Dock names appear immediately and follow the hovered tile.** The Dock has
  its own zero-dwell token and zero opening fade. Generic tooltips keep their
  existing dwell. Grouped-window labels appear during chooser dwell and yield
  when the chooser opens. This intentionally supersedes the 1.4-second Dock
  delay in ADR 0218; the earlier capture remains historical evidence.
- **Shadow pieces share rounded edges.** Motion and physical scaling transform
  both endpoints instead of independently rounding position and size. This
  removes cracks and overlapping translucent rows at fractional scales. A
  bounded 32-entry geometry cache avoids repeating rounded subtraction for
  settled windows while unrelated shell content animates.

## Validation and limits

Coverage includes menu-broker suffix handling, fractional-scale shadow seams,
the window suite, QML menu polarity/materials, drawer fallbacks, immediate Dock
retargeting, and desktop Reduce Transparency. Material model checks retain the
measured LUTs and menu geometry. Native GPU frame-time and flicker acceptance
still require a live session; headless tests do not establish smoothness at a
particular refresh rate. Foreground selection still uses the cached wallpaper
top band, not a readback of each live window under the bar.
