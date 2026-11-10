# ADR 0247 — Files measured anatomy and the Get Info inspector

## Status

Accepted (T230d, 2026-11-09).

## Context

The 2026-11-08 structural review measured Finder at 900×600 pt: a 52 pt
toolbar, a 198 pt sidebar, content-layer tab/path/status bars (28/30/28 pt),
a 64 pt content inset when the toolbar is hidden, and a 328×1034 pt Get Info
auxiliary window with a 32 pt title bar. Files already had the T230b unified
toolbar and T206 app-local chrome material, but its sidebar was 240 pt, it had
no status bar, no toolbar-hidden mode and no inspector. The view switch had
only two live segments and no explanation for the missing Column/Gallery
layouts.

## Decision

1. **Measured geometry is a token, not an app constant.** `component.files`
   holds `sidebarWidth` (198), the 164×52 pt view group, its 40×36 pt segments
   and 2/8 pt padding, `toolbarHiddenInset` (64), and the Get Info
   328×1034/32 pt dimensions. `SegmentedControl` accepts optional measured
   geometry so Files does not fork the component.
2. **The path and status bars are content.** The window stays a 52 pt unified
   window; the 30 pt path bar and 28 pt status bar live below the content and
   do not change the chrome height.
3. **Toolbar-hidden is a mode.** It hides the toolbar controls and the
   sidebar, grows the chrome/content start to the measured 64 pt, and lets the
   status bar span the full window width. It is reversible from the background
   menu and `Ctrl+Alt+T`.
4. **Disabled view segments carry a reason.** Column and Gallery stay in the
   group but are disabled, expose a reason to AT-SPI, show it on hover, and
   raise the inline notice when activated. The live Icon/List choice keeps the
   existing per-location view state.
5. **Get Info is one data-driven column.** The shell builds seven sections
   from the live selection (`General`, `More Info`, `Name & Extension`,
   `Comments`, `Open with`, `Preview`, `Sharing & Permissions`). A single item
   uses `FilesDirectoryModel::nodeInfo`; a multi-selection uses
   `selectionInfo` for count, folder/file split and summed file size. Only
   Name & Extension has a real backend (the files-core rename path); every
   other unavailable control is an explicit row with a reason.
6. **The material pairs stay as measured.** Files keeps the T206 sidebar and
   toolbar alpha/tint pairs and does not use the 0.55 popover alpha.

## Consequences

- The Files window has one measured anatomy that later view work (T244a) can
  extend without changing the bars; the column/gallery segments already carry
  their owner and rerun.
- `SegmentedControl` gains a rejected-state contract that other measured
  segmented controls can reuse; existing consumers keep the token defaults.
- Get Info is a separate window, not a sheet or Quick Look. Quick Look remains
  T244 and can share the selection model but not this surface.
- Tabs, search results, tags, customization and motion stay explicit `OPEN`
  rows in `09-files.md` and the structural review; the capture runner is
  `make t230d-files-capture` and the behavior test is
  `ctest -R tst_files_shell`.
