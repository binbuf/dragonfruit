# ADR 0350 — The Files desktop exits on compositor loss and gates its one layer

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T339
- **Supersedes:** none

## Context

`DesktopProtocol::dispatch()` returned `false` and emitted `fatal` when
`wl_display_dispatch` failed, but `FilesDesktop` only printed the message. The
`QSocketNotifier` stayed enabled on a dispatch fd that is readable forever after
the compositor disconnects, so the desktop process could spin at 100% CPU while
the session was already ending. Separately, `FilesDesktop::start()` called
`createDesktopSurface()` twice, creating a second `background` layer for the
same process.

`dragonfruit-files-desktop.service` already ships `Restart=always`, and ADR
[0169](0169-desktop-process-client.md) already says the client creates exactly
one full-output `background` layer. Neither property was enforced in the code.

## Decision

1. `DesktopLifecycle::exitOnCompositorLoss()` disables the dispatch
   `QSocketNotifier` first, then queues `QCoreApplication::exit(1)` through a
   zero-delay timer. Disabling before the queued exit keeps a second activation
   from re-entering the fatal handler while the loop unwinds; the non-zero code
   is what makes `Restart=always` apply.
2. `DesktopLifecycle::createSurfaceOnce()` is the one-surface gate.
   `FilesDesktop` calls it once, after the QML item is ready, and the gate
   invokes `createDesktopSurface()` exactly once. A later request is a no-op
   instead of stacking a layer.
3. The exit function is injectable (`DesktopLifecycle::ExitFunction`). Production
   uses the default `QCoreApplication::exit`; the regression test observes the
   request without tearing down its own event loop. The child-process probe in
   `tst_files_desktop` runs the default path and asserts exit code 1.

## Consequences

- A dead compositor can no longer leave a CPU-spinning desktop; systemd starts a
  fresh process, and the compositor reissues the launch token on disconnect.
- The desktop owns exactly one `background` layer even if a future start-up
  change adds a second call site.
- `tst_files_desktop` covers the EOF path (notifier disabled, one exit request,
  no spin), the real process exit code, and the one-surface gate, all without a
  compositor or libwayland.
