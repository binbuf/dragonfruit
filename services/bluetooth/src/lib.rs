// SPDX-License-Identifier: MIT
//! The BlueZ Bluetooth adapter: controller and device state, discovery,
//! pairing, and connection (T-15.1a).
//!
//! The Settings Bluetooth pane and the Control Center tile do not talk to
//! BlueZ. They read a [`BluetoothAdapter`], which holds the last state the
//! daemon pushed and exposes the three-state contract from
//! `dragonfruit-system-adapters` ([07-system-integration.md]): available,
//! hidden when `bluetoothd` is absent, or visible-and-inert on a read error.
//! Nothing above this crate sees zbus or a BlueZ type.
//!
//! # The read path
//!
//! 1. [`DbusBluez`] reads the **system bus** once per
//!    [`BluetoothAdapter::refresh`] — `GetManagedObjects` at `/` — and returns
//!    the raw [`BluetoothData`] (or absence/error).
//! 2. [`crate::model`] picks the controller, keeps its devices, and derives
//!    the typed [`BluetoothSnapshot`] the UI draws.
//! 3. The adapter drives the shared subscription lifecycle, so a `bluetoothd`
//!    restart re-subscribes and re-syncs with no user-visible error.
//!
//! # The write path
//!
//! The adapter's explicit actions are [`set_powered`](BluetoothAdapter::set_powered),
//! [`set_discovering`](BluetoothAdapter::set_discovering),
//! [`pair`](BluetoothAdapter::pair), and
//! [`set_connected`](BluetoothAdapter::set_connected). Each is one call, never
//! a loop; a successful call invents no snapshot, because BlueZ pushes the
//! resulting `PropertiesChanged` and the host re-reads. A polkit refusal comes
//! back as [`BluetoothOutcome::Denied`] and leaves the read state live.
//!
//! # Absence
//!
//! `bluetoothd` being absent is the adapter's `Unavailable` state and hides
//! the item. A running daemon with no controller (no hardware, radio removed)
//! still answers `Available`, with [`BluetoothSnapshot::present`] false; a
//! consumer hides the item then too. Neither blocks session startup.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so the adapter is driven by [`MockBluetooth`]
//! over a fixture ([`BluetoothSource`] is the seam). The live D-Bus source is
//! a thin mechanical layer over that seam and is smoke-tested where a session
//! is present.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod bluez;
mod model;
mod source;

pub use adapter::BluetoothAdapter;
pub use bluez::{
    DbusBluez, ADAPTER_INTERFACE, BLUEZ_SERVICE, DEVICE_INTERFACE, OBJECT_MANAGER_INTERFACE,
    PROPERTIES_INTERFACE,
};
pub use model::{BluetoothAdapter as BluetoothController, BluetoothDevice, BluetoothSnapshot};
pub use source::{
    BluetoothAdapterData, BluetoothData, BluetoothDeviceData, BluetoothOutcome, BluetoothSource,
    MockBluetooth,
};
