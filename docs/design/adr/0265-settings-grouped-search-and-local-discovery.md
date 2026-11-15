# 0265 — Settings search is one precomputed, pane-grouped local index

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-15 |
| **Extends** | ADR [0232](0232-settings-search-and-deep-links.md), [0264](0264-settings-navigation-history-deep-links-help.md) |

## Context

T223 shipped a flat ranked list over the map. It matched pane titles but not
pane ids or the reference names users still say (`control center`, `mission
control`, `time machine`, `touch id`, `internet accounts`, `autofill`), treated
`wi-fi`/`wifi`/`Wi Fi` as different words, used the pane's absence note as an
available pane's subtitle, and let owner/key/absence help text compete with the
setting's own name. A pane match also flooded the list with every row in the
pane, the index re-normalized every field on every keystroke, and the sidebar
filter rebuilt on each character.

## Decision

1. **One index, normalized once.** `SettingsSearch.buildIndex()` stores the
   normalized and hyphenless (`compact`) form of the label, subtitle, section,
   pane title, help text, synonyms, and technical terms. A keystroke only
   compares precomputed strings.
2. **Hyphen/space variants are one word.** Matching tries the normalized form
   and the compact form, so `wi-fi`, `wifi`, and `Wi Fi` all reach the Wi-Fi
   pane. Pane ids are aliases; the reference names are lowercase synonyms.
3. **Pane synonyms belong to the pane entry, not to its rows.** Otherwise a
   pane alias makes every row a match (`time machine` returned 48 General rows
   and hid the Backup rows). Rows index their own synonyms, label, section,
   pane title, and help.
4. **Technical help is a lower band.** Owner, key, absence, and control terms
   stay searchable, but a result with no user-word match is marked
   `technicalOnly` and sorts below every primary match.
5. **Subtitles are truthful.** A pane uses its catalog description (or sidebar
   group); a row uses its section. The absence note is help text only.
6. **The result model is pane-grouped and capped.** Each pane group shows at
   most four entries; a capped group ends with a `showAll` entry that expands
   that pane in place. The first entry of a group carries `groupStart` for the
   header, and `SettingsSearch.resultCount()` excludes the affordances.
7. **The field is immediate; the models are debounced.** `SettingsShell`
   copies `searchText` into `searchQuery` after 120 ms (immediately when the
   field empties). The index scan and the sidebar filter read `searchQuery`;
   clearing restores the sidebar at once.
8. **Keyboard and AT-SPI are one path.** Down from the field enters the list;
   Up/Down/Home/End/Return/Escape work; Escape returns focus to the field and
   then the sidebar. The list is an AT-SPI list whose items name their pane and
   section, the no-match state is announced, and the clear affordance has an
   accessible name.

## Consequences

- A new pane alias goes in `paneSynonyms`; a new row alias goes in
  `rowSynonyms`. Adding a pane synonym does not affect row matching.
- A new pane or row is searchable only through `isRowLandable()`, so the
  resolver and the index stay in lockstep.
- The per-pane cap is a single constant (`paneLimit`); the `Show all` row is
  model data, not a second view.
- Tests that drive `searchText` directly must call `commitSearch()` or wait for
  the debounce; the keyboard test exercises the debounce itself.

## References

- [24-system-settings.md](../24-system-settings.md) §3.5
- [08-settings.md](../08-settings.md) search section
- [0264](0264-settings-navigation-history-deep-links-help.md)
