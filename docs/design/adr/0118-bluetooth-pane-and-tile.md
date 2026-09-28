# 0118 — The Bluetooth pane and tile ride the status bridge host

## Status

accepted

## Context

T-15.1a landed `dragonfruit-bluetooth` as a library with no consumer. T-15.1b
must ship the Settings Bluetooth pane and the Control Center tile together (no
half-panes), both rendering the same adapter and calling its four writes. The
C++ Settings app and shell cannot link a Rust adapter (D-Bus is the services
seam, [0029](0029-system-status-bridge-host.md)); the other menu-bar surfaces
already reach their adapters through `dragonfruit-system-status`. BlueZ is on
the **system** bus, and its adapter is not one of the three network/audio/power
adapters the existing `StatusHost` owns.

## Decision

- **The bridge host grows a Bluetooth surface, not a second service.** The
  system-status service serves a fourth interface at the same object path,
  `org.dragonfruit.SystemStatus1.Bluetooth`, backed by a small
  `BluetoothHost<B>` alongside (not inside) `StatusHost`. Keeping it separate
  avoids re-parameterizing the three-adapter host and its tests.
- **One JSON view, four writes.** `State()` returns the flat view
  `bluetooth_view` projects from the snapshot (state/present/powered/
  discovering/label/adapterName, plus `knownDevices` and `nearbyDevices`, each
  device carrying address, name, paired, connected, trusted, blocked, rssi and a
  0–100 signal). `Refresh`, `SetPowered`, `SetDiscovering`, `Pair`, and
  `SetConnected` mirror the Wi-Fi/audio shape. A write returns an outcome report
  (`accepted`/`denied`/`absent`/`failed`); the caller re-reads for state, so no
  write invents a snapshot.
- **The shell and the app each hold a thin client.** The shell's
  `DbusSystemStatusClient` gains the four writes and a `bluetoothState` signal
  (the Control Center tile and the model's second hide rule); the Settings app
  gets a small `BluetoothClient` seam with a `DF_BLUETOOTH_FIXTURE` in-process
  mock, exactly as the Wallpaper pane uses `DF_WALLPAPER_FIXTURE`. Panes never
  touch D-Bus directly.
- **Absence is the adapter's two hide rules, surfaced, not errored.** With no
  bridge host, no `bluetoothd`, or no controller, the view is empty/
  `unavailable`/`present:false`; the tile hides and the pane disables its
  controls and shows a one-line note. A polkit denial is a per-action report
  ([0117](0117-bluetooth-adapter-absence-and-write-outcomes.md)); the read state
  stays live.
- **Opening the pane owns discovery.** The pane starts an inquiry on open and
  stops it on close (the reference's discoverable caption depends on it); this
  is a UI lifecycle, not a poll.

## Consequences

- T-15.16's absent-daemon matrix can drive the Bluetooth column through this
  interface (mask the host, mask `org.bluez`, or drop the controller) and the
  `DF_BLUETOOTH_FIXTURE` seam for the pane.
- The Settings `Bluetooth Settings…` link and the shell's equivalent remain
  entry points that log until T-16 wires launching Settings on a named pane,
  matching the existing Wi-Fi/Focus/Appearance links.
- The menu-bar Bluetooth status slot stays hidden: the task owns the Control
  Center tile, and a visible slot with no Bluetooth menu would be a dead
  control.