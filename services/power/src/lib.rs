// SPDX-License-Identifier: MIT
//! The UPower power adapter: battery presence, level, and charging state
//! (T-07.4).
//!
//! The menu bar's battery item does not talk to UPower. It reads a
//! [`PowerAdapter`], which holds the last state the daemon pushed and exposes
//! the three-state contract from `dragonfruit-system-adapters`
//! ([07-system-integration.md]): available, hidden when the daemon is absent,
//! or visible-and-inert on a read error. Nothing above this crate sees zbus or
//! a UPower type.
//!
//! # The read path
//!
//! 1. [`DbusUPower`] reads the **system bus** once per
//!    [`PowerAdapter::refresh`] and returns the raw [`PowerData`] (or
//!    absence/error).
//! 2. [`crate::model`] picks the present battery and maps its charge state and
//!    level into the typed [`PowerSnapshot`].
//! 3. The adapter drives the shared subscription lifecycle, so a UPower
//!    restart re-subscribes and re-syncs with no user-visible error.
//!
//! # Read-only
//!
//! The battery item is read-only; there is no write half. Power profiles
//! (`power-profiles-daemon`) are explicitly deferred to T-15.
//!
//! # Presence
//!
//! UPower being absent is the adapter's `Unavailable` state and hides the
//! item. A machine with UPower but no present battery (a desktop, a VM) still
//! answers `Available`, with [`PowerSnapshot::present`] false; a consumer hides
//! the item then too.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so the adapter is driven by [`MockPower`] over
//! a fixture ([`PowerSource`] is the seam). The live D-Bus source is a thin
//! mechanical layer over that seam and is smoke-tested where a session is
//! present.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod model;
mod source;
mod upower;

pub use adapter::PowerAdapter;
pub use model::{Battery, BatteryLevel, ChargeState, PowerSnapshot};
pub use source::{MockPower, PowerData, PowerDeviceData, PowerSource, DEVICE_TYPE_BATTERY};
pub use upower::{DbusUPower, UPOWER_SERVICE};
