# 0030 — Desktop settings travel as typed schema keys over `org.dragonfruit.Settings1`

## Status

accepted

## Context

Design/08-settings.md makes `settingsd` the single owner of desktop settings,
with named keys and change signals instead of polling, and `legacy/10-dock.md`
already wrote the shell's interim file in the eventual key shape. T-08.1a had
to fix the concrete transport before T-08.1b (persistence) and T-08.2 (shell
and compositor migration) can build on it, but the design named no value
encoding, no object/interface layout, and no versioning mechanism.

The constraints: the compositor is Rust, the shell is C++/QML, and the
Settings app (T-09) is QML. All three must read the same keys without a
second model, values are of several types (bool, double, int, string, string
list), and the schema is additive-only within a release. The repo's existing
seam is a session-bus service with a small typed surface (ADR 0029).

## Decision

- **`services/settingsd` serves one interface at one path.** The well-known
  name and interface are both `org.dragonfruit.Settings1`; the object path is
  `/org/dragonfruit/Settings1`. It depends on `zbus` and `df-ipc` only.
- **Typed variant values, not JSON strings.** A value is one of `b`, `d`, `x`,
  `s`, `as`, carried as a D-Bus variant. Qt and Rust both decode variants
  natively, and the type is enforced on the wire and in the schema instead of
  being a string convention every consumer re-parses.
- **The surface is deliberately small and additive.** `Get(key) -> v`,
  `Set(key, value)`, `GetAll() -> a{sv}` (one-call resync), `ListKeys() -> as`,
  a `SchemaVersion` (`u`) property, and one `Changed(key, value)` signal.
  Rejections are typed errors under `org.dragonfruit.Settings1.Error`
  (`UnknownKey`, `TypeMismatch`, `OutOfRange`, `NotAllowed`). `Set` emits
  `Changed` **only when the value actually changes**, so a signal always means
  new state.
- **The schema is code, and frozen.** `schema::KEYS` declares every key's
  type, default, range/enumeration, and owner/consumer pair;
  `SCHEMA_VERSION` is the persisted revision; a test freezes the v1 key
  manifest so a rename or removal fails the build. Keys may be added, never
  renamed or removed.

Rejected: JSON-string values (loses typing; every consumer validates again);
per-key D-Bus properties (introspection explodes and cannot stay additive);
gsettings/dconf (an external stack the design explicitly replaces); a
file-watcher bridge (exactly the polling the design forbids). A
compositor-embedded store was rejected for the same reason as ADR 0029 — the
shell and Settings app must stay crashable independently.

## Consequences

- T-08.1b persists this exact key shape (`{"schema":N,"keys":{…}}`), adopts
  the shell's existing file without migration, and runs startup migrations
  keyed by `since`.
- T-08.2a deletes `DockSettings`/`DockPins` and their `QFileSystemWatcher` in
  the same change, so the interim JSON file never has two writers; T-08.2b
  binds `Theme.dark`/`Theme.reducedMotion`; T-08.2c applies motion/input
  policy in the compositor.
- Fixtures/tests for shell and compositor consumers may use the same schema
  names; `make e2e` runs `cargo test -p dragonfruit-settingsd`, whose
  integration test stands up a private session bus, so the D-Bus surface
  cannot regress silently.