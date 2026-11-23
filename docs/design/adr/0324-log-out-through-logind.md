# 0324 — Log Out is one logind session action behind the power confirmation

## Status

Accepted (2026-11-22). Implements [design 27](../27-stability-and-daily-driver.md)
§3.4 and closes S-08.

## Context

The system menu's Log Out row and `Super+Shift+Q` were a live no-op: the menu
action logged a message and the compositor had no binding. The session entry
already owns logout teardown — it waits for the compositor anchor to leave the
active state, stops `dragonfruit-session.target`, clears failed state, and
removes the runtime hand-off files. The missing piece was asking logind to
terminate the session, not signalling the shell or compositor directly.

`org.freedesktop.login1.Manager.TerminateSession` takes only the session id;
the `interactive` boolean belongs to `Reboot`, `PowerOff`, and `Suspend`.
Passing a second argument would make the call invalid.

## Decision

1. **One log-out path.** `SessionActions::requestLogout()` reads
   `$XDG_SESSION_ID` and calls `Manager.TerminateSession(s)` on the system bus.
   The compositor, the session entry, and systemd own the rest of the teardown.
2. **The same destructive-action confirmation as Restart and Shut Down.**
   Both the menu row and the shortcut route through `requestSessionAction`,
   which opens the existing 60-second power dialog. The confirmed action is
   the only caller of the logind request; Sleep and the lock screen's power row
   stay immediate.
3. **The compositor routes the chord.** `InputAction::LogOut` (`log-out`) is
   bound to Cmd+Shift+Q and broadcast over the private protocol. The shell
   maps that action to the same confirmation as the menu row.
4. **A logind-less host falls back to the user target.** When
   `org.freedesktop.login1` is absent, the shell starts
   `systemctl --user stop dragonfruit-session.target`. That stops the anchor
   through systemd, so the session entry still performs the normal teardown.
5. **The row is disabled only when neither path exists.** Logind availability
   and the `systemctl` fallback are checked together; with neither, the Log Out
   row is disabled and carries a `disabledReason` for assistive technology.
   Disabling it whenever logind is absent would make the documented fallback
   unreachable.
6. **Tests never touch the host session.** `tst_sessionactions` runs a fake
   `org.freedesktop.login1` on a private `dbus-run-session` bus and asserts the
   method and session id. `DRAGONFRUIT_NO_LOGOUT` is the fixture seam: it
   records the intended call and suppresses both D-Bus and `systemctl`, so a
   capture or test cannot end the developer's session. The ctest environment
   also points `DBUS_SYSTEM_BUS_ADDRESS` at a dead path.

Rejected alternatives:

- **Signal the shell or compositor.** It bypasses systemd and can leave the
  target and its process groups behind.
- **Use `KillSession`/`TerminateUser`.** They are broader than this session and
  can affect other logins.
- **Pass `interactive=true` to `TerminateSession`.** The method has no such
  parameter; the shell's confirmation is the interactive step.
- **Disable Log Out whenever logind is absent.** It contradicts the required
  systemctl fallback.
- **Run the real logind in tests.** A mistake would end the developer's
  session; the private fake and the fixture guard make that impossible.

## Consequences

- A clean Log Out exits the compositor, stops the target, and returns the
  greeter through the existing T-12.2 entry. No logout animation or fast user
  switching is added.
- A host with logind but no `systemctl` (or vice versa) still has one working
  path; the menu reason only appears when both are missing.
- The dialog copy, glyph, and countdown are shared with Restart/Shut Down;
  `tst_powerdialog` covers the Log Out strings.
- Any future session action must use `SessionActions` rather than a second
  ad-hoc D-Bus call.
