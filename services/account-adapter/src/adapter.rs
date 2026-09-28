// SPDX-License-Identifier: MIT
//! The Users and Groups adapter: one read path over the shared contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the shell bridge calls it when
//! AccountsService or the group provider changes, so nothing above the adapter
//! polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "host stack absent" default, so a
//! session booted without AccountsService renders a hidden item and never
//! blocks.
//!
//! The account writes ([`create_user`](Self::create_user),
//! [`delete_user`](Self::delete_user),
//! [`set_account_type`](Self::set_account_type), [`set_locked`](Self::set_locked),
//! [`set_automatic_login`](Self::set_automatic_login)) and the group writes
//! ([`create_group`](Self::create_group), [`delete_group`](Self::delete_group),
//! [`set_group_members`](Self::set_group_members)) are explicit user actions
//! over the same seam. They invent no snapshot: the daemon (or provider)
//! publishes the resulting state and the host re-reads, so the snapshot stays
//! the single source of truth.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{AccountsChange, AccountsSnapshot};
use crate::source::{AccountOutcome, AccountSource, AccountType};

/// The Users and Groups adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountsAdapter<S> {
    source: S,
    state: AdapterState<AccountsSnapshot>,
    previous: Option<AccountsSnapshot>,
    changes: VecDeque<AccountsChange>,
    subscription: Subscription,
}

impl<S: AccountSource> AccountsAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        AccountsAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the host stack once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports an [`AccountsChange`] per field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = AccountsSnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot.clone());
                self.state = AdapterState::available(snapshot);
                self.subscription.changed();
            }
            Ok(None) => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
            }
            Err(error) => {
                self.subscription.subscribed();
                self.state = AdapterState::Error(error);
                self.subscription.changed();
            }
        }
    }

    /// Create a user account. One explicit write.
    pub fn create_user(
        &mut self,
        user_name: &str,
        real_name: &str,
        account_type: AccountType,
    ) -> AccountOutcome {
        self.source.create_user(user_name, real_name, account_type)
    }

    /// Delete the user account with `uid`. One explicit write.
    pub fn delete_user(&mut self, uid: u64) -> AccountOutcome {
        self.source.delete_user(uid)
    }

    /// Set the account type of the user with `uid`. One explicit write.
    pub fn set_account_type(&mut self, uid: u64, account_type: AccountType) -> AccountOutcome {
        self.source.set_account_type(uid, account_type)
    }

    /// Lock or unlock the user with `uid`. One explicit write.
    pub fn set_locked(&mut self, uid: u64, locked: bool) -> AccountOutcome {
        self.source.set_locked(uid, locked)
    }

    /// Set or clear automatic login for the user with `uid`. One explicit
    /// write.
    pub fn set_automatic_login(&mut self, uid: u64, automatic_login: bool) -> AccountOutcome {
        self.source.set_automatic_login(uid, automatic_login)
    }

    /// Create a group. One explicit write (the distro provider's).
    pub fn create_group(&mut self, name: &str) -> AccountOutcome {
        self.source.create_group(name)
    }

    /// Delete a group. One explicit write (the distro provider's).
    pub fn delete_group(&mut self, name: &str) -> AccountOutcome {
        self.source.delete_group(name)
    }

    /// Replace a group's membership. One explicit write (the distro
    /// provider's).
    pub fn set_group_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        self.source.set_group_members(name, members)
    }

    /// The live snapshot, when the host stack answered.
    pub fn snapshot(&self) -> Option<&AccountsSnapshot> {
        self.state.snapshot()
    }

    /// The transport this adapter reads.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably (for tests and lifecycle control).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed (a restart counts again).
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the domain changes pushed since the last drain, in order.
    pub fn drain_changes(&mut self) -> Vec<AccountsChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: AccountSource> Adapter for AccountsAdapter<S> {
    type Snapshot = AccountsSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::ACCOUNTS
    }

    fn state(&self) -> &AdapterState<Self::Snapshot> {
        &self.state
    }

    fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{AccountData, AccountsData, MockAccounts};

    fn data() -> AccountsData {
        AccountsData {
            users: vec![AccountData {
                uid: 1000,
                user_name: "dan".to_owned(),
                real_name: "Dan Doe".to_owned(),
                ..AccountData::default()
            }],
            groups: Some(vec![]),
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = AccountsAdapter::new(MockAccounts::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::ACCOUNTS).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = AccountsAdapter::new(MockAccounts::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().human_count(), 1);
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.source().reads(), 1);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
        // The first read is the baseline: no domain change.
        assert!(adapter.drain_changes().is_empty());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = AccountsAdapter::new(MockAccounts::present(data()));
        adapter.refresh();
        let _ = adapter.drain_events();

        adapter.source_mut().kill();
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

        adapter.source_mut().restart();
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.subscriptions(), 2);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: true },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = AccountsAdapter::new(MockAccounts::failing("AccountsService: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "AccountsService: timeout"
        );
    }

    #[test]
    fn a_user_move_is_a_domain_event() {
        let mut adapter = AccountsAdapter::new(MockAccounts::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.users.push(AccountData {
            uid: 1001,
            user_name: "kim".to_owned(),
            ..AccountData::default()
        });
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![AccountsChange::UserAdded {
                user_name: "kim".to_owned()
            }]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn the_writes_delegate_to_the_source() {
        let mut adapter = AccountsAdapter::new(MockAccounts::present(data()));
        adapter.refresh();

        assert!(adapter
            .create_user("kim", "Kim", AccountType::Standard)
            .is_applied());
        assert!(adapter
            .set_account_type(1000, AccountType::Administrator)
            .is_applied());
        assert!(adapter.set_locked(1000, true).is_applied());
        assert!(adapter.set_automatic_login(1000, true).is_applied());
        assert!(adapter.create_group("devs").is_applied());
        assert!(adapter
            .set_group_members("devs", &["dan".to_owned()])
            .is_applied());
        assert!(adapter.delete_group("devs").is_applied());
        assert!(adapter.delete_user(1000).is_applied());

        assert_eq!(adapter.source().creates(), 1);
        assert_eq!(adapter.source().type_sets(), 1);
        assert_eq!(adapter.source().lock_sets(), 1);
        assert_eq!(adapter.source().auto_login_sets(), 1);
        assert_eq!(adapter.source().group_creates(), 1);
        assert_eq!(adapter.source().group_member_sets(), 1);
        assert_eq!(adapter.source().group_deletes(), 1);
        assert_eq!(adapter.source().deletes(), 1);
        assert_eq!(adapter.source().writes(), 8);
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut adapter = AccountsAdapter::new(MockAccounts::absent());
        adapter.refresh();
        assert_eq!(
            adapter.create_user("kim", "Kim", AccountType::Standard),
            AccountOutcome::Absent
        );
        // A write never invents state: the adapter stays absent until a read.
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::ACCOUNTS).visible);
    }

    #[test]
    fn a_denial_is_reported_and_leaves_the_read_state_live() {
        let mut adapter =
            AccountsAdapter::new(MockAccounts::present(data()).deny_writes("polkit: denied"));
        adapter.refresh();
        let outcome = adapter.create_user("kim", "Kim", AccountType::Standard);
        assert_eq!(outcome.denial_note(), Some("polkit: denied"));
        assert!(adapter.state().is_available());
        assert!(adapter.state().slot(AdapterId::ACCOUNTS).visible);
    }

    #[test]
    fn a_group_without_a_provider_is_absent_but_the_users_stay_live() {
        let mut adapter = AccountsAdapter::new(MockAccounts::present(AccountsData {
            groups: None,
            ..data()
        }));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().groups_available());
        assert_eq!(adapter.create_group("devs"), AccountOutcome::Absent);
        // The absence is layered: the adapter does not hide.
        assert!(adapter.state().is_available());
        assert_eq!(
            adapter.source_mut().create_group("wheel"),
            AccountOutcome::Absent
        );
    }
}
