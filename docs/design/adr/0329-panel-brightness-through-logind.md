# 0329 — Panel brightness through logind

## Status

Accepted (2026-11-23). Supersedes in part
[ADR 0060](0060-control-center-panel-and-brightness.md)’s “DRM apply slots into
the compositor handler” consequence, and
[ADR 0327](0327-shell-idle-effects.md)’s dim-overlay decision when a real
backlight exists.

## Context

`display.brightness` was stored, forwarded to the compositor as
`df_output.set_brightness`, and reported back, but never reached the panel. The
compositor owns the value for state and the OSD, yet it has no unprivileged
backlight write: `/sys/class/backlight/*/brightness` is normally root-owned.
The shell already owns logind for suspend, sleep inhibitors, and Log Out, and
logind exposes the unprivileged `Session.SetBrightness` path. A second DRM or
sysfs writer in the compositor would duplicate that ownership and still need a
privilege boundary.

## Decision

1. **One resolver, shared by the shell and Settings.**
   `libs/settings-client/backlight.*` enumerates `/sys/class/backlight`,
   requires a readable positive `max_brightness`, prefers `raw` over
   `platform` over `firmware`, and maps a 0..1 level onto `0..max` with
   rounding. The Settings app uses the same resolver only to show the absence
   note.
2. **The shell is the physical writer.** `shell/src/brightnessbackend.*` calls
   `org.freedesktop.login1.Session.SetBrightness("backlight", name, value)`
   asynchronously with a short timeout. The session object defaults to
   `/org/freedesktop/login1/session/auto`; the bus and path are injectable for
   tests. An async failure is logged and reported; it never blocks the GUI
   thread.
3. **The compositor keeps the value for state and the OSD.** The shell sends
   the same effective level to `df_output.set_brightness` (including the
   battery and idle-dim factors) and to the backlight backend. The protocol
   and the compositor’s stored value are unchanged.
4. **Absence is explicit.** If logind is absent, the backend writes the sysfs
   `brightness` file only when it is actually writable; otherwise it logs and
   reports a visible absence reason and does not claim success. With no
   `/sys/class/backlight` device the settingsd level is kept and the Displays
   pane shows a “no backlight” note.
5. **Idle dim uses the panel when it can.** With a real write path the
   backlight is the dim effect and the T317 translucent overlay is not mapped.
   The overlay remains only as the nested/headless fallback, so the screen is
   never dimmed twice.

## Consequences

- `tst_brightness` proves the device choice, the mapped value, the logind
  request, the writable-sysfs fallback, the absent no-op, and the
  non-blocking dispatch with a fake session and a temporary sysfs tree.
- `scripts/settings-consumers.json` marks `display.brightness` verified
  against `shell/tests/tst_brightness.cpp`.
- Keyboard brightness keys still need their T321 binding; once they write
  `display.brightness` they use this same path.
- Auto-brightness, keyboard backlight, and per-output brightness remain out of
  scope.
- The real panel write is hardware-open on a host without a backlight device;
  the hand-off records the exact `busctl` command.
