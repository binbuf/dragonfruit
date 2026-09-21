# T-15 — settingsd: Desktop Settings Daemon and Provider Model

| | |
|---|---|
| **Phase** | 3 · Flagship apps |
| **Area** | `services/settingsd/` |
| **Depends on** | [T-01](01-repo-scaffolding-ci-licensing.md) · [T-20](20-system-service-adapters.md) (host-services providers) |
| **Blocks** | [T-16](16-settings-app.md) · [T-14](14-hot-corners-desktop-background.md) (persistence) · [T-10](10-dock.md) (dock pinning keys) |
| **Estimate** | L |
| **Design docs** | [08-settings.md](../design/08-settings.md) · [07-system-integration.md](../design/07-system-integration.md) · [01-architecture.md](../design/01-architecture.md) |

## Summary

`settingsd`: the single owner of desktop settings and their persistence.
Three provider kinds meet inside it (desktop settings we own; host services
via adapters; the distro provider for distribution-specific
administration), exposed over `org.dragonfruit.Settings1` with change
signals instead of polling, additive schema evolution, and startup
migrations.

## Background

Exactly one process owns each class of state
([01-architecture.md](../design/01-architecture.md)): desktop settings and
their persistence belong to `settingsd`. It is independently restartable;
clients re-read cached state on reappearance. Settings should say "check
for updates," not "execute a `dnf5` command" — the distro adapter is
isolated behind a trait ([08-settings.md](../design/08-settings.md)).

## Scope

### In scope

1. **Three provider kinds**:
   1. **Desktop settings** — our own configuration model: Dock (pinning,
      auto-hide, magnification), workspaces (count/behavior), gestures,
      appearance (themes, dark/light, accent), animation policy —
      persisted by us.
   2. **Host services** — NetworkManager, BlueZ, UPower, UDisks, PipeWire
      via the adapters (T-20) — proxied through, never reimplemented.
   3. **Distro adapter** — distribution-specific administration (updates,
      package management) behind a provider interface.
2. **D-Bus API** `org.dragonfruit.Settings1`:
   - Named, schema-documented keys.
   - Clients **observe changes via signals rather than polling**, so the
     compositor, shell, and apps react to a single source of truth.
   - Suffix per major version (`…1`), additive-only within a release —
     same policy as private Wayland protocols
     ([01-architecture.md](../design/01-architecture.md)).
3. **Persistence**:
   - `$XDG_CONFIG_HOME/dragonfruit/`, **written by settingsd only**.
   - Schema-documented format with named keys.
   - **Schema changes are additive within a release; migrations run at
     settingsd startup.**
4. **Distro provider interface** (from
   [08-settings.md](../design/08-settings.md)):
   ```rust
   trait SystemProvider {
       async fn distribution_info(&self) -> DistributionInfo;
       async fn check_updates(&self) -> Result<Vec<Update>>;
       async fn install_updates(&self) -> Result<()>;
       async fn reboot(&self) -> Result<()>;
   }
   ```
   with a `FedoraProvider` now and a `DebianProvider` later — everything
   above the interface stays identical.
5. **Routing rules** encoded as the authority the Settings app consumes
   (Displays→compositor, Keyboard/Mouse/Trackpad/workspace/Dock→our
   services, Wi-Fi→NM, Bluetooth→BlueZ, Sound→PipeWire, Power→UPower,
   Storage→UDisks/GIO, Updates→distro provider).
6. **Restart behavior**: restartable; clients re-read cached state on
   reappearance; no settings writes are lost (journal/atomic writes).

### Out of scope

- The Settings **app** UI (T-16).
- Host-daemon logic itself (T-20).
- Files' app preferences (Files state, not desktop-wide —
  [09-files.md](../design/09-files.md)).

## Requirements

- FR-1: Every named key round-trips get/set/notify; observers receive
  change signals (no client polls).
- FR-2: Compositor, shell, and first-party apps all consume the same keys
  through the same D-Bus interface — verified by an integration test that
  flips one appearance key and observes all consumers react.
- FR-3: Startup migration: an old-format config upgrades in place, lossless
  for additive changes; destructive changes forbidden within a release.
- FR-4: `FedoraProvider` implements the trait; a `MockProvider` drives
  tests; `DebianProvider` is a stub with a documented plan (T-32).
- FR-5: settingsd never needs the system bus and never runs as root;
  privileged operations delegate to host services behind polkit
  ([07-system-integration.md](../design/07-system-integration.md)).
- FR-6: Absent host daemons surface as explicit "unavailable" states on the
  relevant keys — never errors, never startup blockers.
- FR-7: Kill/restart of settingsd: compositor/shell re-sync, no setting
  visually lost (scripted).

## Acceptance criteria

- [ ] Key schema documented in-repo with every key's owner and consumer.
- [ ] Signal-based change-notification test passes for representative keys
      of each provider kind.
- [ ] Migration test across two schema revisions passes.
- [ ] Provider-mock test matrix (all daemons absent) passes.

## Test plan

- Unit: schema, migrations, atomic writes.
- Integration: D-Bus signal fan-out; restart resync.
- VM: daemon-masking matrix (Phase-4 exit pattern).

## Risks / open questions

- Key granularity (per-Dock vs per-item pinning etc.) — design the schema
  for the panes listed in T-16 before freezing; additive-only makes later
  keys safe but renaming is not.
- Avoid temptation to route *everything* through settingsd — Files
  preferences and notification-service state stay with their owners.
