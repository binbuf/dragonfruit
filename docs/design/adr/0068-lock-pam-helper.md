# 0068 — Lock-screen PAM authentication through a small helper

## Status

accepted

## Context

T-12.3a made the compositor enforce `ext-session-lock-v1` and made the shell
the first-party lock UI, but left the password field inert: unlock was
protocol-only (`ShellProtocol::unlockSession`). T-12.3b has to authenticate the
signed-in account without ever keeping a credential store, and input capture
(typing on the lock surface) is explicitly T-12.3c.

The host's PAM stack is the only sanctioned verifier
([12-session-lock-idle.md](../tracks/12-session-lock-idle.md): "PAM via a small
helper, never a custom credential store"). The build hosts ship only the
libpam runtime (`libpam.so.0`), not the `pam-devel` headers, and writing an
`/etc/pam.d` service belongs to packaging, not to a task that must run
headless.

## Decision

- **A separate helper process owns PAM.** A new crate `services/lock-auth`
  (package `dragonfruit-lock-auth`) ships the library seam
  `PamAuthenticator` and the binary `dragonfruit-pam-helper`. The shell spawns
  the helper per attempt, writes the password to its standard input, and reads
  back **only the exit status** (`0` authenticated, `1` rejected, `2`
  unavailable). Neither process persists, logs, or echoes the password.
- **libpam is loaded with `dlopen` at runtime** (`libpam.so.0`, then
  `libpam.so`). The crate declares the four symbols it uses
  (`pam_start_confdir`, `pam_authenticate`, `pam_acct_mgmt`, `pam_end`) and the
  stable conversation ABI locally, so building never requires `pam-devel` and
  authentication always uses the distribution's own module stack. Only the
  auth and account phases run; the session phase belongs to login.
- **The PAM service is `dragonfruit`, falling back to `login`.** A host may
  install `/etc/pam.d/dragonfruit`; when it is absent the authenticator retries
  the universally present `login` service, so a stock host authenticates.
  `--service` / `DF_PAM_SERVICE` override it.
- **`pam_start_confdir` is the test seam.** `--confdir` points PAM at a
  throwaway directory containing `permit`/`deny` services, so the headless
  suite exercises the real libpam and the real helper binary without root or
  `/etc/pam.d`.
- **The shell keeps only the outcome.** `LockAuthenticator`
  (`shell/src/lockauth.cpp`, in the Wayland-free dock-core) resolves the helper
  (`DF_PAM_HELPER`, then `PATH`, then siblings of the shell), runs it
  asynchronously, clears the password from its own memory after the write, and
  maps the exit status to `succeeded()` / `failed(message)`.
  `ShellController::onLockAuthSucceeded` calls
  `ext_session_lock_v1.unlock_and_destroy`; failures and errors keep the
  session locked. T-12.3c will call `submitLockPassword` once it routes input.
- **The dev tool hands the helper path over the environment.** `dragonfruit
  dev` sets `DF_PAM_HELPER` to the cargo-built sibling of the dev tool.

## Consequences

- Input capture can land independently: T-12.3c only has to route the typed
  password into `ShellController::submitLockPassword`; the authentication path
  and the unlock call are already in place.
- Packaging must install `dragonfruit-pam-helper` on `PATH` (and may install
  `/etc/pam.d/dragonfruit`); otherwise the `login` fallback is used. A missing
  helper or a missing libpam is an `Error` (exit `2`) and leaves the session
  locked — never a false unlock.
- `cargo test -p dragonfruit-lock-auth` and the shell's `tst_lockauth` cover
  success/denied/error and the one-attempt-at-a-time boundary; `make e2e` runs
  the former.
- The password exists only in the shell's write buffer and the helper's
  conversation state for the duration of one attempt. There is no credential
  file, D-Bus object, or settings key carrying it.