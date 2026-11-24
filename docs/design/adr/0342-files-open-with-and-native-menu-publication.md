# 0342 — Files Open With and native menu publication

Status: accepted (2026-11-24)

## Context

The Files MVP could open a file only through its MIME default
(`filesOpen()`), and it published no native menu model. The item menu had no
Open With affordance, the Get Info row said choosing a default was not
implemented, and the global menu kept the fixed application menu for Files
(STATUS G-04). The design requires an Open With picker with installed handlers,
a default-application path, and the same model the window's menu bar uses
published through the menu broker's native tier (ADR 0041, 06-global-menu.md).

## Decision

- **One handler source.** `OpenHandler.h` gains
  `filesHandlersForMime()`: the MIME's registered desktop ids come from
  `gio mime`, each entry is resolved through the standard application
  directories and must pass the existing launchability check, and the
  configured default (`xdg-mime query default`) is placed first and marked.
  `filesOpenWith()` launches a chosen entry with the file path.
  `filesSetDefaultDesktopId()` writes the association through `xdg-mime
  default` (or `gio mime` when `xdg-mime` is absent). Files never rewrites
  `mimeapps.list` itself.
- **One picker, two paths.** The item menu's **Open With** is a live submenu
  of that list, default row checked. **Choose a Default Application…** opens
  `OpenWithDialog.qml`, which lists the same rows, preselects the default,
  and offers **Open** and **Set as Default**. A file with no registered
  handler shows a disabled "No Applications Available" row and a dialog
  message; a refused launch or write is a visible notice.
- **One menu model.** `FilesMenu.qml` is the single source of the window's
  menus. `FilesShell.menuModel` exposes it and `MenuPublisher` publishes the
  same JSON object to `org.dragonfruit.MenuBroker1` under
  `org.dragonfruit.Files.desktop`. File/Edit/View/Go/Window/Help rows carry
  live enable/check state and the shipped accelerators; every row either acts
  or is disabled with a reason. Open With is the same live handler list, not a
  second copy.
- **One dispatcher.** A broker `Activate` returns to
  `FilesShell.dispatchMenuAction()`, the same function a local row and a test
  use. Shell-owned verbs (hide/close/quit/zoom/bring-all) stay on the shell's
  existing paths.

## Consequences

- The global menu shows Files' real menus while Files is focused, and the
  focus-scoped accelerators are derived from the published rows.
- The picker is testable without spawning an application through the existing
  recording-callable seam; `tst_files_open` covers enumeration, the default
  write, and a chosen-handler launch.
- Deferred: the MIME editor and Default Apps Settings pane, exotic MIME
  handlers, and menu rows whose operations are not implemented (Duplicate,
  Compress, Make Alias, Sort By, Connect to Server). Those rows are either
  absent or explicitly disabled with a reason.
- ADR 0190's default-opening path is unchanged; Open With adds the explicit
  chosen-handler and default-write paths beside it.
