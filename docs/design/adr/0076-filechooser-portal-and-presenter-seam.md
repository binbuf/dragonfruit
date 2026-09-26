# 0076 — FileChooser portal and the presenter seam

## Status

accepted

## Context

T-13.2a adds the third concrete portal interface to the backend: the standard
`org.freedesktop.impl.portal.FileChooser`. Unlike Settings and GlobalShortcuts
(ADR [0075](0075-settings-and-globalshortcuts-portals.md)), which answer
immediately from state another service already owns, a file chooser is
**interactive**: the standard method does not return until a user has chosen.
The design delegates browsing to Files in chooser mode over the one
`files-core` implementation ([09-files.md](../09-files.md),
[07-system-integration.md](../07-system-integration.md)). The picker UI itself
is T-13.2b, so T-13.2a must define the seam between the portal and a
presenter, and must be testable with no pixels.

## Decision

- **The registry is the seam.** `portal::chooser` owns a pure
  `ChooserRegistry` of live requests. Each standard method registers a
  `ChooserRequest` (handle, kind, app id, parent window, title, decoded
  options) and awaits a one-shot `ChooserCompletion`. A presenter resolves it
  by handle — `complete` with a selection, or `cancel`. A duplicate handle is
  refused so a caller cannot hijack a request. The D-Bus object is a thin
  wrapper; the model is unit-tested without a bus.
- **The presenter is deferred.** T-13.2a ships no dialog. The diagnostic
  `org.dragonfruit.Portal1` surface carries the presenter's half: the
  `FileChooserOpened(handle, kind, app_id, parent_window, title, options)`
  signal when a request waits, and
  `CompleteFileChooser(handle, uris)` / `CancelFileChooser(handle)` /
  `PendingFileChoosers`. The test client is the presenter in T-13.2a; the
  T-13.2b design-system picker takes the same three calls.
- **files-core normalizes and lists.** Selections (a local path or a URI) are
  run through `dragonfruit-files-core::Location` and only a canonical
  `file://` URI survives; anything else is discarded, per the portal spec's
  "backends must normalize URIs ... into file:// URIs". The diagnostic
  `ListDirectory` projects the picker's browsing through
  `DirectoryModel`/`StdFsSource`, the same streaming model and collation
  Files uses. The portal never re-implements browsing.
- **Response codes are the portal's.** Success 0 with a `uris` result (and
  `writable=false` for open), cancellation 1, other error 2. A selection that
  normalizes to nothing is an error response, never a silent success.

## Consequences

- `model::BACKEND_INTERFACES` and `dragonfruit.portal` now list
  `org.freedesktop.impl.portal.FileChooser`; Screenshot and ScreenCast append
  theirs in T-13.3a/T-13.4a. The backend name, path, and data-file identity
  are unchanged.
- `xdg-desktop-portal-dragonfruit` links `dragonfruit-files-core` as a
  library (never a process); the link is the "one browsing implementation"
  rule made real for the chooser.
- The request/result logic and the listing are pure and unit-tested; the
  integration test (`portal/tests/filechooser.rs`) drives a real D-Bus client
  acting as the presenter, so the blocking round trip is proven.
- T-13.2b adds the visual picker and its selection over the same seam; no
  portal-side change is needed for it. A real `xdg-desktop-portal` routing
  check and the Flatpak walkthrough remain T-13.7.