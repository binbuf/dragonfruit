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
key through one `SettingsClient`:

- `DbusSettingsClient` is the live client for `org.dragonfruit.Settings1`. It
  is seeded with the schema defaults, subscribes to `Changed` (never polls),
  resyncs with `GetAll` when the name appears, and mirrors a write with `Set`.
  The local store updates first so the Dock reacts on the same event-loop
  turn; the daemon's `Changed` echo is de-duplicated by value.
- `MockSettingsClient` is the in-process fixture for the headless tests.

The client itself now lives in `libs/settings-client` (static library
`dragonfruit-settings-client`), shared by the shell's Dock/Theme/policy
controllers and the Settings app's QML `Settings` singleton (T-09.1b, ADR
[0036](../adr/0036-shared-settings-client-and-qml-singleton.md)). The
`Settings` singleton exposes the same reactive values map and `set` path to
panes; `DF_SETTINGS_FIXTURE` selects the mock for headless QML tests.

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

## The design-system Theme follows settingsd (T-08.2b)

`shell/src/themebinding.{h,cpp}` is the one writer of the design-system
`Theme` singleton's settings-driven appearance (ADR
[0033](../adr/0033-theme-binding-single-writer.md)). It consumes the **same**
`SettingsClient` as the Dock — no second bus connection, no watcher, no timer
— and maps:

- `appearance.colorScheme` (`light`/`dark`/`auto`) → `Theme.dark`;
- `accessibility.reduceMotion` (`b`) → `Theme.reducedMotion`.

`auto` follows the host and stays live: the binding listens to
`QStyleHints::colorSchemeChanged` and re-applies while the setting is neither
`light` nor `dark`, so replacing the singleton's own `Application.styleHints`
binding does not freeze the host preference. The resolver is the pure
`ThemeBinding::darkForScheme(scheme, hostDark)`. The shell no longer assigns
`Theme` from the Dock reconfigure path.

**Absent settingsd.** The client's schema defaults set
`appearance.colorScheme` to `auto`, so `Theme` follows the host exactly as
before the binding landed.

**Not yet.** `appearance.accent` is unconsumed: `Theme.color.accent` is a
read-only scheme token and an override needs a design-system change
(T-09.2, the Appearance pane). The compositor keeps its own scheme owner
(`DfState::set_color_scheme`); T-08.2c mirrors the same key into it, so a
light shell over the compositor's dark chrome/backdrop is an interim state.

**Verification.** `tst_themebinding` (shell tests) flips the mock client,
asserts `Theme.dark`/`Theme.reducedMotion` and the collapsed reduced-motion
durations, and grabs the component gallery to prove its rendered variant
follows the scheme.

## The compositor applies the motion/input policy (T-08.2c)

The compositor no longer holds its own motion/input defaults. It consumes the
settingsd-owned policy over the private `df_toplevel_manager` protocol (ADR
[0034](../adr/0034-compositor-policy-via-shell-bridge.md)):

- **`set_motion_policy`** (additive v5): the resolved `appearance.colorScheme`
  (`light`/`dark`; the shell resolves `auto` against the host),
  `dock.titlebarDoubleClick`, and `dock.minimizedAnimation`.
- **`set_input_policy`** (additive v5): `input.repeatDelay`/`input.repeatRate`
  and `gestures.enabled`/`gestures.spaceSwitch`/`gestures.missionControl`.
- `set_reduced_motion` (v3) stays the `accessibility.reduceMotion` path.

The shell is the **only forwarder** (no second D-Bus connection, no watcher, no
timer): `ShellController::applyCompositorPolicy()` re-reads the same
`SettingsClient` the Dock and Theme use, maps it through the pure
`shell/src/compositorpolicy.{h,cpp}`, and sends it on `changed`/`refreshed`,
on host `colorSchemeChanged` (for `auto`), and once after the protocol
authenticates. The compositor is the **only applier**
(`DfState::set_motion_policy`/`set_input_policy`); there is no local settings
file on either side.

`dock.minimizedAnimation=none` collapses the minimize/restore tween to one step
exactly like reduced motion; `genie` is accepted but currently renders as
`scale`. `workspaces.count` is not in this bridge — it is a workspace-model
key, not motion/input, and is a follow-up.

**Verification.** `compositor/tests/shell_protocol_conformance.rs::
motion_and_input_policy_requests_apply_live` drives the v5 requests and reads
the synthetic `query policy` report (scheme, titlebar double-click, minimized
animation, repeat delay/rate, gesture flags), then proves a disabled gesture
family stops firing and re-enabling restores it. `tst_compositorpolicy`
covers the pure settingsd → policy mapping, including `auto`.

## Restart, resync, and the key schema (T-08.3)

`settingsd` is independently restartable and no write is lost. The guarantee is
enforced end-to-end, not just in code:

- **Persist-before-signal.** `Set` writes the durable file before it emits
  `Changed` (T-08.1b, ADR [0031](../adr/0031-settingsd-persistence-format-and-atomic-writes.md)),
  so a `Set` that returned is already on disk. `cargo test -p
  dragonfruit-settingsd --test restart` runs the **real binary** over a private
  session bus: it sets two keys, `SIGKILL`s the daemon, asserts the
  well-known name was released, changes a value straight in the durable file
  while the daemon is down, restarts it, and asserts `GetAll` returns the two
  pre-kill writes plus the while-down change.
- **The shell re-syncs on reappearance.** `DbusSettingsClient` watches the
  well-known name with a `QDBusServiceWatcher`; when the daemon disappears it
  keeps its last-known values (`availableChanged false`, nothing visibly lost),
  and when the name reappears it calls `GetAll` and applies the snapshot. A
  write attempted while the daemon is down is local and in-memory only, so the
  resync reconciles it away: an unpersisted change is not silently kept.

**Bug found live (T-08.3).** The client originally used
`QDBusConnectionInterface::serviceRegistered`/`serviceUnregistered`. Those
signals only fire for a connection's **unique** name, not a well-known name
owned by another process, so the shell resynced once from the constructor's
`GetAll` and then never after a real settingsd restart. The
`QDBusServiceWatcher` fix is the reason the nested capture's restart frame
actually returns to the while-down state. `tst_settingsclient`'s restart case
now serves the fake daemon from a **separate bus connection** so it exercises
the remote path (a same-connection fake passed even before the fix).

**Key schema in-repo.** `docs/settings-keys.md` is the human-facing table: every
key's type, default, constraints, owner, consumer, and a consumer map. It is
kept in lockstep with `services/settingsd/src/schema.rs` by
`services/settingsd/tests/schema_doc.rs`, so adding a key (or editing an
owner/consumer) in code without updating the doc fails the build.

**Capture.** `scripts/capture-settingsd.sh` (`make settingsd-capture`) records
`docs/captures/t08-settingsd.*`: seed dark, launch the nested demo, flip
`appearance.colorScheme` + `dock.size` live, `kill -9` settingsd (nothing
visually lost), write dark + a smaller Dock to the durable file while it is
down, restart, and capture the shell re-syncing to that state. The raw pixels
are the evidence: the menu bar reads `(255,255,255)` after the light flip, is
unchanged after the kill, and returns to `(45,37,52)` on restart; the Dock
region changes between the `dock.size` 0.75 and 0.35 frames.
