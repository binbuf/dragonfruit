# 0293 — Battery and Displays follow the mockup order; sheets own the extra rows

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-18 |
| **Task** | T289 |

## Context

The 2026-11-17 mockup reorders Battery and Displays around a short main flow
and moves secondary controls into sheets. The panes already owned the real
adapters (UPower + power-profiles-daemon; the compositor output list, the
capability table, and the `display.*` keys), but their rows were grouped by
owner rather than by the mockup's task order. Two mockup rows had no direct
Linux owner: a battery status pill with a clock timestamp, and a projector
section that must appear only when a projector is attached.

## Decision

- Battery renders the mockup order: Low Power Mode, Battery Health with
  `Show Detail`, the `power.historyRange` segmented range, a live status pill,
  the shared `SettingsChart` Battery Level chart, and an Options… footer. The
  pill reads UPower's charge state and the real time estimate; UPower exposes
  no last-update timestamp, so the pill's second line is the estimate rather
  than the mockup's example clock time. The old Charging row is folded into
  the pill.
- The Options… sheet owns the `power.*` policy rows, the Linux-only Power
  Adapter values, and the Lock Screen link. They stay live when UPower is
  absent. No second power-policy engine is added.
- Displays renders the display icon row (with the per-output sheet behind its
  chevron), Resolution radios over the five real scale steps, a
  “Show all resolutions” view filter, the scaling note, Brightness, automatic
  brightness, Ambient color (adaptive color), Preset (the color-profile path),
  Refresh rate, the projector section, and Advanced…/Night Light… sheets. The
  large preview art is removed; the display row is the identity. Arrangement,
  Rotation, Use as, and the advanced capability rows live in the Advanced…
  sheet, and the Night Light rows live in their own sheet.
- “Show all resolutions” is a view filter over the existing five steps, not a
  new settingsd key: the list has no hidden pixel-mode registry to reveal.
- The bridge adds one field to each output entry, `projector`. It is derived
  from the EDID-derived text Qt exposes (screen name, model, manufacturer)
  matched against the words a TV or projector reports; `DF_DISPLAY_FIXTURE=
  projector` is the deterministic test/capture seam. The Displays pane shows
  the projector row only for a marked output and an absence row with the
  reason otherwise. It never guesses from a resolution or a connector count.

## Consequences

- A later task that adds a real pixel-mode list or a connector-type signal
  replaces the `projector` heuristic and the resolution radio labels; the
  pane structure does not change.
- A sheet row is still a rendered row: every row carries a `SettingsMap`
  entry, and the pane tests walk the pane and its sheets so a new control
  cannot ship unmapped.
- The mockup's example text (“Fully Charged”, the clock time) is not copied;
  the pill always shows the host's state.
