# 0271 — Wi-Fi and Bluetooth are one host model, a structural absence, and explicit write outcomes

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-15 |
| **Supersedes** | none; extends ADR 0024/0025/0026/0027/0117/0118/0262 |
| **Companions** | [24-system-settings.md](../24-system-settings.md), [08-settings.md](../08-settings.md), [settings-business-logic-map.md](../settings-business-logic-map.md) |

## Context

The first Wi-Fi/Bluetooth pass (T225, T-15.1b) landed the bridge-host
projection, the radio writes, joins/forgets, and a device list. The 2026-11-14
review found four gaps:

- Wi-Fi had no personal-hotspot row, no scan state, no known-network details or
  disconnect, and its Advanced sheet was a dead end.
- The join password survived cancel, and a failed write was invisible because
  the dialog closed optimistically.
- Bluetooth's info button performed a connect instead of opening a device
  sheet; there was no Forget; discovery did not start on power-on; the
  discoverable caption repeated the adapter id.
- Both panes hid whole sections when the adapter was absent, which contradicts
  the no-blank-panes contract.

## Decision

1. **One model per daemon, no fork.** The Settings pane and the shell's quick
   sheet/status menu read the same bridge-host view. The pane never caches a
   connection; every write goes to the adapter and the host's re-read is the
   single source of truth.
2. **A write reports its outcome and never invents state.** `WifiClient` and
   `BluetoothClient` emit `writeFinished(operation, outcome, message)`; the
   bridge publishes `wifiWrite*`/`bluetoothWrite*`. A refused write leaves the
   view untouched, the bound control reverts, and the pane shows the reason.
3. **Absence is structural.** Every section stays mounted. Only controls with
   a real host owner disable with a reason; the settingsd rows stay live.
4. **Personal Hotspot is a real NetworkManager AP-mode connection.** Start is
   `AddAndActivateConnection` with `802-11-wireless.mode=ap`; stop deactivates
   the saved connection. The row is disabled with a reason when no managed
   Wi-Fi device exists.
5. **Scanning is a host flag answered by the next read.** `RequestScan` marks
   the view `scanning`; the next NM read clears it. The pane renders
   scanning/empty/error as rows, never as a blank group.
6. **Bluetooth device info is a sheet, not an action.** It shows address,
   type (from BlueZ `Icon`), battery (`Battery1.Percentage`), status, and
   Connect/Disconnect/Forget. The discoverable row names the computer; the
   adapter address lives in Advanced.
7. **The two `network.wifi.*` prompt keys are `stored-only`.** The shell has no
   Wi-Fi prompt consumer yet. The map and schema say so, and the pane explains
   that NetworkManager's autoconnect remains authoritative.

## Consequences

- The pane tests cover scanning, empty, error, join/cancel password clearing,
  known details/disconnect, hotspot start/stop, device info/forget/connect,
  power-on discovery, structural absence, a failed-write revert, the 740×820
  window, and external prompt-policy convergence.
- `make t271-wifi-bluetooth-capture` records live/scanning/connected/off/
  details/hotspot/device/absence stills in light and dark.
- A future shell Wi-Fi prompt consumer can flip the two keys back to live
  without changing the pane's layout; the map flag is the single switch.
- LE Audio remains a presentation note until BlueZ/PipeWire expose it.

## References

- [24-system-settings.md](../24-system-settings.md) §4.1–4.2
- [08-settings.md](../08-settings.md)
- [settings-business-logic-map.md](../settings-business-logic-map.md)
- ADR 0027, 0117, 0118, 0262
