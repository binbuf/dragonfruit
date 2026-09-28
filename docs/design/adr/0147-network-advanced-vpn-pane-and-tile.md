# 0147 — The Network advanced (VPN) pane and tile ride the bridge host

- **Status:** Accepted (T-15.15b)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  ADR [0146](0146-network-advanced-vpn-adapter.md),
  ADR [0122](0122-tahoe-interface-language-across-chrome.md)

## Context

T-15.15a landed the Network advanced (VPN) adapter inside
`dragonfruit-networkmanager`: a projection of NetworkManager's configured
`vpn`/`wireguard` connections and their live active connections, with two
explicit writes (`connect`/`deactivate`, by UUID) and a polkit read-only
degradation. T-15.15b must ship the Settings pane and the Control Center tile
as one functional unit. The Settings catalog already has the `network` row
(shipped false); Wi-Fi has its own `wifi` sidebar pane and Firewall has no
Linux host owner yet. As with the other T-15 panes (ADR 0143/0145), the C++/QML
side never links the Rust adapter: it decodes the bridge host's JSON view.

The Control Center panel already held seventeen tiles at the 1164 px the nested
output leaves below the bar; an eighteenth tile had to fit without clipping and
without a scrolling panel.

## Decision

- **One new bridge interface, no new daemon.** `services/system-status` serves
  `org.dragonfruit.SystemStatus1.Vpn` at the shared object path. Its
  `VpnHost<DbusVpn>` wraps the T-15.15a adapter and projects one flat `vpn`
  view: the connection list with `kind`/`state` ids and labels, the counts, the
  active name, the glyph (`vpn`/`vpn-off`), the aggregate label, `present`, and
  the read-only degradation (`readOnly`/`note`). The interface exposes the two
  writes `Connect`/`Deactivate`.
- **The pane mirrors the reference honestly.** The macOS Network capture shows
  Wi-Fi, Firewall, and Other Services and no VPN sub-pane, so the reference
  does not pin a VPN layout. The pane (`NetworkPane.qml`) carries the honest
  VPN surface: a `VPN` group with one row per configured connection and a live
  connect/disconnect toggle, the two adapter writes. Wi-Fi is the separate
  `wifi` pane; Firewall and Apple-only services (`Thunderbolt Bridge`, iCloud
  Private Relay) have no Linux host owner and are not invented as dead rows
  (ADR 0122). The pane has **no settingsd key**: NetworkManager owns the state.
- **Absence is layered.** The view is `unavailable` only when the system bus is
  unreachable or no `org.freedesktop.NetworkManager` owns its name; the item
  hides then. A daemon that answers with no VPN configured is `available` with
  `present: false`; the pane shows the empty note and the tile hides (the
  second hide rule). A readable list that polkit refuses to change stays
  visible: the writes disable and the pane shows the degradation note.
- **The Control Center tile is a read-only summary.** It carries the new
  original `vpn` glyph, the live label, and opens the Network pane when tapped.
  The panel's surface grows to the full 1164 px the nested output leaves below
  the bar and the Accessibility/VPN tiles become single-line compact summaries,
  so all eighteen tiles fit the fixed surface without scrolling.
- **The design system gains three glyphs.** `Icon` grows original `network`,
  `vpn`, and `vpn-off` marks for the sidebar row, the tile, and the pane.

Rejected: a second VPN implementation in the shell (the adapter owns the
projection); a durable settingsd copy of the connection state (the daemon is
the source of truth); a Firewall or Thunderbolt row invented without a host
owner; hiding the item when the list is readable but read-only (that is a
degradation, not absence).

## Consequences

- `VpnInterface` joins `interface_names()` (now thirteen); `main.rs` gains
  `--print-vpn`; `services/system-status` reuses its existing
  `dragonfruit-networkmanager` dependency.
- `apps/settings/VpnClient` is the pane's seam (`DF_VPN_FIXTURE` selects the
  in-process mock); `shell/src/systemstatusclient`, `systemstatusmodel`, and
  `shellcontroller` gain the read-only VPN path.
- The Control Center panel stays a non-scrolling fixed surface; its height is
  1164 and its compact tiles are single-line, recorded in the panel comment.
- The live `DbusVpn` path is compile-checked, not exercised in CI, same as the
  Wi-Fi source (ADR 0146).