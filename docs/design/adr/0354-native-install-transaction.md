# ADR 0354 — The native install is one validated transaction

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T343
- **Supersedes:** none

## Context

`make native` staged the session entry, units, app metadata, portal data, and
QML tree under a temporary directory, then copied the whole tree into the
prefix with one `cp -a`. The binary conflict check ran *after* that copy, so a
packaged binary that the user did not want replaced was already clobbered by
the time the script refused. A write that failed halfway left a mixed tree, and
`native-recover` had no record of what the install had written: it restored the
display-manager files and the dev environment but left the `/usr` install in
place.

The session entry had the mirror problem. It removed the runtime hand-off
socket *before* checking whether `dragonfruit-session.target` was already
active. Starting an active target is a systemd no-op, so a stale login (a
lingering user manager, a crashed entry, or a dev restart) could lose a live
compositor's socket while the target kept running. A failed `start` also left
the partially started target behind.

## Decision

1. The complete staged tree is validated before the first write.
   `scripts/native-install.py` enumerates every staged path and the binary link
   farm, and refuses the install when a destination exists, differs, and is not
   owned by a previous install. `--force` is the only way past such a conflict;
   it keeps the replaced file as `<path>.pre-dfdev`.
2. Writes are transactional. Each replaced file gets a rollback copy; each file
   or symlink is written through a temporary sibling and `os.replace`. Any
   write failure, or a failed post-write check of the required entry files and
   executable links, restores the rollback copies, removes files this run
   created, and removes only the directories this run created. A refused
   validation touches nothing.
3. The install records a manifest under the user state directory
   (`~/.local/state/dragonfruit/native-install.json`) listing the paths it owns,
   any `.pre-dfdev` backup, and the directories it created. `recover` removes
   exactly that list and restores the backups. A packaged install has no
   manifest and is never touched.
4. The runtime environment and unit drop-ins are written only after the install
   commits, so a failed install cannot point a login at a tree it did not
   write.
5. The session entry stops and resets an already-active target before the
   hand-off cleanup, and stops the target again when `start` fails. Both paths
   are idempotent.
6. `XCURSOR_PATH` is composed through one colon-list de-duplicator and checked
   after composition, so two runs from a login that sourced `native.env` write
   the same value instead of appending the list again.
7. `scripts/check-native-session.sh` proves the behavior offline: a temporary
   prefix and HOME, fake build artifacts, a fake `cmake`, `systemctl`, and
   session binary. It never writes to `/usr` and never uses root.

## Consequences

- A conflicting packaged binary, a full disk, or a failed verification leaves
  the previous install runnable and the script says so; the user does not have
  to recover a half-installed session from a TTY.
- `recover` now also uninstalls the dev tree it installed. That is the intended
  meaning of "restore the host login"; a distro package that did not come from
  `make native` is untouched because it is not in the manifest.
- The transaction needs `python3` on the install path. Recovery already needed
  it, and the script dies with an explicit message when it is missing.
- `DF_NATIVE_BUILD_ROOT` and `DF_NATIVE_INSTALL_MANIFEST` are documented test
  seams; production runs use the repo build tree and the user state directory.
