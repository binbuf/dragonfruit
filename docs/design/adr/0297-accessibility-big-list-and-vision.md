# 0297 — Accessibility is a big list of in-place sub-panes

- **Status:** Accepted (T291a)
- **Date:** 2026-11-19
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.5, [24-system-settings.md](../24-system-settings.md) §4.8,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.7,
  ADR [0276](0276-accessibility-pane-accommodations.md),
  ADR [0287](0287-system-settings-visual-parity.md),
  ADR [0295](0295-session-sized-settings-tasks.md)

## Context

The mockup opens Accessibility with intro text and a big list of sections
(Vision, Physical and Motor, Hearing, Speech, General), each opening an
in-place sub-pane. T275 shipped one long scroll of five grouped cards instead.
That form mixed every section on one page, rendered ownerless Apple-only rows
as inert controls, and could not grow one section per session-sized task
without rewriting the page.

## Decision

- **One shared list and loader.** `AccessibilityPane.qml` owns the intro
  contract (the shell `SettingsIntro` from the catalog description), the
  `SettingsBigList`, the `subPaneId`/`subPaneTitle` state, the back button, the
  sub-pane `Loader`, and `openSubPane`/`closeSubPane`/`openRowSurface`. Each
  child adds exactly one big-list row and one sub-pane component; it does not
  fork the list or the loader.
- **One section per task.** T291a ships Vision; T291b Physical and Motor,
  T291c Hearing, T291d Speech, and T291e General and the close-out. A later
  section is not represented by a disabled placeholder.
- **Vision renders only rows with a real owner.** The shipped rows are the
  read-only AT-SPI screen-reader/bridge status, the compositor magnifier
  (`accessibility.magnifier`, `magnifierZoom`, `magnifierMode`), the settingsd
  display accommodations (`increaseContrast`, `reduceTransparency`,
  `differentiateWithoutColor`, `textSize`, `appearance.cursorSize`), the
  motion keys (`reduceMotion`, `dimFlashingLights`), and the speech-dispatcher
  Read & Speak delegate. Ownerless mockup rows are `state: "omitted"` in
  `SettingsMap` with a reason; they are not rendered and not search-indexed.
- **Absence keeps the structure.** A missing AT-SPI host disables the live
  status rows with a reason and shows the note; the durable settingsd rows
  stay live. A present-but-unavailable adapter is a disabled row, never an
  omission.
- **Map rows carry the sub-pane.** `SettingsMap.subPaneForRow()` maps a stable
  row prefix (`vision.`) to the sub-pane id; `rowSurface()` reports `subpane`
  so search and deep links open the body before scrolling. The shell queues a
  requested sub-pane across a pane-body load for every big-list pane, not only
  General.
- **Row identity.** The vendored Phosphor set has no eye mark, so the Vision
  big-list row uses the closest shipped glyph (`scan`) in the same
  `SettingsCategoryIcon` container as every other category.

Rejected: keeping the long five-section scroll; a disabled placeholder for a
later section; rendering ownerless rows as inert toggles; a second row list or
loader per child.

## Consequences

- The Accessibility top-level page is intro + big list; the toolbar title
  becomes the sub-pane title while a body is open.
- Search synonyms are per Vision row; omitted rows never produce a result.
- T291b–T291e add a row and a body, flip their own map rows from `planned` to
  `shipped`, and extend `accessibilityRowSubPanes`.
- The legacy T275 capture now writes the list and Vision; the T291a capture
  adds the reduce-motion, increase-contrast, and absence stills.
