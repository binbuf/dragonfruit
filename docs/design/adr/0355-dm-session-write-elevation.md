# 0355 — Display-manager writes elevate the one root-owned file and verify the selection

## Status

accepted

## Context

The round trip (T-12.6b) must point the next login at Dragonfruit and then put
the previous default back. On a real host the DM's session state is root-owned:
GDM's AccountsService user file (`/var/lib/AccountsService/users/<user>`), SDDM's
`/var/lib/sddm/state.conf`, and the autologin drop-ins under `/etc`. The harness
runs as the desktop user, so the direct write fails with `EACCES`.

T-12.6a treated every write error as `Selected::Unsupported`, which is the
degradation for an *unknown* display manager. For a detected DM with a refused
write that is wrong: the harness still logged the user out, and the next login
came back to the host desktop with Dragonfruit unselected. The failure was
silent and the round trip was lost.

## Decision

- **The adapters take a file API, not `std::fs`.** `DmFiles` is read/write/remove
  over a path; fixtures use `LocalFiles` (direct) and the host uses
  `ElevatedFiles`, which retries only `EACCES`/`EPERM` through a `Privilege`.
- **Elevation is scoped to the DM file.** `HostPrivilege` prefers `pkexec` (a
  polkit prompt works from the shell's no-terminal return action) and falls
  back to `sudo`. It writes through `tee` so an existing 0600 AccountsService
  file keeps its mode; it never starts the session as root.
- **GDM prefers AccountsService.** When `gdbus` is installed the adapter calls
  `org.freedesktop.Accounts.FindUserByName` and
  `org.freedesktop.Accounts.User.SetXSession` on the system bus, with a 5 s
  bound. A missing service or a refusal falls back to the elevated file; the
  file remains authoritative for the exact-byte snapshot.
- **A detected DM must verify.** `select_dragonfruit` reads the selection back
  after the write. A refusal, a missing helper, or a value that does not match
  Dragonfruit is `Selected::Failed`, not `Unsupported`.
- **The arm is transactional with the logout.** `RoundTrip::arm` returns an
  error on `Selected::Failed` (and rolls the captured files back), so the
  session manager is never asked to log out. A state-file write failure rolls
  the DM files back too. `--return`/`--recover` restore the captured bytes
  through the same elevated file API.

`Selected::Unsupported` remains the correct outcome for an unknown or ambiguous
host: nothing is written and the human picks Dragonfruit in the greeter, with a
loud warning.

## Consequences

- A refused elevation fails `make native`'s arm before logout, with the host's
  message; the previous desktop stays selected.
- A real GDM host can preselect without a root prompt when AccountsService and
  its polkit policy allow `SetXSession`; otherwise the same pkexec prompt as
  SDDM/LightDM appears.
- `--return`/`--recover` need the same helper. A text-console recovery still has
  `scripts/native-session.sh recover`, which runs its snapshot restore under
  `sudo` and does not depend on the built dev tool.
- The elevated write is a file write, so it cannot be exercised against `/` in
  the unit suite; the fake-DM fixtures and the `FakePrivilege` call log pin the
  retry, the read-back, the denial, and the byte-identical restore. The real
  polkit/sudo prompt remains on the T-159…T-161 hardware rail.
