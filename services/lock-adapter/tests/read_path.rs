// SPDX-License-Identifier: MIT
//! T-15.8a acceptance: the adapter reports Lock Screen policy state and events
//! against a mock, and an absent host stack is a normal hidden state. No
//! compositor and no Wayland connection are involved.

use std::time::Duration;

use dragonfruit_lock_adapter::{
    IdlePolicy, IdleStage, LockDisplay, LockDisplayOption, LockPolicyAdapter, LockPolicyChange,
    LockPolicyData, LockPolicySnapshot, LockPolicySource, LockState, MockLockPolicy, IDLE_STAGES,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

fn data(locked: bool) -> LockPolicyData {
    LockPolicyData {
        locked,
        idle: IdlePolicy::new(),
        display: LockDisplay::default(),
    }
}

#[test]
fn the_mock_drives_the_default_policy_and_lock_state() {
    let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(LockPolicyData::default()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the host stack answered");
    assert_eq!(snapshot.state, LockState::Unlocked);
    assert!(!snapshot.is_locked());
    assert_eq!(snapshot.state_label(), "Unlocked");
    assert_eq!(snapshot.glyph(), "lock");
    // The session's shipped idle defaults are reused unchanged.
    assert_eq!(snapshot.display_off_after(), Some(Duration::from_secs(300)));
    assert_eq!(snapshot.lock_after(), Some(Duration::from_secs(600)));
    // The display defaults mirror the pane's controls.
    assert!(snapshot.display_option(LockDisplayOption::ShowUserNameAndPhoto));
    assert!(!snapshot.display_option(LockDisplayOption::ShowPasswordHints));
    assert!(snapshot.display_option(LockDisplayOption::ShowPowerButtons));

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::LOCK);
    assert_eq!(slot.id, AdapterId::LOCK);
    assert!(slot.visible && slot.enabled);
    let projected = StatusSource::slot(&adapter);
    assert_eq!(projected.id, AdapterId::LOCK);
    assert!(projected.visible);
}

#[test]
fn the_lifecycle_resubscribes_after_the_bridge_returns() {
    let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(false)));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // The bridge goes away: the item hides, never an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::LOCK).visible);
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
fn locking_and_unlocking_are_observable_across_reads() {
    let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(false)));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    adapter.source_mut().push(data(true));
    adapter.refresh();
    assert_eq!(
        adapter.drain_changes(),
        vec![LockPolicyChange::StateChanged {
            from: LockState::Unlocked,
            to: LockState::Locked,
        }]
    );
    assert!(adapter.snapshot().unwrap().is_locked());
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);

    adapter.source_mut().push(data(false));
    adapter.refresh();
    assert_eq!(
        adapter.drain_changes(),
        vec![LockPolicyChange::StateChanged {
            from: LockState::Locked,
            to: LockState::Unlocked,
        }]
    );
}

#[test]
fn policy_moves_are_an_event_stream() {
    let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(false)));
    adapter.refresh();
    let _ = adapter.drain_changes();

    let mut idle = IdlePolicy::new();
    idle.set_delay(IdleStage::Blank, Some(Duration::from_secs(900)));
    idle.set_delay(IdleStage::Suspend, Some(Duration::from_secs(3600)));
    let display = LockDisplay {
        show_message_when_locked: true,
        message: "Back at 3.".to_owned(),
        ..LockDisplay::default()
    };
    adapter.source_mut().push(LockPolicyData {
        locked: false,
        idle,
        display,
    });
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&LockPolicyChange::IdleDelayChanged {
        stage: IdleStage::Blank,
        from: Some(Duration::from_secs(300)),
        to: Some(Duration::from_secs(900)),
    }));
    assert!(changes.contains(&LockPolicyChange::IdleDelayChanged {
        stage: IdleStage::Suspend,
        from: None,
        to: Some(Duration::from_secs(3600)),
    }));
    assert!(changes.contains(&LockPolicyChange::DisplayOptionChanged {
        option: LockDisplayOption::ShowMessageWhenLocked,
        from: false,
        to: true,
    }));
    assert!(changes.contains(&LockPolicyChange::MessageChanged {
        from: String::new(),
        to: "Back at 3.".to_owned(),
    }));
    assert_eq!(adapter.snapshot().unwrap().message(), "Back at 3.");
    // The change is drained exactly once.
    assert_eq!(adapter.pending_changes(), 0);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No bridge at all: hidden, absent, no error, no panic.
    let mut adapter = LockPolicyAdapter::new(MockLockPolicy::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());

    // The bridge is present but unreadable: visible, inert, with the message.
    let mut failing =
        LockPolicyAdapter::new(MockLockPolicy::failing("lock bridge: cannot read policy"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "lock bridge: cannot read policy"
    );
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    // Constructing the mock touches no process and no socket.
    let source = MockLockPolicy::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // And the trait object path is the same seam the live bridge will fill.
    let mut boxed: Box<dyn LockPolicySource> = Box::new(MockLockPolicy::absent());
    assert_eq!(boxed.read(), Ok(None));

    // The reused idle vocabulary round-trips: the four configurable stages are
    // exactly the session's, and the display-option ids are the settings key
    // suffixes T-15.8b declares.
    assert_eq!(
        IDLE_STAGES,
        [
            IdleStage::Dim,
            IdleStage::Blank,
            IdleStage::Lock,
            IdleStage::Suspend,
        ]
    );
    for option in LockDisplayOption::ALL {
        assert!(option.id().starts_with("show"), "{}", option.id());
    }
}

#[test]
fn a_snapshot_diffs_cleanly_against_itself() {
    let snapshot = LockPolicySnapshot::from_data(&data(true));
    assert!(snapshot.changes(&snapshot).is_empty());
}
