// SPDX-License-Identifier: MIT
//! The battery and power-profiles adapter: battery presence, level, charging
//! state, health, and the performance/balanced/power-saver selection
//! (T-07.4, T-15.6a).
//!
//! Nothing above this crate sees zbus, a UPower type, or a
//! power-profiles-daemon type. Consumers read a [`PowerAdapter`], which holds
//! the last state the daemons pushed and exposes the three-state contract from
//! `dragonfruit-system-adapters` ([07-system-integration.md]): available,
//! hidden when **both** daemons are absent, or visible-and-inert on a read
//! error.
//!
//! # The read path
//!
//! 1. [`DbusUPower`] reads the **system bus** once per
//!    [`PowerAdapter::refresh`] and returns the raw [`PowerData`] (or
//!    absence/error).
//! 2. [`crate::model`] picks the present battery and maps its charge state,
//!    level, capacity, and cycles, and decodes the power-profiles half into the
//!    typed [`PowerSnapshot`].
//! 3. The adapter drives the shared subscription lifecycle, so a daemon
//!    restart re-subscribes and re-syncs with no user-visible error, and it
//!    diffs each read against the previous one into a [`PowerChange`] stream.
//!
//! # The one write
//!
//! The battery half is read-only; the power-profile half has one explicit
//! write, [`PowerAdapter::set_active_profile`], a single `Properties.Set` of
//! `ActiveProfile`. A successful write invents no snapshot: the daemon pushes
//! the resulting state and the host re-reads.
//!
//! # Presence
//!
//! UPower and power-profiles-daemon are **independent** daemons. UPower being
//! absent hides the battery item; a machine that runs UPower but has no present
//! battery (a desktop, a VM) still answers `Available`, with
//! [`PowerSnapshot::present`] false. power-profiles-daemon being absent is
//! carried inside the snapshot ([`PowerSnapshot::profiles`] is `None`) and only
//! disables the profile control. The adapter answers `Unavailable` only when
//! **neither** daemon is reachable. Nothing blocks session startup in any case.
//!
//! # Testing
//!
//! CI has no bus and no daemons, so the adapter is driven by [`MockPower`] over
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
pub use model::{
    Battery, BatteryHealth, BatteryLevel, ChargeState, PowerChange, PowerProfile,
    PowerProfilesSnapshot, PowerSnapshot,
};
pub use source::{
    MockPower, PowerData, PowerDeviceData, PowerProfileData, PowerProfilesData, PowerSource,
    ProfileOutcome, DEVICE_TYPE_BATTERY,
};
pub use upower::{
    DbusUPower, POWER_PROFILES_INTERFACE, POWER_PROFILES_PATH, POWER_PROFILES_SERVICE,
    UPOWER_DEVICE_INTERFACE, UPOWER_POWER_PROFILES_INTERFACE, UPOWER_POWER_PROFILES_PATH,
    UPOWER_POWER_PROFILES_SERVICE, UPOWER_SERVICE,
};
