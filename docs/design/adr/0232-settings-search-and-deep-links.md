# 0232 — Settings search is a local map index; deep links land on a row

## Status

accepted

## Context

T222 landed the executable Settings map (`SettingsMap.qml`) and the 29-entry
pane catalog, but the shell's search still filtered the sidebar pane list by
title/id/description only. There was no way to find a row such as "Show
previews", no `settings://` consumer, and no shared row anatomy: panes drew
labels, controls, chevrons, and trailing buttons by hand. The macOS reference
also has a bottom-right `?` help control, which the earlier pane tasks omitted
project-wide rather than shipping a dead button.

T223 has to make Settings navigable without adding a second search service or
duplicating the map's information architecture.

## Decision

- **One local index, built from the executable map.** `SettingsSearch.qml`
  indexes every shipped pane title, every map section, and every shipped map
  row. Each entry carries its pane/section context, a deep link, pane/row
  synonyms, and help text assembled from the row's owner, key/API, absence kind,
  and the pane's absence note. Search is a pure synchronous scan of that array;
  there is no bus call, daemon, or per-keystroke I/O. Unshipped panes and
  `planned`/`omitted` rows are not indexed (the no-half-panes rule).
- **Ranking is field-weighted.** Exact label > label prefix > label word start >
  label substring, then synonyms, section, pane title, and help text. A pane
  whose own name matches the query sorts first, so "display" opens Displays
  before an Accessibility row called Display, and "battery" opens Battery before
  the menu-bar Battery checkbox. Technical key/control terms are weighted below
  the user's words.
- **Deep links are the result contract.** `settings://pane/<id>` and
  `settings://pane/<id>/row/<rowId>` are parsed by the shell. A row link is
  accepted only when the map knows the row and marks it shipped. The shell
  swaps the pane, then finds the rendered `SettingsRow` by `settingsId` first
  and by label/synonym second, scrolls the detail view to it, paints an accent
  wash, and moves keyboard focus into its control. If a pane's row is custom
  chrome rather than a `SettingsRow`, the link lands on the matching
  `SettingsGroup` section; if no section exists it lands at the pane top. A
  result is never a dead link.
- **One row anatomy.** `SettingsRow` owns label, description, control,
  disclosure chevron, trailing action, and help affordance in one component.
  `settingsId`, `owner`, and `settingsKey` link a rendered row back to the map;
  `highlighted` is the deep-link landing state. Long labels elide and long
  descriptions wrap; the control slot stays inside the row.
- **Search results replace the sidebar list while typing.** The ranked list
  supports Up/Down/Home/End/Return/Escape, exposes AT-SPI list items, and shows
  an explicit no-results state. The search field's Down key hands focus to the
  list, so the whole path is keyboard-only.
- **Help is a live document anchor.** The bottom-right `?` control opens
  `Settings.helpUrl(paneId)` — the business-logic map with a `#pane-<id>`
  anchor. When the document is not installed the control disables instead of
  opening a dead link. The four reference header panes (General, Accessibility,
  Notifications, Privacy & Security) keep their header card; settings-list
  panes stay headerless.

## Consequences

- T225–T229 should give each new row its map `settingsId`, `owner`, and
  `settingsKey`; the index and deep links then work without another search
  change. Rows that remain custom chrome still get a section/pane landing.
- Row synonyms live in `SettingsSearch.rowSynonyms` until a later IA pass moves
  them beside the map rows; changing a map label or id must update the search
  test and, if present, the synonym key in the same change.
- The map document's `#pane-<id>` anchors are part of the help contract; keep
  one anchor per catalog id.
- The deterministic visual check is
  `make t223-settings-search-capture`, writing
  `docs/captures/t223-settings-{search,deep-link,long-strings}.png`; the
  headless test `tst_settings_search` covers the three-row deep-link sweep,
  keyboard navigation, history, no-results, and long strings.
