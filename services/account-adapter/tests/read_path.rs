// SPDX-License-Identifier: MIT
//! T-15.11a acceptance: the Users and Groups adapter reports state and events
//! against a mock, and an absent host stack is a normal hidden state. No
//! AccountsService, group provider, or hardware is involved.

use dragonfruit_account_adapter::{
    AccountData, AccountOutcome, AccountSource, AccountType, AccountsAdapter, AccountsData,
    AccountsSnapshot, HostAccounts, MockAccounts, PasswordMode,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live AccountsService source
/// builds from `ListCachedUsers` plus each user's properties.
fn fixture() -> AccountsData {
    serde_json::from_str(include_str!("fixtures/accounts-workstation.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_users_and_groups() {
    let mut adapter = AccountsAdapter::new(MockAccounts::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("AccountsService answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.glyph(), "users");
    assert_eq!(snapshot.label(), "2 Users");

    // Human users first (dan 1000, sam 1001), then the system account.
    assert_eq!(snapshot.users().len(), 3);
    assert_eq!(snapshot.human_users()[0].user_name, "dan");
    assert_eq!(snapshot.human_users()[1].user_name, "sam");
    assert_eq!(snapshot.system_users()[0].user_name, "root");
    assert_eq!(snapshot.admin_count(), 1);
    assert_eq!(snapshot.locked_count(), 1);

    let dan = snapshot.user_by_uid(1000).unwrap();
    assert_eq!(dan.display_name(), "Dan Doe");
    assert_eq!(dan.initial(), "D");
    assert!(dan.is_admin());
    assert!(dan.has_avatar());
    assert_eq!(dan.password_mode, PasswordMode::Regular);
    assert_eq!(dan.login_time, 1_700_000_000);
    assert_eq!(
        snapshot
            .automatic_login_user()
            .map(|user| user.user_name.as_str()),
        Some("dan")
    );

    // The group list is live and ordered user groups first.
    assert!(snapshot.groups_available());
    assert_eq!(snapshot.groups().len(), 2);
    assert_eq!(snapshot.groups()[0].name, "wheel");
    assert!(!snapshot.groups()[0].is_system());
    assert!(snapshot.groups()[0].contains("dan"));
    assert_eq!(snapshot.groups()[1].name, "users");
    assert!(snapshot.groups()[1].is_system());

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::ACCOUNTS);
    assert_eq!(slot.id, AdapterId::ACCOUNTS);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::ACCOUNTS);
}

#[test]
fn the_lifecycle_resubscribes_after_the_host_stack_returns() {
    let mut adapter = AccountsAdapter::new(MockAccounts::present(fixture()));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // AccountsService goes away: the item hides, never an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::ACCOUNTS).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // It comes back: a re-subscribe and a re-sync, no user-visible error.
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
fn a_user_move_is_an_observable_event() {
    let mut adapter = AccountsAdapter::new(MockAccounts::present(fixture()));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    // A new standard user is added.
    let mut next = fixture();
    next.users.push(AccountData {
        uid: 1002,
        user_name: "kim".to_owned(),
        real_name: "Kim Lee".to_owned(),
        ..AccountData::default()
    });
    adapter.source_mut().push(next);
    adapter.refresh();
    assert_eq!(
        adapter.drain_changes(),
        vec![dragonfruit_account_adapter::AccountsChange::UserAdded {
            user_name: "kim".to_owned()
        }]
    );
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);

    // A user is locked, then deleted.
    let mut next = fixture();
    next.users[2].locked = false;
    adapter.source_mut().push(next);
    adapter.refresh();
    assert!(adapter.drain_changes().contains(
        &dragonfruit_account_adapter::AccountsChange::UserChanged {
            user_name: "sam".to_owned()
        }
    ));

    let mut next = fixture();
    next.users.retain(|user| user.uid != 1000);
    adapter.source_mut().push(next);
    adapter.refresh();
    let changes = adapter.drain_changes();
    assert!(
        changes.contains(&dragonfruit_account_adapter::AccountsChange::UserRemoved {
            user_name: "dan".to_owned()
        })
    );
    assert!(changes.contains(
        &dragonfruit_account_adapter::AccountsChange::AutomaticLoginChanged {
            from: Some("dan".to_owned()),
            to: None,
        }
    ));
}

#[test]
fn a_missing_group_provider_is_a_normal_available_state() {
    // AccountsService answers but there is no distro group provider: the user
    // list stays live and only the group controls are absent.
    let mut adapter = AccountsAdapter::new(MockAccounts::present(AccountsData {
        groups: None,
        ..fixture()
    }));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert_eq!(snapshot.human_count(), 2);
    assert!(!snapshot.groups_available());
    assert!(snapshot.groups().is_empty());

    // A group write while the provider is absent is absent, never an error,
    // and does not hide the users.
    assert_eq!(adapter.create_group("devs"), AccountOutcome::Absent);
    assert_eq!(adapter.delete_group("wheel"), AccountOutcome::Absent);
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::ACCOUNTS).visible);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No host stack at all: hidden, absent, no error, no panic.
    let mut adapter = AccountsAdapter::new(MockAccounts::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
    assert_eq!(
        adapter.create_user("kim", "Kim", AccountType::Standard),
        AccountOutcome::Absent
    );

    // The stack is present but unreadable: visible, inert, with the message.
    let mut failing = AccountsAdapter::new(MockAccounts::failing("AccountsService: timeout"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "AccountsService: timeout"
    );
}

#[test]
fn the_writes_are_explicit_and_do_not_invent_a_snapshot() {
    let mut adapter = AccountsAdapter::new(MockAccounts::present(fixture()));
    adapter.refresh();
    let before = adapter.snapshot().unwrap().clone();
    let _ = adapter.drain_events();
    let _ = adapter.drain_changes();

    assert!(adapter
        .create_user("kim", "Kim Lee", AccountType::Standard)
        .is_applied());
    assert!(adapter
        .set_account_type(1001, AccountType::Administrator)
        .is_applied());
    assert!(adapter.set_locked(1001, false).is_applied());
    assert!(adapter.set_automatic_login(1001, true).is_applied());
    assert!(adapter.create_group("devs").is_applied());
    assert!(adapter
        .set_group_members("devs", &["dan".to_owned(), "kim".to_owned()])
        .is_applied());
    assert!(adapter.delete_group("devs").is_applied());
    assert!(adapter.delete_user(1001).is_applied());

    assert_eq!(adapter.source().creates(), 1);
    assert_eq!(adapter.source().type_sets(), 1);
    assert_eq!(adapter.source().lock_sets(), 1);
    assert_eq!(adapter.source().auto_login_sets(), 1);
    assert_eq!(adapter.source().group_creates(), 1);
    assert_eq!(adapter.source().group_member_sets(), 1);
    assert_eq!(adapter.source().group_deletes(), 1);
    assert_eq!(adapter.source().deletes(), 1);
    assert_eq!(adapter.source().writes(), 8);

    // The adapter state is unchanged until the host re-reads; a write never
    // invents a snapshot.
    assert_eq!(adapter.snapshot().unwrap(), &before);
    assert!(adapter.drain_events().is_empty());

    // The daemon (mock) published the result; the host re-reads.
    adapter.refresh();
    let after = adapter.snapshot().unwrap();
    assert!(after.user_by_name("kim").is_some());
    assert!(after.user_by_name("sam").is_none());
    // The automatic-login user moved off dan and then its account was deleted.
    assert!(after.automatic_login_user().is_none());
}

#[test]
fn a_denied_write_is_reported_and_the_read_state_stays_live() {
    let mut adapter = AccountsAdapter::new(
        MockAccounts::present(fixture()).deny_writes("polkit: not authorized"),
    );
    adapter.refresh();

    let outcome = adapter.delete_user(1000);
    assert_eq!(
        outcome,
        AccountOutcome::Denied("polkit: not authorized".to_owned())
    );
    assert_eq!(outcome.denial_note(), Some("polkit: not authorized"));
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::ACCOUNTS).visible);
}

#[test]
fn a_failed_write_is_not_a_denial_and_leaves_the_read_state_live() {
    let mut adapter =
        AccountsAdapter::new(MockAccounts::present(fixture()).fail_writes("AccountsService: busy"));
    adapter.refresh();

    let outcome = adapter.set_locked(1000, true);
    assert_eq!(outcome.denial_note(), None);
    assert!(matches!(outcome, AccountOutcome::Failed(_)));
    assert_eq!(
        outcome.error().map(|e| e.message()),
        Some("AccountsService: busy")
    );
    assert!(adapter.state().is_available());
}

#[test]
fn the_vocabulary_round_trips_through_its_stable_ids() {
    for account_type in AccountType::ALL {
        assert_eq!(AccountType::from_id(account_type.id()), account_type);
        assert_eq!(AccountType::from_code(account_type.code()), account_type);
    }
    assert_eq!(AccountType::from_id("bogus"), AccountType::Standard);
    for mode in PasswordMode::ALL {
        assert_eq!(PasswordMode::from_id(mode.id()), mode);
        assert_eq!(PasswordMode::from_code(mode.code()), mode);
    }
    assert_eq!(PasswordMode::from_id("bogus"), PasswordMode::Regular);
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    let source = MockAccounts::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // The trait-object path is the same seam the live host fills.
    let mut boxed: Box<dyn AccountSource> = Box::new(MockAccounts::absent());
    assert_eq!(boxed.read(), Ok(None));
}

#[test]
fn an_empty_read_is_available_but_not_present() {
    // AccountsService answers with no cached user: the adapter is available,
    // and `present()` is what hides the pane.
    let mut adapter = AccountsAdapter::new(MockAccounts::present(AccountsData::default()));
    adapter.refresh();
    assert!(adapter.state().is_available());
    assert!(!adapter.snapshot().unwrap().present());
    assert_eq!(adapter.snapshot().unwrap().label(), "No Users");
}

#[test]
fn the_live_accounts_reads_when_a_session_is_present() {
    // The live D-Bus path is exercised only where an AccountsService exists
    // (the test machine); CI without one reports absence and skips.
    let mut source = HostAccounts::new();
    match source.read() {
        Ok(None) => {
            // No AccountsService here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = AccountsSnapshot::from_data(&data);
            for user in snapshot.users() {
                assert!(!user.user_name.is_empty());
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
