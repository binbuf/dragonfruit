# 0146 — The Network advanced (VPN) adapter projects NetworkManager connections

- **Status:** Accepted (T-15.15a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [08-settings.md](../08-settings.md),
  [15-system-services-breadth.md](../tracks/15-system-services-breadth.md),
  ADR [0027](0027-networkmanager-join-read-only-degradation.md)
- **Supersedes:** —

## Context

T-15.15 splits the macOS Network surface into an adapter (T-15.15a) and a
Settings pane plus Control Center tile (T-15.15b). The task is scoped to
**Network advanced / VPN**: the VPN connections the reference pane nests behind
its service rows. [07-system-integration.md] routes networking to
**NetworkManager** — "device/access-point enumeration; activate, deactivate,
create, edit, delete connections" — and T-07.2a/T-07.2b already own the Wi-Fi
read and join over NetworkManager's D-Bus API in
`dragonfruit-networkmanager`. A VPN is not a second daemon: NetworkManager
already owns every `vpn` and `wireguard` connection, its settings objects, and
its active connections.

## Decision

- **The adapter grows the existing NetworkManager crate, not a new one.** VPN
  connections come from the same daemon the Wi-Fi adapter already reads, so
  `services/networkmanager/src/vpn/` is a second adapter behind its own
  `VpnSource` seam. `AdapterId::VPN` (id `vpn`) joins the shared ids. The live
  source, `DbusVpn`, reuses the crate's `zbus` client, proxies, and absence
  discipline; it reimplements no VPN stack, exactly as the audio adapter grew
  input routing and the power adapter grew profiles in their own crates.
- **Two reads are joined into one snapshot.** One `DbusVpn::read` enumerates
  the settings connections (`Settings.Connection.GetSettings`, keeping the
  `vpn`/`wireguard` types) and the live active connections (`Connection.Active`
  properties), then `VpnSnapshot::from_data` joins them by settings path (with
  a UUID fallback) and types each `VpnKind` (OpenVPN, OpenConnect, IPsec,
  PPTP, L2TP, WireGuard, generic VPN) and `VpnState`
  (`Connecting`/`Connected`/`Disconnecting`/`Disconnected`). Connected
  connections sort first.
- **Absence is normal and layered.** `Unavailable` only when the system bus is
  unreachable or no `org.freedesktop.NetworkManager` owns its name; a daemon
  that answers with no VPN configured is `Available` with
  `VpnSnapshot::present()` false, which is the tile/pane's second hide rule. A
  daemon that owns its name but cannot be read is `Error`, visible and inert.
- **Two explicit writes with the Wi-Fi read-only degradation.** `connect` and
  `deactivate` are one `ActivateConnection`/`DeactivateConnection` call each,
  by UUID. NetworkManager authorizes them with polkit; a refusal is reported
  separately from a failure (`VpnOutcome::Denied`) and the adapter records a
  `VpnAccess::ReadOnly { note }` degradation, mirroring the Wi-Fi join
  (ADR 0027). Reads stay live and only the write affordances disable. A write
  never invents a snapshot: the daemon pushes the resulting state and the host
  re-reads.

Rejected: a new `dragonfruit-vpn-adapter` crate (the host stack and its
`zbus` client already live in `dragonfruit-networkmanager`); linking `libnm`
([07-system-integration.md] principle 1 — the D-Bus API is the seam); treating
a configured-but-empty store as absence (empty is a value, not a missing
daemon); inventing VPN providers NetworkManager does not own.

## Consequences

- `dragonfruit-networkmanager` carries the VPN adapter alongside Wi-Fi. CI
  drives it with `MockVpn` (which mutates its simulated store on an accepted
  write, so a connect/disconnect round-trip is observable headlessly); no bus,
  daemon, or hardware is involved. `make e2e` already runs
  `cargo test -p dragonfruit-networkmanager`.
- `DbusVpn` is the live `VpnSource`; the fixture shape is pinned by
  `tests/fixtures/nm-vpn.json` and `vpn_connection_from_settings` is unit-tested
  against constructed settings maps.
- T-15.15b adds the Network pane and Control Center tile. It projects this
  adapter through the bridge host like the other T-15 adapters and must not add
  a second VPN implementation. The pane's Overlay/VPN rows and the tile's
  write controls map to `VpnAdapter::connect`/`deactivate`; the bridge seam is
  T-15.15b's to add.