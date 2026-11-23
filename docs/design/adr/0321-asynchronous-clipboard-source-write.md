# 0321 — Clipboard serving is an asynchronous, deadline-bounded write

## Status

Accepted (2026-11-22). Implements the recovery contract in
[design 27](../27-stability-and-daily-driver.md) §3.4 and closes S-10.

## Context

The shell forwards a stored clipboard-history entry by creating a
`zwlr_data_control_source_v1` and granting the client a pipe fd in the
`send` event. The client is not required to read that pipe. The previous
handler looped on a blocking `write()` from the Wayland dispatch callback, so
a client that requested the selection and stopped reading filled the pipe
buffer and froze the shell, including its compositor connection and input
handling. A client that closed its read end instead raised `SIGPIPE`, whose
default action terminates the process.

The payload is already bounded by `ClipboardHistory::kMaxEntryBytes` (1 MiB),
so the fix does not need a second buffer or a worker thread; it needs to stop
writing synchronously.

## Decision

1. **One writer owns the fd.** `ClipboardSourceWriter`
   (`shell/src/clipboardwriter.h/.cpp`, part of the Wayland-free
   `dragonfruit-shell-dockcore` library) takes ownership of the granted fd,
   sets `O_NONBLOCK`, and holds the payload's implicit share until the
   transfer ends.
2. **Write from the event loop, one call at a time.** Construction defers the
   first write to the event loop. Each `QSocketNotifier` writable callback
   performs at most one `write()` and resumes at the last offset. `EAGAIN`
   leaves the notifier armed; `EINTR` returns to the loop. The Wayland
   dispatch callback never loops on `write()`.
3. **Bound the transfer.** A 2 s single-shot deadline closes the fd when a
   client neither drains nor closes it. `EPIPE` and other hard write errors
   finish immediately. The deadline is a safety net, not a user-visible delay.
4. **Cancel by selection, not by guess.** A repeat `send` for the same MIME
   supersedes the in-flight writer; a new selection, source cancellation, and
   protocol teardown cancel every writer. Cancellation closes the fd, so the
   client sees EOF.
5. **Never die on a closed reader.** `SIGPIPE` is blocked for the duration of
   each write and the synchronously generated signal is consumed; the shell
   process also ignores `SIGPIPE` at startup as defence in depth.

Rejected alternatives:

- **Keep the blocking loop but cap the payload.** A 1 MiB payload still fills
  the pipe buffer; the shell still blocks.
- **Write on a worker thread per send.** It adds thread lifetime and
  cancellation to the Wayland path for a bounded, non-CPU-bound write.
- **Buffer the payload into a temporary file or a second allocation.** The
  existing `QByteArray` is already bounded and implicitly shared; a second
  copy buys nothing.
- **Treat the timeout as a user-visible error.** The entry stays in history;
  the next paste re-serves it.

## Consequences

- Any future clipboard source (primary selection, X11 bridge) that grants a
  pipe should use the same writer rather than writing from a dispatch
  callback.
- A client that stalls receives a partial payload and EOF after the deadline;
  it can re-request the selection.
- The writer's `Result` (`Completed`, `TimedOut`, `Cancelled`, `Error`) is the
  observable contract for tests and for any future telemetry.
