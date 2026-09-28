// SPDX-License-Identifier: MIT
//! The Privacy and Security adapter (T-15.13a).
//!
//! Privacy and Security does not wrap one daemon per row the way Bluetooth or
//! UPower do: it projects the **host stack**, the record `xdg-desktop-portal`
//! already keeps of which applications may reach the resources portals mediate.
//! That record is the standard
//! `org.freedesktop.impl.portal.PermissionStore`
//! ([`HostPrivacy`]): a free-form set of tables, each with resource ids, each
//! with a map from application id to permission strings. The adapter reuses it
//! and never reimplements a portal or makes a permission decision; the store is
//! the single source of truth.
//!
//! # The read path
//!
//! 1. A [`PrivacySource`] returns the raw [`PrivacyData`] once per
//!    [`PrivacyAdapter::refresh`] (or absence/error).
//! 2. [`PrivacySnapshot::from_data`] types every application permission,
//!    orders the categories and rows, and derives the labels and glyph the pane
//!    draws.
//! 3. The adapter drives the shared subscription lifecycle, so a store that
//!    comes and goes re-subscribes and re-syncs with no user-visible error.
//!
//! # Absence is normal
//!
//! The adapter is `Unavailable` only when the session bus is unreachable or no
//! `org.freedesktop.impl.portal.PermissionStore` owns its name — a normal
//! hidden state, never an error. A store that answers with no permission is
//! `Available` with an empty snapshot; the pane's hide rule is
//! [`PrivacySnapshot::present`]. A store that owns its name but cannot be read
//! is `Error`, visible and inert with the message. Nothing blocks session
//! startup.
//!
//! # Explicit writes
//!
//! Granting or revoking an application's access is one explicit user action over
//! the same seam ([`PrivacyAdapter::set_permission`],
//! [`PrivacyAdapter::delete_permission`]). It invents no snapshot: the store
//! publishes the resulting state and the host re-reads, so the snapshot stays
//! the single source of truth. The durable preferences `settingsd` owns land
//! with T-15.13b.
//!
//! # Testing
//!
//! CI has no `xdg-desktop-portal`, so the adapter is driven by [`MockPrivacy`]
//! over the source seam. `kill`/`restart` exercise absence and re-subscribe;
//! `push` drives the tables. [`HostPrivacy`] reads the live store,
//! [`apps_from_lookup`] decodes a `Lookup` map, and the table vocabulary is
//! pinned by [`KNOWN_TABLES`].
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md
//! [08-settings.md]: ../../../docs/design/08-settings.md

mod adapter;
mod model;
mod privacy;
mod source;

pub use adapter::PrivacyAdapter;
pub use model::{
    table_label, table_rank, AppPermission, KnownTable, PermissionCategory, PermissionResource,
    PermissionState, PrivacyChange, PrivacySnapshot, KNOWN_TABLES,
};
pub use privacy::{
    apps_from_lookup, HostPrivacy, PERMISSION_STORE_INTERFACE, PERMISSION_STORE_PATH,
    PERMISSION_STORE_SERVICE,
};
pub use source::{
    AppPermissionData, MockPrivacy, PrivacyData, PrivacyOutcome, PrivacySource, ResourceData,
    TableData,
};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_privacy_slot() {
        let adapter = PrivacyAdapter::new(MockPrivacy::absent());
        assert_eq!(
            <PrivacyAdapter<MockPrivacy> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::PRIVACY
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_vocabulary_ids_are_stable() {
        assert_eq!(PermissionState::Allowed.id(), "allowed");
        assert_eq!(PermissionState::Denied.id(), "denied");
        assert_eq!(PermissionState::Ask.id(), "ask");
        assert_eq!(PermissionState::Unset.id(), "unset");
        assert_eq!(KNOWN_TABLES[0].table, "devices");
        assert_eq!(KNOWN_TABLES[0].label, "Camera");
    }
}
