# 0122 — The macOS Tahoe interface language extends across every piece of chrome

## Status

accepted

## Context

The 2026-09 reference capture set under `docs/reference/macos/` is macOS
**Tahoe** (the About capture reads `macOS Tahoe 26.6.2`). The first named
capture, the Dock, already adopted the Tahoe floating-glass language
([0091](0091-dock-tahoe-floating-glass-language.md), [0102](0102-dock-material-role-and-qml-glass-layers.md)).
The remaining captures — the fifteen System Settings panes, the Apple menu and
the Wi-Fi/Bluetooth status menus, Spotlight, About This Mac, Finder, the
Applications grid, Activity Monitor, and the desktop — were reviewed with the
project's vision model in September 2026 and distilled into one note per image
(`docs/reference/macos/<Name>.md`, local only).

Those notes are the reference for the remaining work: T-15 panes and tiles,
T-16 polish, and the T-17 premium gate. Without one shared statement of the
Tahoe language, every pane ticket re-derives window chrome, sidebar styling,
materials, and control vocabulary on its own and drifts. This decision lifts
the Dock's Tahoe rule to the whole surface and fixes the Linux adaptations
once.

It sits inside [10-design-system.md](../10-design-system.md)'s "Visual
direction": Tahoe is the **interaction and visual reference**, reproduced with
Dragonfruit's own symbols, strings, materials, and tokens — never Apple's
bitmap output or branding.

## Decision

- **Tahoe is the reference for all chrome, not just the Dock.** Menu-bar
  menus, settings windows, dialogs/sheets, Spotlight, Control Center, the
  Applications grid, and app toolbars follow the Tahoe language captured in
  `docs/reference/macos/*.md`; measurements are taken from the captures, not
  invented.
- **One material language.** Surfaces are layered glass driven by the
  semantic `material` tokens (`chrome*`, `dock*`, `popup*`) and the
  `component.elevation.*` shadows (ADR [0011](0011-elevation-shadow-tokens.md),
  [0013](0013-backdrop-blur-pass.md)): a translucent fill, a bright inner
  rim/hairline, and a soft shadow. The Dock keeps its own `dock*` tunables
  (ADR 0102). At `Reduced`/`Minimal` material tiers the glass collapses to a
  translucent then flat fill and must still read as intentional (ADR
  [0015](0015-material-degrade-tiers.md)); no refraction is claimed.
- **Window chrome (settings, Files, app windows).** Rounded window with a soft
  shadow and traffic lights (close/minimize/zoom, disabled states dimmed); a
  centered title; a back/forward chevron pair in a pill; the pane's search
  field lives in the sidebar with the placeholder `Search`.
- **Settings window.** A split view with a fixed sidebar (~220–280 px) and a
  fluid detail pane; the sidebar has the search field at top, an account row,
  then a flat, ungrouped list of panes with **colored icon tiles**; the
  selected row is a solid accent fill with on-accent text. Detail content is
  **grouped inset cards** (rounded, subtle separators, generous padding;
  Privacy & Security and Network are flat lists instead). A pane may open with
  a **header card** (icon, title, one-line description, optional `Learn
  more...`) or directly at its first group. Controls apply live; there is no
  Apply button. A trailing `?` help control sits at the bottom-right.
- **Control vocabulary.** Toggle rows; popup rows (value + chevron); slider
  rows with end labels; segmented pickers / tab bars (selected segment accent
  filled); checkbox rows (accent check); radio groups; stepper/number fields;
  disclosure rows (right chevron); trailing action buttons (`Details...`,
  `Options...`, `Edit...`, `Set...`, `Add Fingerprint`, `Add User...`); info
  rows; and warning rows (yellow triangle + caption). These are the
  design-system components, not per-app reimplementations.
- **Menus.** Rounded (~12 px) translucent dropdowns with soft shadow, thin
  separators grouping rows, monochrome leading symbols, right-aligned badges,
  submenu chevrons, and shortcut hints. System and status menus carry the
  captured rows (see [macos-ui-inventory.md](../../reference/macos-ui-inventory.md));
  item sizes are uniform and behave identically in light and dark.
- **Overlays.** Spotlight is a solid light pill (placeholder `Spotlight
  Search`) with a trailing quick-action strip of circular icon buttons;
  Control Center is a floating rounded panel with an on/off toggle header and
  icon-tile rows; the OSD is a floating capsule. Sheet-like quick settings
  keep a `... Settings...` entry into the full pane.
- **Dialogs and sheets.** Rounded, centered content, a primary and a secondary
  button, and no title bar text unless the capture shows one (About This Mac
  has none). The About dialog is the model: illustration, device name and
  generation, a right-aligned label/value block, a primary `More Info...`
  button, and a legal footer with the Apple text replaced.
- **Linux adaptations (decided here, per pane).** Command/Control/Option
  glyphs become Dragonfruit's keymap labels (Super etc., [keymap.md](../../keymap.md));
  Apple-only rows and services are dropped (Apple Account/iCloud, AppleCare,
  AirDrop & Continuity, Apple Intelligence & Siri, Screen Time, Touch ID
  payments, App Store); `About This System` replaces the Apple name;
  `Software Update` reads the distro provider; host daemons that are absent
  degrade to a normal empty/disabled state (ADR
  [0028](0028-audio-adapter-over-wireplumber-cli.md)); all artwork, sounds,
  and fonts are original (`assets/`, Inter).
- **References are local and never ship.** The captures and their per-image
  notes stay under the git-ignored `docs/reference/macos/`
  ([14-risks.md](../14-risks.md)); tickets cite the note by name and the
  distilled inventories in
  [System_Preferences.md](../../reference/System_Preferences.md) and
  [macos-ui-inventory.md](../../reference/macos-ui-inventory.md).

## Consequences

- New UI tickets cite this ADR plus the matching `docs/reference/macos/<Name>.md`
  note instead of restating the language; the per-pane reference sections stay
  short and point at the capture.
- The design-system components already own the language; panes consume them.
  `make check-design-tokens` and the gallery goldens enforce the tokens, and
  the T-17 visual floor reviews the surface against this ADR.
- A true liquid-glass refraction pass is still a future material-track item
  (as in ADR 0091); this decision ships the token-driven approximation and
  says so. (Now scheduled by [ADR 0182](0182-tahoe-liquid-glass-material-pass.md).)
- Apple/Mac-only concepts remain out of scope; the Linux deviations above are
  normative so later tickets do not re-litigate them.