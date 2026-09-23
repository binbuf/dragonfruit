# T-08 — settingsd: One Owner for Desktop Settings

> **Track, not a single slice.** This file is the design reference. It is executed as 3 one-session units: [T-08.1](units/033-t-08.1-settingsd-daemon-core-and-d-bus-api.md) · [T-08.2](units/034-t-08.2-consumer-migration-to-settingsd.md) · [T-08.3](units/035-t-08.3-restart-resync-and-key-schema-documentation.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 8 of 17 — settings stop being interim files |
| **Area** | `services/settingsd/` · shell/compositor consumption |
| **Depends on** | T-01 (may run in parallel with T-07; the desktop provider has no adapter dependency) |
| **Blocks** | T-09, T-10, T-12 |
| **Legacy detail** | [legacy/15-settingsd-settings-model.md](legacy/15-settingsd-settings-model.md) (MVP slice) · [legacy/10-dock.md](legacy/10-dock.md) (dock.* keys) · [legacy/03-input-keymaps-shortcuts.md](legacy/03-input-keymaps-shortcuts.md) (input keys) · [08-settings.md](../design/08-settings.md) |

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
