# 0032 — The shell consumes settingsd through one typed client

## Status

accepted

## Context

ADR 0030/0031 gave `settingsd` the `org.dragonfruit.Settings1` schema, D-Bus
surface, and persistence, and T-08.2a must migrate the shell's Dock off its
interim file owners (`DockSettings`/`DockPins` and a `QFileSystemWatcher`).
The same consumer seam will serve T-08.2b (design-system `Theme`) and later
the Settings app (T-09), so it should not be Dock-specific. Two realities
shape the choice: the dev/demo tool deliberately does not start settingsd, so
the shell must keep working with no daemon on the bus; and a write must not
require a blocking round-trip before the Dock reacts.

## Decision

- **One client abstraction in `shell/src/settingsclient.h`.** `SettingsClient`
  is a typed key store (`value`/`boolean`/`real`/`integer`/`string`/
  `stringList`) over a `QVariantMap`, with `changed(key,value)`,
  `availableChanged`, and `refreshed` signals. `DbusSettingsClient` is the
  live implementation; `MockSettingsClient` is the in-process fixture for
  headless tests. Consumers never touch `QDBus*` types directly.
- **Schema defaults are mirrored in C++** (`settingsSchemaDefaults()`), typed
  exactly as `schema.rs` declares (`b`/`d`/`x`/`s`/`as`). This makes the shell
  complete without a daemon. A `GetAll` snapshot overrides them once the
  daemon answers, and a `Changed` signal updates a single key.
- **No polling.** Only `Changed` (subscription) and `GetAll` (initial resync /
  name-appearance resync) are used; there is no timer and no read loop, so the
  shell idle-trace guarantee holds.
- **Writes are optimistic and de-duplicated.** `set()` updates the local store
  and emits `changed` on the same event-loop turn, then mirrors the write with
  the `Set` method when the daemon is present. The daemon's `Changed` echo is
  ignored because the value already matches; a write made while the daemon is
  down survives in memory for the session only.
- **The Dock's typed view lives with the Dock.** `DockConfig` +
  `dockConfigFromValues()` in `dockmodel` keep the pure layout code free of
  D-Bus and testable; `dockIconSize()` and `resolveDefaultDockPins()` (moved
  out of the deleted `DockPins`) are pure helpers.

Rejected: per-consumer D-Bus calls (duplicates marshalling and resync logic);
reading the JSON file directly as a fallback (creates a second owner and
re-introduces the watcher); blocking synchronous `Get` on first paint (stalls
the shell for a daemon that may be absent); a `dock.*`-specific client (T-08.2b
needs the same seam for `appearance.*`/`accessibility.reduceMotion`).

## Consequences

- T-08.2b binds `Theme.dark`/`Theme.reducedMotion` by connecting to
  `SettingsClient::changed` and reading `appearance.colorScheme`,
  `appearance.accent`, and `accessibility.reduceMotion`; it must not start a
  second bus connection or a watcher.
- The schema default list in `settingsSchemaDefaults()` must be extended
  whenever a key is added in `schema.rs`; the frozen v1 manifest is enforced by
  `settingsd`, and the mirrored list is enforced by the C++ tests
  (`tst_dockcore`, `tst_settingsclient`).
- `dock.pinned` seeds the installed defaults once per session when empty. Since
  the daemon exposes no "file existed" bit, an intentionally emptied pin set is
  re-seeded on the next launch; T-08.3 may add a seeding marker if that matters.
- The client holds last-known values across a daemon restart (`availableChanged
  false`), so nothing is visually lost; T-08.3 owns the full kill/restart
  resync story and the "change made while down" semantics.