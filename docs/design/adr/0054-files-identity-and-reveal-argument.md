# 0054 — Files identity and the path-reveal argument

## Status

accepted

## Context

The Dock has two navigation paths that must land in the Files app
([09-files.md](../09-files.md), track 10): "Show in Files" reveals an app's
executable, and a Downloads-stack row opens a downloaded file "through Files"
([10-files-mvp.md](../tracks/10-files-mvp.md)). Both need (a) an installed
app identity the interim `.desktop` resolver can find and (b) a contract for
"open this path and reveal this item" without a running-instance API. Files
already accepts one positional location argument (`dragonfruit-files trash://`,
T-10.6b), but it has no way to ask a window to select an item inside a folder.

## Decision

- **Install the identity.** `apps/files/org.dragonfruit.Files.desktop`
  (`Exec=dragonfruit-files %U`, `DBusActivatable=true`, `MimeType=
  inode/directory;`) and `apps/files/org.dragonfruit.Files1.service` are
  installed by CMake to `${CMAKE_INSTALL_DATADIR}/applications` and
  `/dbus-1/services`. The app owns the well-known name `org.dragonfruit.Files1`
  on the session bus. The shell keeps launching through the `.desktop` Exec
  until the running-instance `OpenPaths`/`RevealItems` API (T-18) replaces it.
- **A file argument implies a reveal.** A positional argument that names an
  existing file is resolved to its parent directory plus that file
  (`filesOpenTarget`, `apps/files/FilesArguments.h`). A directory or a
  non-`file://` scheme (`trash://`) browses itself with no reveal. The seam is
  carried to QML as `Files.revealUri` / `DF_FILES_START_REVEAL`.
- **The shell resolves the executable.** `revealExecutable`
  (`shell/src/filestarget.cpp`) takes the first token of the app's launch
  command and makes it absolute, resolving a bare program name on `PATH`.
  Show in Files and a Downloads-stack row both hand that absolute path to
  `launchFiles`, the single Files launch path.
- **The window selects the reveal once the listing settles**, scrolls to it,
  and does nothing when the item is absent.

## Consequences

- One argument contract serves the Trash click, Show in Files, and Downloads:
  a file reveals, a directory browses. Later tasks reuse it for the portal and
  desktop surfaces.
- Reveal selection compares the exact `file://` URI; non-ASCII names rely on
  Files and files-core encoding the same path the same way (they do today).
- The identity files are the first CMake `install()` rules in the tree; Qt
  packaging and the D-Bus name's running-instance methods are still T-18.