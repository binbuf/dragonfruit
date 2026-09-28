# 0145 — The Accessibility pane and tile ride the bridge host

- **Status:** Accepted (T-15.14b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0144](0144-accessibility-adapter.md),
  ADR [0143](0143-privacy-security-pane-and-tile.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.14a landed `dragonfruit-accessibility-adapter`, the read-only projection of
the AT-SPI accessibility bus (`org.a11y.Status`: `IsEnabled`,
`ScreenReaderEnabled`). T-15.14b must ship the Settings pane and the Control
Center tile as one functional unit. The macOS capture lists a `Vision` card
(VoiceOver, Zoom, Hover Text, Display, Motion, Read & Speak, Audio Descriptions)
and a `Hearing` card; on Linux only the live bridge status has a host owner, and
the durable presentation preferences live in `settingsd` while magnification is
compositor-owned (ADR 0144). As with Privacy (ADR 0143), the C++/QML side never
links the Rust adapter: it decodes the bridge host's JSON view.

The Control Center panel already held sixteen tiles at the ceiling of its fixed
360×1160 surface; a seventeenth had to fit without clipping and without a
scrolling panel (the compositor forwards no pointer-axis events).

## Decision

- **One new bridge interface, no new daemon.** `services/system-status` serves
  `org.dragonfruit.SystemStatus1.Accessibility` at the shared object path. Its
  `AccessibilityHost<HostAccessibility>` wraps the T-15.14a adapter and projects
  one flat `accessibility` view: the two booleans, their `On`/`Off` labels, the
  combined `label` (`Screen Reader On` / `On` / `Off`), and `present`. The
  interface is **read-only** (`State`/`Refresh`); `org.a11y.Status` has no
  setter, and the pane's one durable preference goes through `settingsd`.
- **The durable preference is the existing settingsd key.** The pane's
  `Reduce Motion` toggle writes `accessibility.reduceMotion`, which the shell
  theme, the Dock, and the compositor already apply live. No new settingsd key
  and no schema bump: a compositor magnifier and a durable display-contrast
  preference have no host owner yet and are left as a follow-up rather than
  shipped as dead rows.
- **Absence is single-layered.** The view is `unavailable` only when there is
  no session bus or no `org.a11y.Bus` owner; an all-off bus is `available` with
  `present: false` (the tile's second hide rule). The pane shows a one-line
  absence note beside the still-live `Reduce Motion` row; the tile hides on
  `unavailable` or `present: false`.
- **The pane mirrors the reference honestly.** A grouped `Vision` card carries
  `Screen Reader` (the reference `VoiceOver`, dropping the Apple brand name) and
  `Accessibility` (the toolkit bridge) as read-only live status rows, and the
  `Motion` group carries the `Reduce Motion` toggle. The `Hearing` card and the
  Apple-only rows (`Zoom`, `Hover Text`, `Display`, `Read & Speak`, `Audio
  Descriptions`, `Live Captions`, `Name Recognition`) are omitted (ADR 0122).
- **The Control Center tile is a read-only summary.** It carries the new
  original `accessibility` glyph, the live label, and opens the pane when
  tapped. It is a compact single-row tile (no separate link line), the panel's
  internal gap compacts once more from `xs` to `xxs`, and the fixed surface
  holds all seventeen tiles.
- **The design system gains one glyph.** `Icon` grows an original
  person-in-circle `accessibility` mark for the sidebar row, the pane header,
  and the tile.

Rejected: a second accessibility implementation in the shell (the adapter owns
the AT-SPI projection); a compositor magnifier or a contrast preference invented
for this task; re-enabling the inert menu-bar `accessibility` status item (it is
a separate surface, T-16); hiding the whole item when every feature is off
(off is a value, not a missing daemon).

## Consequences

- `AccessibilityInterface` joins `interface_names()` (now twelve); `main.rs`
  gains `--print-accessibility`; `services/system-status` depends on
  `dragonfruit-accessibility-adapter`.
- `apps/settings/AccessibilityClient` is the pane's seam
  (`DF_ACCESSIBILITY_FIXTURE` selects the in-process mock);
  `shell/src/systemstatusclient`, `systemstatusmodel`, and `shellcontroller`
  gain the read-only accessibility path.
- The Control Center panel stays a non-scrolling fixed surface at 360×1160; the
  seventeenth tile fits by compaction, recorded in the panel comment.
- The compositor magnifier and the display-contrast rows remain a follow-up;
  this task ships the live bridge projection and the one durable preference.