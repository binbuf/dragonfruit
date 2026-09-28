// SPDX-License-Identifier: MIT
//! The UDisks2 storage and removable-media adapter: drives, volumes, mount
//! state, and explicit mount/unmount/eject (T-15.2a).
//!
//! The Settings Storage pane and the Control Center tile do not talk to
//! UDisks2. They read a [`StorageAdapter`], which holds the last state the
//! daemon pushed and exposes the three-state contract from
//! `dragonfruit-system-adapters` ([07-system-integration.md]): available,
//! hidden when UDisks2 is absent, or visible-and-inert on a read error.
//! Nothing above this crate sees zbus or a UDisks2 type.
//!
//! # The read path
//!
//! 1. [`DbusUDisks`] reads the **system bus** once per
//!    [`StorageAdapter::refresh`] — `GetManagedObjects` at
//!    `/org/freedesktop/UDisks2` — and returns the raw [`StorageData`] (or
//!    absence/error).
//! 2. [`crate::model`] joins each block object to its drive and derives the
//!    typed [`StorageSnapshot`] the UI draws.
//! 3. The adapter drives the shared subscription lifecycle, so a UDisks2
//!    restart re-subscribes and re-syncs with no user-visible error.
//!
//! # The write path
//!
//! The adapter's explicit actions are [`mount`](StorageAdapter::mount),
//! [`unmount`](StorageAdapter::unmount), and
//! [`eject`](StorageAdapter::eject). Each is one call, never a loop; a
//! successful call invents no snapshot, because UDisks2 pushes the resulting
//! `PropertiesChanged`/`InterfacesAdded` and the host re-reads. A polkit
//! refusal comes back as [`StorageOutcome::Denied`] and leaves the read state
//! live.
//!
//! # Absence
//!
//! UDisks2 being absent is the adapter's `Unavailable` state and hides the
//! item. A running daemon with no mountable volumes still answers `Available`,
//! with [`StorageSnapshot::present`] false; a consumer hides the item then too.
//! Neither blocks session startup.
//!
//! # Testing
//!
//! CI has no bus and no daemon, so the adapter is driven by [`MockStorage`]
//! over a fixture ([`StorageSource`] is the seam). The live D-Bus source is a
//! thin mechanical layer over that seam and is smoke-tested where a session is
//! present.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod model;
mod source;
mod udisks;

pub use adapter::StorageAdapter;
pub use model::{StorageDrive, StorageSnapshot, StorageVolume};
pub use source::{
    MockStorage, StorageData, StorageDriveData, StorageOutcome, StorageSource, StorageVolumeData,
};
pub use udisks::{
    DbusUDisks, BLOCK_INTERFACE, DRIVE_INTERFACE, FILESYSTEM_INTERFACE, OBJECT_MANAGER_INTERFACE,
    UDISKS_ROOT, UDISKS_SERVICE,
};
