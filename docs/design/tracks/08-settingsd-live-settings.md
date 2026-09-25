# T-08 — settingsd: One Owner for Desktop Settings

> **Track, not a single slice.** This file is the design reference. It is executed as 6 one-session tasks: [T-08.1a](../../tasks/042-t-08.1a-settingsd-model-and-dbus-api.md) · [T-08.1b](../../tasks/043-t-08.1b-settingsd-persistence-and-migrations.md) · [T-08.2a](../../tasks/044-t-08.2a-shell-migration-to-settingsd.md) · [T-08.2b](../../tasks/045-t-08.2b-design-system-theme-binding.md) · [T-08.2c](../../tasks/046-t-08.2c-compositor-policy-migration.md) · [T-08.3](../../tasks/047-t-08.3-restart-resync-and-key-schema-documentation.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 8 of 17 — settings stop being interim files |
| **Area** | `services/settingsd/` · shell/compositor consumption |
| **Depends on** | T-01 (may run in parallel with T-07; the desktop provider has no adapter dependency) |
| **Blocks** | T-09, T-10, T-12 |
| **Legacy detail** | [legacy/15-settingsd-settings-model.md](../../tasks/legacy/15-settingsd-settings-model.md) (MVP slice) · [legacy/10-dock.md](../../tasks/legacy/10-dock.md) (dock.* keys) · [legacy/03-input-keymaps-shortcuts.md](../../tasks/legacy/03-input-keymaps-shortcuts.md) (input keys) · [08-settings.md](../08-settings.md) |

## Demo

```
change a setting (Dock size, magnification, auto-hide, dark/light, reduced
motion, Space count) from the D-Bus API / a test client
→ every consumer reacts live within one interaction beat (Dock re-lays-out,
  theme flips, compositor applies the motion policy)
→ kill -9 settingsd → nothing visually lost
→ restart settingsd → clients re-sync; a change made while it was down is not
  silently applied
```

Capture: `docs/captures/t08-settingsd.*` (a scripted D-Bus flip with the Dock
and theme reacting).

## Why now

The shell currently persists `dock.*` in its own JSON file (`DockSettings`,
`DockPins`) and the design-system theme is bound to host defaults. Those are
explicitly interim, and the Settings app (T-09) and Files (T-10) both consume
settings. Landing the owner now removes the debt before two more consumers
build on it, and it is the smallest slice that makes "live settings" true
everywhere.

## Inherited and reused

- The interim file is already written in the eventual
  `org.dragonfruit.Settings1` key shape, so adoption needs no migration
  (`legacy/10-dock.md`, section 19).
- The shell's `QFileSystemWatcher` stand-in for change signals is replaced by
  real D-Bus signals.
- Existing keys: `dock.*`, `accessibility.reduceMotion`, input repeat/gesture
  policy, Spaces/gesture policy.

## Scope

### In

1. **Desktop-settings provider**: our own configuration model (Dock,
   workspaces, gestures, appearance dark/light/accent, animation policy,
   input repeat) persisted by settingsd only under
   `$XDG_CONFIG_HOME/dragonfruit/`.
2. **D-Bus API** `org.dragonfruit.Settings1`: named, schema-documented keys;
   get/set; change signals (no polling); additive-only within a release.
3. **Persistence**: schema-documented format; atomic writes; startup
   migrations; the existing shell file is adopted without migration.
4. **Consumers migrate**: the shell drops `DockSettings`/`DockPins`; the
   compositor consumes motion policy and input settings; the design-system
   `Theme.dark`/`Theme.reducedMotion` are bound to the daemon.
5. **Restart behavior**: settingsd is restartable; clients re-read cached
   state on reappearance; no write is lost (journal/atomic writes).
6. **Key schema documented in-repo** with every key's owner and consumer.

### Out / explicitly deferred

- Host-service provider proxying and the distro `FedoraProvider` (T-15/T-16).
- The Settings app UI (T-09).
- Files app preferences (owned by Files, T-10).

## Acceptance

- [ ] The demo runs and the capture is committed.
- [ ] Every named key round-trips get/set/notify; a signal-based test flips a
      representative key per provider and observes all consumers react.
- [ ] Kill/restart of settingsd: compositor and shell re-sync; no setting is
      visually lost.
- [ ] The key schema is documented with owners and consumers.
- [ ] A migration test across two schema revisions passes.
- [ ] `make e2e` and `make soak` stay green; T-01/T-07 demos still pass.

## Test plan

- Unit: schema, migrations, atomic writes.
- Integration: D-Bus signal fan-out to shell/compositor; restart resync.
- Regression: the existing Dock settings behavior and the design-system
  reduced-motion tests.

## Risks

- **Key granularity**: design the schema against the T-09 Wave-1 panes before
  freezing; additive-only makes later keys safe, renaming is not.
- **Two writers during migration**: remove the shell's file watcher in the
  same change that lands the daemon, or they will clobber each other.
- **Do not route everything through settingsd**: Files preferences and
  notification state stay with their owners.

## Hand-off

- T-09 binds Wave-1 panes to this API.
- T-10 binds Files preferences to their own store, not settingsd.
- T-15 adds the host-services provider.

## The key schema and D-Bus surface (T-08.1a)

T-08.1a landed the desktop-settings model and the `org.dragonfruit.Settings1`
surface in `services/settingsd` (ADR
[0030](../adr/0030-settingsd-schema-and-dbus-surface.md)); persistence is
T-08.1b. The schema lives in code as `schema::KEYS` — one declaration per key
with its D-Bus type, default, range/enumeration, and owner/consumer pair — and
`SchemaVersion` (`u`) reports `SCHEMA_VERSION`. Renames and removals are
forbidden within the `1` series; a frozen v1 manifest test enforces it.

Values are typed D-Bus variants (`b`, `d`, `x`, `s`, `as`), not JSON strings.
The interface at `/org/dragonfruit/Settings1` is `Get(key) -> v`,
`Set(key, value)`, `GetAll() -> a{sv}` (one-call resync), `ListKeys() -> as`,
plus the `SchemaVersion` property and one `Changed(key, value)` signal emitted
only on a real change. Rejections carry `org.dragonfruit.Settings1.Error`
names (`UnknownKey`, `TypeMismatch`, `OutOfRange`, `NotAllowed`). Consumers
never poll: there is no timer and no read loop.

| Key | Type | Default | Owner → consumer |
|---|---|---|---|
| `dock.size` | d (0..1) | 0.5 | shell/Dock |
| `dock.magnification` | d (0..1) | 0.5 | shell/Dock |
| `dock.position` | s (bottom/left/right) | bottom | shell/Dock |
| `dock.autohide` | b | false | shell/Dock |
| `dock.animateOpening` | b | true | shell/Dock |
| `dock.showIndicators` | b | true | shell/Dock |
| `dock.minimizeIntoTileIcon` | b | false | shell/Dock → compositor motion |
| `dock.minimizedAnimation` | s (genie/scale/none) | scale | shell/Dock → compositor motion |
| `dock.titlebarDoubleClick` | s (zoom/minimize/none) | zoom | settingsd → compositor SSD |
| `dock.showRecentApps` | b | false | shell/Dock |
| `dock.pinned` | as | [] (seeds defaults) | shell/Dock |
| `workspaces.count` | x (1..16) | 3 | settingsd → compositor workspace model, shell |
| `gestures.enabled` | b | true | settingsd → compositor/input |
| `gestures.spaceSwitch` | b | true | settingsd → compositor/input |
| `gestures.missionControl` | b | true | settingsd → compositor/input |
| `appearance.colorScheme` | s (light/dark/auto) | auto | settingsd → compositor + shell Theme |
| `appearance.accent` | s (`#rrggbb`, empty = default) | "" | settingsd → shell Theme |
| `accessibility.reduceMotion` | b | false | settingsd → shell Theme, compositor motion |
| `input.repeatDelay` | x (0..5000 ms) | 200 | settingsd → compositor/input |
| `input.repeatRate` | x (0..200 Hz) | 25 | settingsd → compositor/input |

The key names and defaults match the shell's interim
`$XDG_CONFIG_HOME/dragonfruit/settings.json` (`dock.*`,
`accessibility.reduceMotion`), so T-08.1b adopts that file without migration.
The integration test in `services/settingsd/tests/session_bus.rs` drives the
interface over a private session bus; `make e2e` runs it.

## Persistence, atomic writes, and migrations (T-08.1b)

`services/settingsd/src/persist.rs` owns the on-disk state (ADR
[0031](../adr/0031-settingsd-persistence-format-and-atomic-writes.md)).
Desktop settings live in `$XDG_CONFIG_HOME/dragonfruit/settings.json`, with
`$HOME/.config/dragonfruit/settings.json` as the fallback, in the shell's
existing shape:

```json
{
  "schema": 1,
  "keys": {
    "dock.size": 0.5,
    "dock.autohide": false,
    "dock.pinned": ["a.desktop", "b.desktop"]
  }
}
```

`schema` is `SCHEMA_VERSION`; each entry under `keys` is JSON whose shape
follows the key's declared type (bool, number, string, or string array). A
file with no `schema` field is revision 0 and is migrated at startup: the
revision is stamped and every key the older file predates is filled with its
default (a key's `since` revision drives this). The upgraded file is written
back once. A value that no longer validates falls back to its default rather
than rejecting the file; a malformed or unreadable file is reported and the
daemon runs from defaults.

Saves are **atomic**: the document is staged in a hidden sibling of the target
(`.settings.json.<pid>.<n>.tmp`), flushed with `sync_all`, then `rename(2)`d
over `settings.json`. A process that dies between staging and the rename
leaves the previous file intact and the stale temporary file inert. Root
fields other than `schema`/`keys` and key names outside the schema are
preserved verbatim across a save, so settingsd does not clobber the interim
shell writer until T-08.2a removes it.

A `Set` that really changes a value persists **before** its `Changed` signal,
so a consumer reacting to the signal can rely on the durable file already
holding the new value. `dragonfruit-settingsd --config-path` prints the
resolved path. Tests: `persist.rs` unit tests cover the round-trip, the
staged-write failure leaving the prior file intact, the revision-0 migration
fixture, unknown-entry preservation, and invalid-value fallback; the
`session_bus.rs` integration test serves the daemon with a persistence path
and asserts a `Set` lands on disk and reloads after a restart.

## The shell consumes settingsd (T-08.2a)

The shell's interim `DockSettings`/`DockPins` file owners and their
`QFileSystemWatcher` are **gone**. The Dock reads and writes every `dock.*`
key through one `SettingsClient` (`shell/src/settingsclient.h`):

- `DbusSettingsClient` is the live client for `org.dragonfruit.Settings1`. It
  is seeded with the schema defaults, subscribes to `Changed` (never polls),
  resyncs with `GetAll` when the name appears, and mirrors a write with `Set`.
  The local store updates first so the Dock reacts on the same event-loop
  turn; the daemon's `Changed` echo is de-duplicated by value.
- `MockSettingsClient` is the in-process fixture for the headless tests.

The typed Dock view is `DockConfig` + `dockConfigFromValues()` in
`shell/src/dockmodel.cpp`; `dockIconSize()` maps `dock.size` onto the icon
token range and `resolveDefaultDockPins()` (moved out of the deleted
`DockPins`) supplies the first-run pin set.

**Absent daemon.** The demo/dev tool deliberately does not start settingsd;
the shell then runs entirely from the schema defaults plus in-memory writes,
so the Dock still lays out and resizes. When the daemon appears its
`GetAll` snapshot becomes authoritative. `dock.pinned` is seeded with the
installed defaults once per session when it is empty (the schema default);
there is no file-existence check to consult anymore, so an intentionally
emptied pin set is re-seeded on the next launch (a known corner; T-08.3 can
carry a "seeded" marker if it matters).

**Verification.** `tst_dockcore` covers the typed view, the icon-size map,
the default pins, and the overflow re-layout. `tst_settingsclient` drives a
fake `org.dragonfruit.Settings1` service on a private bus
(`dbus-run-session`) and asserts the live `GetAll`/`Changed`/`Set` path; it
skips the bus half where no bus exists. The capture
`docs/captures/t08-settingsd.*` is the track's scripted flip.
