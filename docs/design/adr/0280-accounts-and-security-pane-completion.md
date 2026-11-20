# 0280 — Accounts and security panes: grouped privacy, honest biometric stages, one account path

- **Status:** Accepted
- **Date:** 2026-11-16
- **Task:** T279
- **Supersedes:** none; extends ADR 0138, 0139, 0142, 0143, 0233, 0237, 0262
- **Related:** ADR 0031, 0032, 0185, 0266, 0267, 0279

## Context

T228 landed the four account/security panes as host-stack projections, but the
2026-11-14 review found four gaps. Privacy & Security was a flat un-carded list
with one glyph, no keyboard path, and no master/analytics/accessory rows.
Biometrics reported an invented "Touch the fingerprint sensor" line instead of
fprintd's stage, discarded a polkit denial, and closed the password dialog
before the host answered. Users & Groups created an account without a password,
had no change/reset action, deleted a group without confirmation, duplicated
automatic login in two controls, and had no real group provider (T256 had not
landed). Online Accounts disappeared when GOA was absent and had no details or
sign-in surface.

## Decision

1. **Privacy is grouped and keyboard-complete.** The list is Location Services
   (master + per-app), App access (the fourteen remaining macOS categories,
   each with its own Tabler glyph), Analytics & Improvements, and Security.
   Every category is a `SettingsRow` with a focus ring and AT-SPI name, so it
   opens from the keyboard. The one durable master is
   `privacy.locationServices`: the portal store has no global switch, so the
   pane enforces it by disabling per-app location writes and saying so. It is
   not described as a portal-wide block.

2. **Analytics is opt-in, default off, and stored-only.** `privacy.analytics`
   is a schema key (revision 36) with no collector in this build; the map and
   the row say stored-only rather than presenting a live telemetry switch.
   USB accessories project USBGuard read-only and delegate rule changes to the
   `usbguard` CLI; disk encryption gains a device sheet and stays read-only.

3. **Biometrics reports the owner, not a guess.** The enrollment wizard shows
   fprintd's own `EnrollStatus` stage id and a human line derived from it,
   never a percentage. A denied/absent/failed write is shown at the row and in
   the wizard; an unsupported reader is the honest no-reader state and leaves
   the password half live. The password dialog stays open until the host's
   report arrives. Fast user switching, purchases, and watch unlock stay
   omitted: no display manager or secure element exposes them.

4. **Users & Groups has one account path.** Add User requires and confirms a
   password; the create action waits for the host and never sends a mismatch.
   The per-user sheet adds one Change/Reset Password action and drops the
   duplicate automatic-login toggle. User and group deletion are confirmed.
   The real group provider (`FileGroupProvider`) reads through NSS and writes
   `groupadd`/`groupdel`/`gpasswd` under `pkexec`; a host without it disables
   only the group rows.

5. **Online Accounts is always visible.** With GOA absent the pane keeps its
   header, states "Online accounts are not installed", and offers a delegate
   action naming the distribution package. With GOA present, Details and Sign
   in again project the service (`EnsureCredentials`); Settings never
   implements OAuth or stores a credential.

## Consequences

- Schema revision 36 adds `privacy.locationServices` (bool, true) and
  `privacy.analytics` (bool, false); the C++ client table mirrors it.
- `SettingsRow` gains an optional `leadingIcon`, shared by every pane.
- The Tabler asset set grows fourteen category glyphs; the generated registry
  and `make check-tabler-icons` stay the source of truth.
- The group write path depends on `pkexec`; a host without it reports a
  visible denial rather than editing `/etc/group` directly.
- The live host check (real fprintd stage signals, a real USBGuard daemon, a
  real GOA sign-in) remains a hardware/session row for T283/T284.
