# 0291 — The Settings shell geometry is a token opt-in, and the shell components are the pane vocabulary

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-17 |
| **Task** | T286 |
| **Supersedes** | — |
| **Related** | [0287](0287-system-settings-visual-parity.md), [0266](0266-settings-component-contracts.md), [0290](0290-settings-ia-aliases-and-status-row.md) |

## Context

T286 applies the mockup's shell geometry to System Settings: a left-aligned
title beside a 78 × 36 history pill, 20/24 pt content insets, a 232 pt sidebar
with 32 pt rows and a neutral selection, 33/52/53/42 pt row variants, a
24 pt help circle, and a set of shared pane components. The shell components
are shared with Files, the Applications drawer, and the gallery, so changing a
shared default would regress those surfaces. The category tasks (T287–T301)
must not fork the row anatomy or the shell chrome.

## Decision

1. The mockup geometry is data in `design-system/tokens/tokens.json`
   (`settingsWindow`, `settingsSidebar`, `settingsRow`, `settingsGroup`, and
   the `settingsRadio`/`settingsScale`/`settingsModeTile`/`settingsWallTile`/
   `settingsAvatar`/`settingsChart` slots). `Theme.qml` and
   `compositor/src/design_tokens.rs` are regenerated; the Settings app has no
   app-local geometry constants.
2. Settings opts into the density through component properties: `TitleBar`
   gains `titleAlignment`/`titleFontSize`/`titleFontWeight`/`titleElide`/
   `titleGap`/`leadingInset`; `Sidebar` gains `rowHeight`/`rowRadius`/
   `padding`/`sectionSpacer`/`selectionStyle`/`neutralSelectionFill`/
   `scrollSelectedIntoView`. The shared defaults (centred title, 28 pt accent
   selection) are unchanged for every other consumer.
3. The neutral selected row is a Settings style, not a global accent change.
4. The pane vocabulary lives in `design-system/components/`: `SettingsIntro`,
   `SettingsBigList`, `SettingsBigRow`, `SettingsIconRow`, `SettingsFooter`,
   `SettingsNote`, `SettingsBanner`, `SettingsServiceRow`, `SettingsScanRow`,
   `SettingsRadioGroup`, `SettingsScaleSlider`, `SettingsModeTile`,
   `SettingsNavPill`, `SettingsHelpButton`, `SettingsSidebarStatusRow`,
   `SettingsCollectionHead`, `SettingsWallTile`, `SettingsChart`,
   `SettingsHero`, `SettingsStorageBar`, `SettingsAvatar`,
   `SettingsDeviceRow`, and `SettingsDiskRow`. A category task may add a
   component it owns, but it lands there and not in a pane-local copy.
5. The detail toolbar is the content surface; the sidebar keeps the
   `settingsSidebar` material and the Reduce Transparency / Increase Contrast
   resolver.
6. The update status row stays visible with the provider's honest label
   (including "Up to Date" and "Unavailable"); it is not hidden or faked.
7. The help control opens the live business-logic map anchor. The mockup's
   placeholder help text is not shipped; with no document the control disables
   and explains why.

## Consequences

- Later pane tasks consume these components and tokens; a pane that re-rolls a
  row, card, radio, slider, or status row is out of contract.
- A shared-default change must be opt-in or it regresses Files and the drawer.
- The shell geometry is testable in logical points at 1×/1.5×/2×, in RTL, and
  under increase contrast/reduced motion without pixel comparisons.
- The `settingsToolbar` glass preset remains in the token set for other
  consumers and for the increase-contrast edge; the shell no longer paints a
  second chrome sheet.
