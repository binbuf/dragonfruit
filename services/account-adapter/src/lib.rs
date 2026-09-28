// SPDX-License-Identifier: MIT
//! The Users and Groups adapter (T-15.11a).
//!
//! Users and Groups do not wrap a single desktop daemon the way Bluetooth or
//! UPower do: they project the **host stack**, which has two halves, both
//! reused, never reimplemented:
//!
//! * the **users** half — the cached user list AccountsService
//!   (`org.freedesktop.Accounts`) serves over its system-bus D-Bus API
//!   ([`HostAccounts`]); and
//! * the **groups** half — the distribution provider behind the
//!   [`GroupProvider`] seam, because AccountsService has no group API. The
//!   concrete provider is distro-specific and belongs with packaging; this
//!   crate ships the seam and the mock.
//!
//! # The read path
//!
//! 1. An [`AccountSource`] returns the raw [`AccountsData`] once per
//!    [`AccountsAdapter::refresh`] (or absence/error).
//! 2. [`AccountsSnapshot::from_data`] types every user and group, orders the
//!    lists, and derives the labels and glyph the pane draws.
//! 3. The adapter drives the shared subscription lifecycle, so a host stack
//!    that comes and goes re-subscribes and re-syncs with no user-visible
//!    error.
//!
//! # Absence is layered and normal
//!
//! The whole adapter is `Unavailable` only when neither AccountsService nor the
//! group provider is reachable. AccountsService that answers but caches no user
//! is `Available` with an empty snapshot; a running host with **no group
//! provider** is `Available` with `groups: None`, so only the group controls
//! disable and the user list stays live. That mirrors the battery and update
//! adapters' per-half hide rules. A present host stack that cannot be read is
//! `Error`, visible and inert with the message. Nothing blocks session startup.
//!
//! # Explicit writes
//!
//! The pane's create/delete/lock/account-type/automatic-login and its group
//! writes are explicit user actions over the same seam
//! ([`AccountsAdapter::create_user`] and friends). They invent no snapshot: the
//! daemon or provider publishes the resulting state and the host re-reads, so
//! the snapshot stays the single source of truth. Durable presentation
//! preferences are `settingsd`'s and land with T-15.11b.
//!
//! # Testing
//!
//! CI has no AccountsService and no group provider, so the adapter is driven by
//! [`MockAccounts`] over the source seam. `kill`/`restart` exercise absence and
//! re-subscribe; `push` drives the user and group lists. [`HostAccounts`] reads
//! the live daemon, [`account_from_props`] decodes a property map in tests, and
//! [`MockGroupProvider`] proves the provider seam.
//!
//! [08-settings.md]: ../../../docs/design/08-settings.md
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod accounts;
mod adapter;
mod model;
mod source;

pub use accounts::{
    account_from_props, user_path, GroupProvider, HostAccounts, MockGroupProvider,
    ACCOUNTS_INTERFACE, ACCOUNTS_ROOT, ACCOUNTS_SERVICE, PROPERTIES_INTERFACE, USER_INTERFACE,
};
pub use adapter::AccountsAdapter;
pub use model::{Account, AccountsChange, AccountsSnapshot, Group};
pub use source::{
    AccountData, AccountOutcome, AccountSource, AccountType, AccountsData, GroupData, MockAccounts,
    PasswordMode,
};

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_accounts_slot() {
        let adapter = AccountsAdapter::new(MockAccounts::absent());
        assert_eq!(
            <AccountsAdapter<MockAccounts> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::ACCOUNTS
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = AccountsAdapter::new(MockAccounts::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_vocabulary_ids_are_stable() {
        assert_eq!(AccountType::Administrator.id(), "administrator");
        assert_eq!(AccountType::Standard.id(), "standard");
        assert_eq!(PasswordMode::SetAtLogin.id(), "set-at-login");
        assert_eq!(PasswordMode::Empty.id(), "empty");
    }
}
