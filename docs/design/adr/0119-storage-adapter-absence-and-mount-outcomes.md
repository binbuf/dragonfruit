# 0119 — The storage adapter: UDisks2 state, explicit mount/eject, and absence

## Status

accepted

## Context

T-15.2a is the storage adapter of T-15 (the track lists A6 storage over
"UDisks2 + GIO/GVfs"). It follows the Bluetooth adapter ([0117](0117-bluetooth-adapter-absence-and-write-outcomes.md))
and the T-07 contract: reuse the host stack, expose state + events, and treat
absence as a normal state. The backend is the whole task; the Settings pane and
Control Center tile are T-15.2b. The track demo needs to mount/eject a USB
drive and see it in Files and the sidebar, so the adapter must carry mount
state and the three writes.

## Decision

- **`services/storage`** (`dragonfruit-storage`) is its own crate behind the
  dependency-free contract, per [0026](0026-concrete-adapters-in-their-own-crates.md).
  It reaches UDisks2 over the **system bus** as `org.freedesktop.UDisks2`; one
  `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at
  `/org/freedesktop/UDisks2` enumerates every `Drive` and `Block` object, and
  the property maps are decoded directly (a missing optional property is a
  default, not an error).
- **UDisks2 is the one consumed stack, not GIO/GVfs.** UDisks2 already
  provides the block enumeration, the `HintAuto`/`HintSystem`/`HintIgnore`
  user-interesting hints, and the mount operations, so the adapter would
  reimplement GIO's volume monitor to consume both. GIO/GVfs stays where it
  already lives — the Files sidebar's volume view (`files-core`). "Reuse,
  never reimplement" is read as one source of truth, not two.
- **The transport seam is `StorageSource`**: `read` returns the raw
  `StorageData` (`Ok(Some)`), absence (`Ok(None)`), or a read failure (`Err`);
  `mount`/`unmount`/`eject` are the one-call writes. `DbusUDisks` is the live
  source; `MockStorage` serves a fixture and has `kill`/`restart` for the
  lifecycle tests.
- **Writes are addressed by UDisks2 object path**, not by device node: paths
  are stable and unambiguous, and the snapshot carries them.
- **The model joins each `Block` to its `Drive`** and keeps only mountable
  blocks that are not `HintIgnore`; it exposes mounted/removable/internal
  filters and a display name (label, then device node, then UUID). It maps no
  UDisks2 enums.
- **A successful write invents no snapshot.** UDisks2 pushes the resulting
  property changes, the host calls `StorageAdapter::refresh`, and the snapshot
  stays the single source of truth.
- **A polkit refusal is a per-request `StorageOutcome::Denied`**, not a
  session-wide read-only degradation: the read state stays live. A busy device
  that refuses to unmount is `Failed`, not `Denied`.
- **Two hide rules, as for Bluetooth.** UDisks2 absent ⇒ `Unavailable`
  (hidden, not an error). UDisks2 present but no mountable volume ⇒
  `Available` with `StorageSnapshot::present()` false (hidden by the
  consumer).

## Consequences

- T-15.2b renders `StorageSnapshot` and calls `mount`/`unmount`/`eject`; a
  refused action surfaces its own `Denied` note.
- T-15.16's absent-daemon matrix adds a Storage column keyed off the state
  (`Unavailable`/`Error`) plus `present()`.
- Locked encrypted volumes (the `Encrypted` interface) have no `Filesystem`
  until unlocked, so the adapter does not list them as mountable volumes;
  unlock/format are explicitly out of scope here.
- The live D-Bus source is compile-checked but not exercised in CI; the tested
  contract is the fixture-driven seam, mirroring the other adapters.