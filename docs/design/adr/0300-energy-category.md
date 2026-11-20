# 0300 — Energy is one category for laptops and desktops

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-20 |
| **Task** | T289 post-acceptance (label and logic follow-up) |

## Context

macOS uses one System Settings pane with two labels: **Battery** on a Mac with
a battery, **Energy** on a Mac desktop. The 2026-11-17 visual target was
captured on a MacBook, so the mockup says Battery. Dragonfruit ships on both
laptop and desktop hardware, and the category owns both the battery readings
and the power policy.

The pane id `battery` is persisted in pane history, deep links
(`settings://pane/battery`), the executable map, search synonyms, tests, and
capture scripts. The visible label and the id do not have to match.

## Decision

- The sidebar title is **Energy** on every machine. The id stays `battery`.
  `settings://pane/energy` is an alias for the same pane.
- The pane's content is capability-driven. UPower and power-profiles-daemon are
  independent owners; a missing daemon is an in-pane absence, not a hidden
  category. A desktop with no battery still selects a profile and owns the
  session/firmware policy rows.
- The pane keeps the mockup's laptop row order. Desktop-only macOS Energy rows
  are added only when a real owner exists; until then they are omissions.
- Deliberate divergences from the MacBook capture are recorded in
  [`26-settings-logic.md`](../26-settings-logic.md) §2.5. Later visual-parity
  work must not delete a host-backed row or rename the category to make a
  laptop capture match.

## Consequences

- The T289 stills and the mockup keep the word "Battery"; they are reference
  artifacts, not the current label. New captures and the T302a evidence matrix
  use Energy.
- Search already maps `battery`, `power`, and `energy` to the pane; the alias
  adds the deep link, and the pane title is the single visible label.
- The behaviour split and the open desktop rows live in
  [`26-settings-logic.md`](../26-settings-logic.md), which is the authority for
  content when the mockup and the host disagree.
