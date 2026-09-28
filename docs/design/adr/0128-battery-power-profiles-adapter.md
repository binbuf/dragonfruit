# 0128 — The battery and power profiles adapter reads two independent daemons

- **Status:** Accepted (T-15.6a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [15-system-services-breadth.md](../tracks/15-system-services-breadth.md),
  ADR [0024](0024-system-adapter-contract.md),
  ADR [0117](0117-bluetooth-adapter-absence-and-write-outcomes.md)

## Context

T-07.4 shipped `dragonfruit-power` as a **read-only** UPower adapter for the
menu-bar battery item; its crate docs explicitly deferred power profiles to
T-15. T-15.6a is that task: battery **and** power profiles, with state, events,
and absent-daemon behavior.

Two details make the naive "add a `profiles` field to the one read" design
wrong. First, UPower and power-profiles-daemon are **separate system-bus
daemons** that can be masked independently: a desktop has no battery but may
run power-profiles-daemon, and a laptop can have power-profiles-daemon masked
with its battery perfectly live. A single `Unavailable` state would conflate
them and hide a working half. Second, unlike Bluetooth/storage/audio, no
consumer yet diffs two reads to learn what moved; T-15.6a is the first power
consumer that needs an event stream (the pane and tile are T-15.6b).

The upstream name also moved: power-profiles-daemon started as
`net.hadess.PowerProfiles` and now also owns
`org.freedesktop.UPower.PowerProfiles`, with a matching object path and
interface.

## Decision

Extend `dragonfruit-power` in place (one crate owns the subsystem); keep the
`PowerSource` seam and the shared `Subscription` lifecycle.

1. **One read, two half-results.** `PowerData` gains
   `profiles: Option<PowerProfilesData>`; the battery fields are unchanged but
   gain `Capacity` and `ChargeCycles`. `Ok(None)` means **both** daemons are
   unreachable (the only `Unavailable`), `Err` means a daemon that owns its
   name is unreadable, and `Ok(Some)` carries whichever halves answered.
   Absence is therefore per daemon and never an error.
2. **The profile vocabulary is typed.** `PowerProfile` enumerates
   `power-saver` / `balanced` / `performance` with stable ids and glyphs, in
   canonical order; `PowerProfilesSnapshot` carries the active profile, the
   supported subset, the inhibited/degraded state, and the hold count. An
   unrecognized active id is `None`, never guessed.
3. **The one write is explicit and snapshot-neutral.** `set_active_profile` is
   a single `Properties.Set` of `ActiveProfile`; a successful write invents no
   snapshot and the host re-reads, exactly as Bluetooth/storage/audio writes.
   The trait's default is `ProfileOutcome::Absent`, so a source without the
   daemon satisfies it and the battery half is unaffected.
4. **Events are a pure diff.** `PowerSnapshot::changes(previous)` reports
   battery level/state/health/on-battery moves, available/active profile moves,
   performance-degradation changes, and hold-count moves. The adapter queues
   them in `drain_changes`; nothing above the adapter polls.
5. **Both daemon generations are probed.** The live source checks
   `net.hadess.PowerProfiles` then `org.freedesktop.UPower.PowerProfiles` and
   speaks to the first owner. `Capacity`/`ChargeCycles` are read through
   `Properties.Get` and accepted as double or integer, because UPower's
   spelling of `Capacity` has varied.
6. **Health is a documented projection.** `BatteryHealth::from_capacity`
   maps the raw `Capacity` percentage to `Normal`/`Service` at a stated 80%
   threshold; the pane shows the word and the raw percentage.

Rejected: a separate `dragonfruit-power-profiles` crate (the two halves drive
one pane/tile and share the system-bus connection); a single combined presence
flag (loses independent absence); the old UPower-only name (broken on current
daemons).

## Consequences

- `PowerData` and `PowerDeviceData` gain public fields; every in-tree struct
  literal was updated. `system-status`'s `battery_view` is unchanged and does
  not yet project the profiles half.
- T-15.6b adds the Settings pane and Control Center tile over the status bridge
  host and may extend `battery_view` (or add a sibling interface) with the
  profile list and the `SetActiveProfile` write. Until then the adapter is the
  contract/model, exercised by its own crate tests.
- Holding/inhibiting a profile (`HoldProfile`/`ReleaseProfile`) is read
  (`holds`) but not written; no consumer needs it yet. A later task adds the
  write if one appears.
- A machine that runs only power-profiles-daemon is `Available` with
  `present: false`; the shell's existing second hide rule (present, not
  adapter state) still hides the battery item, so the T-07 masking matrix is
  unchanged.