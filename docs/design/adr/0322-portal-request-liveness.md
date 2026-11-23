# 0322 — Portal requests are bounded and cancel when the presenter leaves

## Status

Accepted (2026-11-22). Implements the recovery contract in
[design 27](../27-stability-and-daily-driver.md) §3.4 and closes S-11.

## Context

The interactive portal methods (`FileChooser.OpenFile`/`SaveFile`/`SaveFiles`,
`Screenshot.Screenshot`, `ScreenCast.SelectSources`) do not return until a
presenter answers through the diagnostic `org.dragonfruit.Portal1` surface.
The backend awaited a one-shot completion with no deadline. If the shell
crashed, exited, or restarted, the completion never fired and the calling
application blocked forever. The shell bridges also cleared their live handle
on a second request without answering the old one, so the first caller waited
for a presenter that would never return to it.

The presenter is the shell process, but it had no well-known bus name. There
was no observable "the presenter is gone" event for the backend to watch.

## Decision

1. **A 30 s deadline on every interactive completion.** `portal::liveness`
   races the completion against `async-io`'s timer (the reactor zbus already
   runs). On expiry the interface removes the pending registry entry and
   answers the standard `cancelled` code. A timeout is a normal response, not
   an error dialog. `serve_with_timeout`/`initialize_with_timeout` exist so
   tests can prove the path without waiting 30 s.
2. **The shell owns `org.dragonfruit.PortalPresenter1`.** `ShellController`
   registers the name before any bridge subscribes. The backend watches
   `NameOwnerChanged` for that name (the same mechanism as the existing
   frontend watch). Any owner loss or replacement cancels every pending
   FileChooser, Screenshot, and ScreenCast picker request. A first appearance
   is not a loss.
3. **A live handle is never silently dropped.** `ScreenshotBridge` and
   `ScreenCastBridge` answer `Cancel*` for the old handle before `reset()` when
   a second request or the desktop shortcut replaces it. The chooser keeps its
   queue behavior and answers `CancelFileChooser` on an explicit cancel.
4. **Startup queries `Pending*` and cancels.** Each bridge asks its backend for
   the requests a previous shell left behind. A stale request is cancelled,
   not re-presented: a `Pending*` row carries only the handle and a kind/mode,
   so re-presenting a chooser or picker without its options would answer the
   caller with the wrong result (SaveFiles' file names in particular).
5. **The bridge calls use message calls, not `QDBusInterface`.** A
   `QDBusInterface` constructor performs a synchronous `Introspect`; when the
   presenter is served by the same process (the tests, and the nested
   presenter rail), that introspection can deadlock on queued dispatch. The
   presenter calls and `Pending*`/`ScreenCastStreamMode` probes are
   `QDBusMessage` calls.

Rejected alternatives:

- **Re-present every `Pending*` row.** The row is lossy; a chooser shown
  without `files` or `current_folder` answers wrongly. Cancelling is the
  honest answer, and the portal's presenter watch normally cancels first.
- **Watch the frontend (`org.freedesktop.portal.Desktop`) instead.** The
  frontend is not the presenter; it may be absent while the shell is alive.
- **Make the timeout configurable in production.** A single 30 s bound is the
  contract; a per-session knob would let a hung presenter hide behind a longer
  value.
- **Treat presenter loss as an error response.** `cancelled` is what the
  standard frontend and the application already handle.

## Consequences

- A shell crash or restart resolves every waiting portal caller within the
  bus round trip; the 30 s deadline covers a presenter that is present but
  silent.
- The presenter name is part of the shell's session-bus surface. A future
  shell split (a separate presenter process) must own the same name.
- The `Pending*` methods are now part of the recovery path, not only a
  diagnostic; their `(handle, detail)` shape is frozen.
- The backend cancels on any owner replacement, including a fast restart that
  transfers the name without an absent interval.
