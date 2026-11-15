# 0267 — Settings accessibility is one keyboard map and one focus contract

- **Status:** accepted
- **Date:** 2026-11-15
- **Task:** T267

## Context

The System Settings shell, sidebar, search results, rows, popups, dialogs, and
debug view each carried their own key handlers and partial AT-SPI metadata.
Some overlay focus scopes were Tab stops with no ring, the search clear
control was pointer-only, dialog/sheet focus was not restored, and RTL/scale
behavior was untested. The pane-level accessibility work in T275 cannot repair
a shell contract that panes inherit.

## Decision

1. **One keyboard table.** `design-system/Keymap.qml` is the single map for
   the shell, sidebar, list, search-field, and "any" scopes. Components ask it
   for an action instead of re-declaring keys. The map covers Ctrl+F,
   Alt+Left/Right, Escape, Up/Down, Home/End, Return/Space, and Tab/Shift+Tab.
2. **Overlay scopes are not Tab stops.** `Dialog`, `Sheet`, `Popup`,
   `Popover`, and `ContextMenu` own focus and trap/restore it, but their root
   `activeFocusOnTab` is false. Visible rings belong to the content inside.
   Every real Tab stop shows a `FocusRing`; the search clear control and an
   absence-only pane's note are real, ringed stops.
3. **Focus restoration is mandatory.** Pane/sub-pane navigation, search
   activation, Back/Forward, and dialog/sheet close return focus to the first
   stop, the landed row's control, or the opening control respectively.
4. **AT-SPI is named and announced.** Role, name, description, and current
   value/range are present on the shell, sidebar, results, rows, controls,
   popups, dialogs, sheets, debug view, and absence notices. Validation errors
   and absence notes are `AlertMessage` regions and are announced once when
   they appear; result-count changes are announced once per query.
5. **The app mirrors the shell's accessibility keys.** `accessibility.*` is
   bound live into `Theme` and `Glass`; Reduce Motion collapses every motion
   token to 0, Reduce Transparency takes the opaque plate, and Increase
   Contrast boosts material edges plus secondary/tertiary ink and separators.
6. **RTL is explicit.** The shell enables `LayoutMirroring` and inherits it;
   text alignment is set explicitly because Qt does not mirror it. The
   sidebar/detail split and row label/control sides swap; control order is not
   reversed. A disclosure chevron points left.
7. **Scale is tested with the same QML.** `tst_settings_scale_1x/1_5x/2x`
   assert `Screen.devicePixelRatio` and lay out 740×820 and 720×480 in logical
   points. A screenshot scaled after the fact is not evidence.

## Consequences

- T275 and later pane tasks inherit the map and focus contract; they must not
  add a second key handler or a pane-local focus policy.
- T283's evidence matrix consumes the baseline/high-contrast/reduced-
  transparency/reduced-motion/RTL/2× stills and the scale cases.
- The offscreen 2× PNG is clipped to logical size by Qt's `grabImage`; the 2×
  QML case remains the executable evidence, and a physical 2× still is an
  explicit `OPEN` row for a host Wayland capture.
- A third-party AT-SPI app that does not expose a control, Orca/speech-
  dispatcher integration, and lock/screen-saver accessibility remain `OPEN`
  owners (T252/T278/T284), not silent gaps.
