# 0302 — Accessibility General: the recorded shortcut, and the pane close-out

- **Status:** Accepted (T291e)
- **Date:** 2026-11-20
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.5, [24-system-settings.md](../24-system-settings.md) §4.8,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.7,
  ADR [0276](0276-accessibility-pane-accommodations.md),
  ADR [0297](0297-accessibility-big-list-and-vision.md)

## Context

T291a–T291d shipped Vision, Physical and Motor, Hearing, and Speech as rows of
one big list of in-place sub-panes. The mockup's last section, General, was
carried in `SettingsMap` as a single `planned` row. The pane therefore still
had a section with no body, and the list could not be called complete. The
build already has the owner: the durable `accessibility.shortcut` key, which
the compositor's shortcut engine parses (`parse_binding`) and installs, and the
`accessibility-shortcut` action that opens this pane. The Settings app already
has one chord editor, `ShortcutRecorder` (T275).

## Decision

- **General is the fifth and last big-list row,** after Speech, in mockup
  order, with a gear tile in the same accessibility blue. It opens the
  **General** in-place sub-pane.
- **One group, one live row.** **Accessibility Shortcut** holds **Accessibility
  shortcut**, backed by `accessibility.shortcut`. It reuses the existing
  `ShortcutRecorder`; no second recorder and no new key. The row is
  `live-on-defaults`, not stored-only, because the compositor consumes the
  chord; Backspace clears it and disables the shortcut.
- **No absence state.** The recorder writes a settingsd preference, so it stays
  live with no AT-SPI host and no delegate. The General body therefore has no
  absence note, unlike Vision.
- **The pane ships no `planned` rows.** Every mockup section is either rendered
  from its shipped rows or represented only by `omitted` rows with a reason;
  an omitted row is never rendered, landable, or search-indexed. A section with
  no shipped row is not rendered as an empty heading, and the list has no
  disabled placeholder and no duplicate section.
- **Close-out evidence is one case and one capture target.**
  `tst_settings_accessibility` walks the whole pane (list order, every
  sub-pane, every shipped/omitted row, the section table), and
  `make t291e-settings-accessibility-general-capture` writes the complete list,
  every sub-pane, the recorded shortcut, the reduce-motion/increase-contrast
  states, and the absent-host list. The earlier per-child captures stay.

Rejected: a second shortcut editor or a General pane that duplicates the Search
pane's shortcut; deleting the General section (its row has a real owner and a
consumer); rendering the ownerless sections as disabled placeholders.

### The absent update host has to answer

The close-out gate exposed a pre-existing defect the Accessibility row depends
on indirectly (the sidebar's update status row). The live updates client
returned from its constructor without publishing a view when the session bus
was missing, so the shell could not tell "absent host" from "not read yet" and
kept the neutral catalog label. The client now publishes the empty (absent)
view through the same `changed` path the mock uses, on a bus-less session, on a
bus without the host, and when a read fails against a vanished host. Absence is
already a normal state; it now also resolves.

## Consequences

- `SettingsMap.accessibilityRowSubPanes` maps the `shortcut.` prefix to the
  `general` sub-pane; `accessibilitySectionSubPanes` accepts "Accessibility
  Shortcut" and "Shortcut".
- `tst_settings_accessibility_general_absence` proves the row stays live with
  no bridge host; `tst_settings_absence` pins the five-row list.
- A later section that gains an owner adds one `SettingsBigRow`, one body, and
  its map rows — never a placeholder.
