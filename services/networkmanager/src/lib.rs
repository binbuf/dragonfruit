// SPDX-License-Identifier: MIT
//! The NetworkManager adapter: read Wi-Fi state, the access-point list, and
//! signal strength (T-07.2a).
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
//! Absence is a normal state: a session booted without NetworkManager renders
//! a hidden Wi-Fi item and is otherwise unaffected. The write path (join and
//! polkit degradation) is T-07.2b.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so the adapter is driven by
//! [`MockNetworkManager`] over a fixture ([`NetworkManagerSource`] is the
//! seam). The live D-Bus source is a thin mechanical layer over that seam.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod dbus;
mod model;
mod source;

pub use adapter::NetworkManagerAdapter;
pub use dbus::{DbusNetworkManager, NM_SERVICE};
pub use model::{AccessPoint, Band, Connectivity, Security, WifiSnapshot, WifiState};
pub use source::{
    AccessPointData, MockNetworkManager, NetworkManagerData, NetworkManagerSource, WifiDeviceData,
};
