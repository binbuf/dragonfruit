# 0320 — The Xwayland `DISPLAY` is read at launch time

## Status

Accepted (2026-11-22). Implements the launch contract in
[design 27](../27-stability-and-daily-driver.md) §3.7 and constrains later
launcher work.

## Context

The session entry exports the startup environment before the compositor has
started Xwayland, so `--print-env` cannot carry a live `DISPLAY`. The
compositor writes `$XDG_RUNTIME_DIR/<socket>.x11-display` when Xwayland is
ready and removes it at teardown. The shell, Files, and Settings each start
external programs, and an X11-only app fails without that value.

Reading the file once at process startup is also wrong: a long-lived shell or
Settings window can outlive an Xwayland restart, and a late Xwayland start
would never reach a child. Conversely, a parent process may carry a stale
`DISPLAY` from the login environment; leaving it in place makes a Wayland-only
launch fall back to the wrong X server.

## Decision

1. **Read the hand-off at each launch.** A small shared header,
   `libs/launch-env/launch_env.h`, exposes
   `Dragonfruit::x11DisplayFromHandoff` and
   `Dragonfruit::applyX11DisplayHandoff`. The caller supplies the compositor
   socket it actually connected to; an empty argument falls back to the
   environment's `WAYLAND_DISPLAY`.
2. **Trim the file, and remove an absent value.** The file's contents are
   trimmed, so the compositor's trailing newline is not part of `DISPLAY`. An
   absent, unreadable, or empty file removes any inherited `DISPLAY` rather
   than leaving it for a Wayland-only child.
3. **Apply it at every external launch.** The shell's
   `appLaunchEnvironment`, the Files desktop child environment, and the
   Settings accessibility delegates call the helper immediately before
   starting the child. The read is cheap and is not cached across launches.
4. **Leave the Wayland and Qt platform rules alone.** `WAYLAND_DISPLAY`
   continues to come from the connected socket, and the shell's offscreen
   `QT_QPA_PLATFORM` is still scrubbed to `wayland`; this ADR only adds the X11
   variable.

Rejected alternatives:

- **Import `DISPLAY` through `--print-env`.** Xwayland has not started when
  the session entry runs, and a compositor restart changes the value.
- **Cache the hand-off at shell startup.** It breaks a late Xwayland start and
  the Xwayland-restart path.
- **Keep an inherited `DISPLAY` when no file exists.** That is the stale-X11
  leak this task fixes.

## Consequences

- Any new launcher that starts a GUI child must call the helper; the existing
  three paths are the reference.
- A Wayland-only session launches children without `DISPLAY`, even when the
  parent environment had one.
- The shell-side read is sufficient; the session entry does not need to learn
  `--x11-display` for normal launches.
