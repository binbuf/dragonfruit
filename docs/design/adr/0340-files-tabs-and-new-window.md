# 0340 — Files tabs are one window's browsing contexts; New Window is a second process

## Status

Accepted (2026-11-23). Implements T333 and the Files window rules in
[09-files.md](../09-files.md#windows-tabs-and-navigation).

## Context

Files shipped as a single-window browser: `FilesBrowser` owned one location,
one history, and the per-location view map. The design's Files menu and window
section promise tabs with independent histories and a New Window row, and
STATUS G-04 names tabs/New Window as the first missing user operation. The
task's design note fixes the shape: a tab is a browsing context, not a second
application, and the title and Dock grouping follow the active tab.

Two things must not regress. `org.dragonfruit.Files1` is the app's
single-instance D-Bus name (T-10.6c); components use it to reach a running
Files. The Dock and the desktop launch a folder with a positional URI and
expect a window even when Files is already running.

## Decision

- **One `FilesTabs` model per window.** Each tab is
  `{ id, title, history, historyIndex }`; the active tab's location is the
  window's location, and back/forward walk that tab's stack. A tab's selection
  is saved by stable tab id and restored on switch; the per-location view kind
  stays in `FilesBrowser`, so each location remembers icon/list. Every model
  edit replaces the array (a QML `var` cannot mutate in place) and bumps one
  revision counter the bindings read.
- **The tab bar is a presentation of that model.** It appears from the second
  tab (or via Show Tab Bar), sits below the unified toolbar, paints
  `{id,title}` rows, and reports selection/close/new-tab/context intent. The
  shell owns the state; the bar owns no pixels beyond its own row.
- **Chords:** `Cmd+T` new tab, `Cmd+W` close the current tab, `Ctrl+Tab` /
  `Ctrl+Shift+Tab` step tabs, `Cmd+1…Cmd+9` activate the nth tab (`Cmd+9` the
  last), `Cmd+N` new window, `Shift+Cmd+N` new folder (New Folder keeps its
  T-10.4c chord; New Window takes the design's File menu row). `Ctrl+Tab` is kept off
  Super/Meta so the compositor's own switcher is never shadowed.
- **Closing the last tab closes the window.** The model empties first, then
  the shell emits the close request; Files never shows an empty browser shell
  in place of a browsing context.
- **New Window is a second process.** `FilesBridge::newWindow` launches this
  executable detached with the normal browser environment, exactly like the
  desktop's folder launch, so the windows do not share a crash domain. The
  first process keeps `org.dragonfruit.Files1`; a second window process runs
  without it (the pre-T333 single-instance trade-off, unchanged).
- **Component activation is a hand-off, not a new window.**
  `--activation-handoff=<token> [--location=<uri>]` asks the running instance
  over `org.dragonfruit.Files1` (`Activate(location)`), which opens the
  location as a tab or activates the tab already showing it; an empty location
  raises the window. A bare launch, a positional URI (the Dock/desktop), and
  `--new-window` always open a window. If nobody answers the hand-off, the
  caller becomes the window.
- **The window title follows the active tab**; a multi-tab window is
  `"<location> — Files"`, a single-tab window is the location name as before.

## Consequences

- Two windows are two processes: one crash cannot take the other down, but
  they do not share window-scoped state (the app-local clipboard, Get Info,
  notices). App-wide clipboard sharing is a follow-up.
- Only the first process answers `Activate`; if it exits, the next launch
  claims the name. A window opened while another process held the name simply
  logs the loss, as before.
- Drag-tab-out, tab reordering, per-window session restore, and a published
  File menu model remain open (G-04). The `menuLaunchCommands` seam already
  produces the New Window/New Tab argv for the T334 menu publication.
- `--location=` is the non-positional location form the hand-off and New
  Window use; `filesLocationArgument` never mistakes an option value for a
  positional location.
