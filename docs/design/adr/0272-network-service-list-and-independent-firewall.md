# ADR 0272 — Network service list, independent firewall, and the optional VPN pane

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-15 |
| **Context** | T272, [24-system-settings.md](../24-system-settings.md) §4.3 |

## Context

The Network pane combined NetworkManager and the firewall into one view, but
the pane's `ready`/`readOnly` flags were a single switch. A missing
NetworkManager hid the firewall, and a read-only VPN adapter disabled the
firewall toggle. Services had no status dots and one text-wall Advanced dialog
stood in for per-service details. The VPN entry point was a navigation link to
Network even when a profile existed, and Add Service offered only three kinds.

## Decision

1. **The firewall is its own state.** `NetworkPane` derives `firewallWritable`
   from the firewall sub-view (`detected`, `statusKnown`, `readOnly`) only.
   NetworkManager's `ready`/`readOnly` and the VPN adapter's `readOnly` never
   feed it. The combined `network` view stays the transport, but the pane
   treats its two halves independently. A NetworkManager-absent fixture
   (`nm-absent`) keeps the firewall live and the sections visible.
2. **One service row anatomy.** Each row carries a status dot (green
   connected, yellow connecting/disconnecting, red failed, gray idle), a
   Details sheet, and an actions menu. The menu owns connect/disconnect,
   move up/down, make inactive, delete, and add. Order remains
   NetworkManager's `autoconnect-priority`; no second order model exists.
3. **Details, not a text wall.** The Details sheet reads the live address,
   subnet, router, DNS, search domains, IPv6, proxy, hardware, metered, MAC
   randomization, 802.1X, interface, and auto-connect. These rows are
   read-only live values; the full connection editor owns their writes.
4. **VPN is an optional top-level pane.** It is a real shipped pane in the
   catalog, gated by `optionalVisible` on a NetworkManager VPN profile. When
   no profile exists it leaves the sidebar and Network → VPN remains the
   entry point. It reads the same one VPN view as Network → VPN.
5. **Every write reports its outcome.** `NetworkClient` and `VpnClient` emit
   `writeFinished(operation, outcome, message)`; the bridge publishes
   `networkWrite*`/`vpnWrite*` and the panes show a failure line. A failed
   write leaves the view untouched.
6. **Add Service covers the reference kinds.** Ethernet, Wi-Fi, VPN, bridge,
   bond, and VLAN travel through one `AddConnectionSpec` with an optional
   interface name; the sheet validates required fields and shows a result.

## Consequences

- The pane has two independent failure surfaces (NetworkManager and firewall)
  and a third optional one (VPN). Tests must stop each adapter separately.
- The VPN pane is visible only after a VPN read. `SettingsShell` performs one
  explicit `refreshVpn()` at startup so the sidebar can gate on a live
  profile; this is not a poll.
- `NetworkService` grows prefix/gateway/search-domain/IPv6 fields. The
  D-Bus source reads them from `IP4Config`/`IP6Config` and the connection
  settings map; fixtures provide them directly.
- The old `Advanced…` row and dialog are removed. The map's `advanced` row
  becomes `details`, with separate read-only `metered`, `mac`, `ipv6`, and
  `8021x` rows.
