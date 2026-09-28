// SPDX-License-Identifier: MIT
//! The NetworkManager adapter: read Wi-Fi state, the access-point list, and
//! signal strength (T-07.2a), and join a network with polkit degradation
//! (T-07.2b).
//!
//! The menu bar's Wi-Fi item does not talk to NetworkManager. It reads a
//! [`NetworkManagerAdapter`], which holds the last state the daemon pushed and
//! exposes the three-state contract from `dragonfruit-system-adapters`
//! ([07-system-integration.md]): available, hidden when the daemon is absent,
//! or visible-and-inert on a read error. Nothing above this crate sees zbus or
//! an `NM*` type.
//!
//! # The read path
//!
//! 1. [`DbusNetworkManager`] reads the system bus once per
//!    [`NetworkManagerAdapter::refresh`] and returns a raw
//!    [`NetworkManagerData`] (or absence/error).
//! 2. [`crate::model`] decodes that into the typed [`WifiSnapshot`]: one
//!    aggregate [`WifiState`], the [`Connectivity`], and one [`AccessPoint`]
//!    per SSID, strongest first.
//! 3. The adapter drives the shared subscription lifecycle, so a NetworkManager
//!    restart re-subscribes and re-syncs with no user-visible error.
//!
//! # The write path
//!
//! [`NetworkManagerAdapter::join`] activates a network through the same source
//! seam. NetworkManager joins are authorized by polkit; when polkit refuses,
//! the join comes back as [`JoinResult::Denied`] and the adapter records a
//! [`WifiAccess::ReadOnly`] degradation (see
//! [adr/0027](../../../docs/design/adr/0027-networkmanager-join-read-only-degradation.md)).
//! The network list stays live — only the join affordance is disabled — and the
//! adapter refuses further joins locally rather than asking the daemon again.
//!
//! Absence is a normal state: a session booted without NetworkManager renders
//! a hidden Wi-Fi item and is otherwise unaffected.
//!
//! # The VPN path (T-15.15a)
//!
//! The same daemon is also the host stack for **Network advanced / VPN**
//! ([`VpnAdapter`]): NetworkManager already owns every `vpn` and `wireguard`
//! connection, so the adapter projects its settings objects and active
//! connections ([`DbusVpn`]) rather than reimplementing a VPN stack. It is a
//! second adapter behind its own [`VpnSource`] seam with its own
//! `AdapterId::VPN` slot, mirroring the Wi-Fi shape: three-state reads,
//! subscription events, a configurable-but-empty `present` distinction, and a
//! polkit read-only degradation on its two writes ([`VpnAdapter::connect`] /
//! [`VpnAdapter::deactivate`]). The Settings pane and Control Center tile land
//! with T-15.15b.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so each adapter is driven by a mock over a
//! fixture ([`NetworkManagerSource`] / [`VpnSource`] are the seams). The live
//! D-Bus sources are thin mechanical layers over those seams; [`MockVpn`]
//! mutates its simulated store on an accepted write so a connect/disconnect
//! round-trip is observable headlessly.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod dbus;
mod model;
mod source;
mod vpn;

pub use adapter::{JoinRequest, JoinResult, NetworkManagerAdapter, WifiAccess};
pub use dbus::{DbusNetworkManager, DbusVpn, NM_SERVICE, VPN_SETTINGS_PATH};
pub use model::{AccessPoint, Band, Connectivity, Security, WifiSnapshot, WifiState};
pub use source::{
    AccessPointData, ActivateOutcome, ActivateRequest, MockNetworkManager, NetworkManagerData,
    NetworkManagerSource, WifiDeviceData,
};
pub use vpn::{
    ActiveVpnData, MockVpn, VpnAccess, VpnAdapter, VpnConnection, VpnConnectionData, VpnData,
    VpnKind, VpnOutcome, VpnRequest, VpnResult, VpnSnapshot, VpnSource, VpnState,
};
