# 0231 — The Settings map is executable; the catalog mirrors it

- **Status:** accepted
- **Date:** 2026-11-07
- **Task:** T222
- **Companions:** [settings-business-logic-map.md](../settings-business-logic-map.md), [08-settings.md](../08-settings.md), ADR [0035](0035-settings-shell-and-pane-catalog.md)

## Context

The Settings business-logic map was a design document only. The pane catalog
(`SettingsPanes.qml`) carried `shipped` flags but no owner, deep link, or row
metadata, so a new row could land without an owner or an absence path and the
sidebar could drift from the map silently. T223–T229 need one machine-readable
source for pane/section/row search and deep links.

## Decision

`apps/settings/SettingsMap.qml` is the executable projection of the map. It
owns the pane fields (id, label, icon, shipped flag, owner,
`settings://pane/<id>` deep link, absence kind) and row metadata
(`{ id, section, label, control, owner, key, absence, state }`). Row `state` is
`shipped`, `planned`, or `omitted`; only `shipped` rows are visible today.

`SettingsPanes.qml` remains the one ordered sidebar catalog. Its entries mirror
`SettingsMap.panes` one-for-one; `tst_settings_map.qml` fails on drift, on a
shipped pane without a body, on a shipped pane without a visible row, and on a
visible row with no owner/key/absence path. The test also feeds
`SettingsMap.validateRow()` an intentionally unowned row so the gate is proven
to fail. Omitted panes live only in `SettingsMap.omittedPanes` with a reason,
so they cannot appear in the sidebar.

The existing `storage`, `mission-control`, and `menu-bar` panes keep their ids
and top-level placement. New panes use the map names; user-facing titles follow
the T220 generic naming map. The Help menu opens a debug view of pane/row
owners and links the human document; `DF_SETTINGS_DEBUG=1` opens the same view
on launch.

## Consequences

- Adding a Settings row now means adding it to `SettingsMap` with an owner, key
  or API, absence kind, and state; a visible row without those fails the test.
- T225–T229 flip a row/state and ship its body together. The catalog `shipped`
  flag still gates the sidebar and the body registry.
- The map document and `SettingsMap.qml` are intentionally duplicated at the
  pane/row label level; the test is the drift guard. Change both in one commit.
- The debug view is a diagnostic surface, not a user-facing settings pane; it
  stays out of the catalog and the sidebar.
