# 0031 — settingsd persists the key shape as atomic JSON and migrates at startup

## Status

accepted

## Context

ADR 0030 fixed the `org.dragonfruit.Settings1` key schema and the wire
encoding. T-08.1b must make `settingsd` the on-disk owner of that same state
under `$XDG_CONFIG_HOME/dragonfruit/`. The shell's interim `DockSettings`/
`DockPins` already write `settings.json` in the eventual
`{"schema":N,"keys":{…}}` shape, so the file must be adopted without
conversion, and the design requires schema-documented storage, atomic writes,
and startup migrations (Design/08-settings.md, `legacy/15`). Two constraints
shape the choice: the shell will keep writing that file until T-08.2a removes
it, and a settings write must never corrupt the previous file if the process
dies.

## Decision

- **One JSON file, `$XDG_CONFIG_HOME/dragonfruit/settings.json`** (fallback
  `$HOME/.config/...`), with the schema revision in `schema` and named keys
  under `keys`. JSON value shapes follow the key's declared type: bool, JSON
  number (double or integer), string, or array of strings.
- **Atomic replace.** A save is staged in a hidden sibling
  (`.settings.json.<pid>.<n>.tmp`), `sync_all`ed, then `rename(2)`d over the
  target; the directory is best-effort fsynced. Readers ignore anything but
  the target name, so a crash between staging and rename leaves the prior file
  intact and a stale temp file inert.
- **Startup migrations keyed by `schema`.** A file with no `schema` field is
  revision 0; a key's `since` revision lets the migration fill a key an older
  file predates with its default. Migration is additive-only, matching ADR
  0030; a migrated file is written back at the current revision once at
  startup.
- **Unknown entries are preserved.** Root fields other than `schema`/`keys`
  and key names outside the schema are carried through a save verbatim, so a
  settingsd write cannot clobber what the interim shell writer left behind.
  A schema key always wins over an unknown entry of the same name.
- **Load is lossy-tolerant, save is strict.** A stored value that fails schema
  validation falls back to its default instead of rejecting the file; a
  malformed or unreadable file is reported and the daemon runs from defaults.

Rejected: one file per key or per group (the shell already shares one file and
`DockPins` re-reads it); a journal/WAL (atomic rename plus the schema default
fallback is enough for a preference file); `gsettings`/dconf (ADR 0030
rejected the external stack); adding a `tempfile`/`serde_json`-alternative
dependency (std plus the repo's pinned `serde_json` suffices).

## Consequences

- T-08.2a deletes the shell's file owners in the same change it binds the
  daemon; until then, preserving unknown entries means the two writers do not
  clobber each other's keys, though only settingsd is the durable owner.
- T-08.3 (restart/resync) can rely on `Changed` being emitted only after the
  file holds the new value; a consumer that reappears after a restart reads
  the same file or calls `GetAll`.
- Adding a key later is safe: bump `SCHEMA_VERSION`, set the key's `since`,
  and old files are filled at startup. Renaming or removing a key still fails
  the frozen v1 manifest test.
- The persisted format is the shell's already; do not change the file name or
  the `schema`/`keys` wrapper without a migration step and a coordinated
  shell change.