# 0117 — The Bluetooth adapter: BlueZ state, explicit writes, and absence

## Status

accepted

## Context

T-15.1a is the first breadth adapter of T-15 and the first BlueZ consumer. The
track demo needs discover/pair/connect, but the task's scope is the backend
adapter only (the pane and Control Center tile are T-15.1b). The adapter must
reuse the host stack, expose state + events, and treat absence as a normal
state, like the T-07 NetworkManager/audio/power adapters. BlueZ has one
property the other adapters did not: its writes are a mix of polkit-authorized
operations (power, pairing) and unprivileged ones (discovery, connect), so the
Wi-Fi "degrade the whole adapter to read-only on one denial" model does not
fit.

## Decision

- **`services/bluetooth`** (`dragonfruit-bluetooth`) is its own crate behind
  the dependency-free contract, per [0026](0026-concrete-adapters-in-their-own-crates.md).
  It reaches BlueZ over the **system bus** as `org.bluez`; one
  `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at `/` reads every
  `org.bluez.Adapter1` and `org.bluez.Device1` object, and the property maps
  are decoded directly (no per-property proxy round trips, so a missing
  optional property is a default, not an error).
- **The transport seam is `BluetoothSource`**: `read` returns the raw
  `BluetoothData` (`Ok(Some)`), absence (`Ok(None)`), or a read failure
  (`Err`); `set_powered`, `set_discovering`, `pair`, and `set_connected` are
  the one-call writes. `DbusBluez` is the live source; `MockBluetooth` serves a
  fixture and has `kill`/`restart` for the lifecycle tests.
- **A successful write invents no snapshot.** BlueZ pushes the resulting
  `PropertiesChanged`/`InterfacesAdded`, the host calls
  `BluetoothAdapter::refresh`, and the snapshot stays the single source of
  truth.
- **A polkit refusal is a per-request `BluetoothOutcome::Denied`**, not a
  session-wide read-only degradation: the read state stays live and the adapter
  does not set an access mode. A write against an absent daemon is
  `BluetoothOutcome::Absent` and moves the adapter to `Unavailable`.
- **Two hide rules, as for the battery.** `bluetoothd` absent ⇒ state
  `Unavailable` (hidden, not an error). BlueZ present but no controller ⇒
  `Available` with `BluetoothSnapshot::present()` false (hidden by the
  consumer). A present-but-unpowered controller is still present and shown
  with an Off switch.

## Consequences

- T-15.1b renders `BluetoothSnapshot` and calls the four write methods; it gets
  no read-only access mode to key off, so each refused action surfaces its own
  `Denied` note.
- T-15.16's absent-daemon matrix adds a Bluetooth column keyed off the state
  (`Unavailable`/`Error`) plus `present()`.
- The live D-Bus source is compile-checked but not exercised in CI; the tested
  contract is the fixture-driven seam, mirroring the other adapters.