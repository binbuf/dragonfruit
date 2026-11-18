# 0292 — General big-list identity, omitted owners, and the fixed big-row height

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-17 |
| **Task** | T287 |

## Context

The 2026-11-17 System Settings mockup opens General with an intro and a card
of 52 pt icon rows. The existing General list used three `SettingsGroup`s of
plain 33/52 pt `SettingsRow`s with live per-row summaries. The mockup also
lists Device Management, which has no Linux MDM/profile owner in this build,
and several About/Passwords/Language/Sharing rows whose macOS owner has no
Linux consumer.

`SettingsBigRow` already existed (T286) but its optional second line made the
label column one pixel taller than the 52 pt token, so a row with a sub line
measured 53 pt.

## Decision

- General renders one `SettingsBigList` of `SettingsBigRow`s in the mockup
  order. Every row carries a category tile resolved by
  `SettingsPanes.rowStyle(name)`, which maps the mockup identity to the
  closest vendored Phosphor glyph. The sidebar tile and the big-list tile use
  the same `SettingsCategoryIcon` component.
- Only Passwords & Keys and Backups carry a second line, as in the mockup.
  Other rows do not repeat a live summary that the sub-pane already owns.
- Device Management is not rendered. `SettingsMap` records it as
  `state: "omitted"`, `absence: "omitted"` with the reason “no Linux
  MDM/profile owner in scope”. The same pattern records every other ownerless
  mockup row (Energy Compliance, Beta Updates, AutoFill Passwords and Keys,
  Delete After Use, Text in Images, Applications, Translation Languages,
  Content Caching, Bluetooth Sharing, Remote Application Scripting).
- `SettingsRow` gains `fixedRowHeight`. `SettingsBigRow` sets it so the big
  row is exactly the 52 pt token; a row that is not a big-list row keeps the
  grow-to-fit behavior for long localized labels.
- General's catalog description is empty. The pane body owns the mockup intro;
  the shell must not paint a second intro above it.

## Consequences

- A later category task can add rows to the same big list without forking the
  row component; a new row needs a category style and a map row.
- Omitting a row is executable: `SettingsMap.validate()` rejects an omitted
  row without an `omitted` absence, and search never indexes it.
- The fixed height trades a one-pixel growth path for the measured token. A
  future big row with a multi-line label must opt out of `fixedRowHeight` or
  shorten the label.
