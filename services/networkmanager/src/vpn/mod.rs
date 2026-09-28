// SPDX-License-Identifier: MIT
//! The Network advanced (VPN) adapter (T-15.15a).
//!
//! A VPN is not a second daemon: NetworkManager already owns every connection
//! — including `vpn` and `wireguard` — and exposes its settings and its active
//! connections over the **system-bus D-Bus API**. This adapter is the
//! projection of those connections ([`VpnAdapter`]); it reuses the same
//! `zbus` client and the same transport discipline as the Wi-Fi adapter
//! ([`crate::NetworkManagerAdapter`]) and reimplements no VPN stack.
//!
//! # The read path
//!
//! 1. A [`VpnSource`] returns the raw [`VpnData`] once per
//!    [`VpnAdapter::refresh`] (or absence/error).
//! 2. [`VpnSnapshot::from_data`] joins the settings connections to the active
//!    connections, types each [`VpnKind`] and [`VpnState`], and derives the
//!    labels the pane and the Control Center tile draw.
//! 3. The adapter drives the shared subscription lifecycle, so a
//!    NetworkManager restart re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Absence is normal
//!
//! The adapter is `Unavailable` only when the system bus is unreachable or no
//! `org.freedesktop.NetworkManager` owns its name — a normal hidden state,
//! never an error. A daemon that answers with no VPN connection is `Available`
//! with [`VpnSnapshot::present`] false; a consumer that shows only an active
//! status hides the item then. A daemon that owns its name but cannot be read
//! is `Error`, visible and inert with the message. Nothing blocks session
//! startup.
//!
//! # The write path
//!
//! [`VpnAdapter::connect`] and [`VpnAdapter::deactivate`] are the two explicit
//! user actions, over the same source seam. NetworkManager VPN activations are
//! authorized by polkit; when polkit refuses, the write comes back as
//! [`VpnOutcome::Denied`] and the adapter records a [`VpnAccess::ReadOnly`]
//! degradation (see [`crate::WifiAccess`] for the sibling Wi-Fi behavior). A
//! write never invents a snapshot: the daemon pushes the resulting state and
//! the host re-reads, so the snapshot stays the single source of truth.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so the adapter is driven by [`MockVpn`] over a
//! fixture ([`VpnSource`] is the seam). The mock models a connection store
//! that mutates on an accepted write, so a connect/disconnect round-trip is
//! observable headlessly. [`crate::DbusVpn`] is the live source and is
//! compile-checked, not exercised in CI.

pub(crate) mod adapter;
pub(crate) mod model;
pub(crate) mod source;

pub use adapter::{VpnAccess, VpnAdapter, VpnResult};
pub use model::{VpnConnection, VpnKind, VpnSnapshot, VpnState};
pub use source::{
    ActiveVpnData, MockVpn, VpnConnectionData, VpnData, VpnOutcome, VpnRequest, VpnSource,
};
