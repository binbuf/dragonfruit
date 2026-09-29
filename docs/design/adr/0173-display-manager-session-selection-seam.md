# 0173 — Display-manager session selection is a per-DM adapter, and an ambiguous host degrades to the greeter

## Status

accepted

## Context

The real-session round trip (T-12.6b) must point the *next* display-manager
login at the Dragonfruit session and then put the previous default back. The
three common Wayland-capable DMs record that selection differently: GDM in the
AccountsService user file
(`/var/lib/AccountsService/users/<user>`, `[User] XSession=`), SDDM in its
state file (`/var/lib/sddm/state.conf`, `[LastUser] Session=`), and LightDM in
`~/.dmrc` (`[Desktop] Session=`). Autologin is separate, needs root, and is
written to a different file per DM (`/etc/gdm/custom.conf`,
`/etc/sddm.conf.d/autologin.conf`, `/etc/lightdm/lightdm.conf`). The harness
must not guess on a host it does not recognize, and it must never leave
autologin armed outside a round trip
([ADR 0053](0053-real-session-dev-harness.md)).

## Decision

`tools/dragonfruit-dev/src/session_selector.rs` is the seam.

- **One adapter per DM.** `DmSession` (read/select/restore) and `DmAutologin`
  (snapshot/arm/disarm/restore) are traits; `GdmSession`/`SddmSession`/
  `LightdmSession` and the three autologin adapters are the concrete
  implementations. Every edit is a minimal keyfile change that preserves
  unrelated lines, so the file round-trips byte-for-byte.
- **Detection never guesses.** `SessionSelector::dm` returns a DM only when one
  is running (matched from `/proc/*/comm`) or exactly one is installed
  (`/etc/gdm…`, `/etc/sddm.conf*`, `/etc/lightdm`). It returns `None` for an
  unknown *or ambiguous* host; `select_dragonfruit` then yields
  `Selected::Unsupported` and **no write is attempted**, so the caller falls
  back to "pick it in the greeter".
- **Autologin is opt-in and reversible.** `snapshot` reports off when no
  configuration exists; `arm` writes the user/session; `disarm` forces it off;
  `restore` puts a prior snapshot back. This module never arms autologin by
  itself.
- **Every path derives from an explicit root and home**, so fixture tests drive
  real reads and exact-byte writes with no DM installed; on a host the root is
  `/`.

## Consequences

- T-12.6b calls the seam, persists the returned previous selection, and owns
  the crash-recovery state file and the "session-ready" beat.
- Real DM validation (session entry installed, correct session id format for a
  given SDDM/GDM version, root-owned autologin writes) stays on the
  T-159…T-161 hardware rail.
- An unsupported/ambiguous host is a normal state, not an error: the round trip
  still works, the human just picks Dragonfruit in the greeter.