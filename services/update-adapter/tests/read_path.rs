// SPDX-License-Identifier: MIT
//! T-15.10a acceptance: the adapter reports General, About, and Updates state
//! and events against a mock, and an absent host stack is a normal hidden
//! state. No package manager, provider, or hardware is involved.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};
use dragonfruit_update_adapter::{
    MockSystem, SystemData, SystemIdentity, SystemSource, UpdateAdapter, UpdateChange, UpdateData,
    UpdateItem, UpdateOutcome, UpdatePhase, UpdateSeverity,
};

fn identity() -> SystemIdentity {
    SystemIdentity {
        host_name: "dragon".to_owned(),
        os_name: "Dragonfruit Linux".to_owned(),
        os_version: "44".to_owned(),
        os_id: "dragonfruit".to_owned(),
        kernel: "6.12.0".to_owned(),
        architecture: "x86_64".to_owned(),
        device_model: "Dragonfruit Book".to_owned(),
        processor: "Example CPU".to_owned(),
        memory_bytes: 16 * 1024 * 1024 * 1024,
        serial: "SERIAL".to_owned(),
    }
}

fn update(id: &str, severity: UpdateSeverity) -> UpdateItem {
    UpdateItem {
        id: id.to_owned(),
        name: id.to_owned(),
        summary: "An update".to_owned(),
        current_version: "1.0".to_owned(),
        available_version: "1.1".to_owned(),
        severity,
    }
}

fn data(phase: UpdatePhase, updates: Vec<UpdateItem>) -> SystemData {
    SystemData {
        identity: identity(),
        updates: Some(UpdateData {
            phase,
            updates,
            last_checked_ms: Some(1000),
            message: None,
        }),
    }
}

#[test]
fn the_mock_drives_the_about_identity_and_the_update_state() {
    let mut adapter = UpdateAdapter::new(MockSystem::present(data(
        UpdatePhase::Available,
        vec![update("glibc", UpdateSeverity::Security)],
    )));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the host stack answered");
    assert_eq!(snapshot.identity.host_name, "dragon");
    assert_eq!(snapshot.identity.os_label(), "Dragonfruit Linux 44");
    assert_eq!(snapshot.identity.memory_label(), "16 GB");
    assert_eq!(snapshot.identity.device_name(), "Dragonfruit Book");
    assert!(snapshot.identity.has_serial());
    assert!(snapshot.updates_available());
    assert_eq!(snapshot.phase(), Some(UpdatePhase::Available));
    assert_eq!(snapshot.update_count(), 1);
    assert_eq!(snapshot.security_count(), 1);
    assert_eq!(snapshot.label(), "1 Update Available");
    assert_eq!(snapshot.glyph(), "software-update");

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::UPDATES);
    assert_eq!(slot.id, AdapterId::UPDATES);
    assert!(slot.visible && slot.enabled);
    let projected = StatusSource::slot(&adapter);
    assert_eq!(projected.id, AdapterId::UPDATES);
    assert!(projected.visible);
}

#[test]
fn the_lifecycle_resubscribes_after_the_host_stack_returns() {
    let mut adapter = UpdateAdapter::new(MockSystem::present(data(UpdatePhase::UpToDate, vec![])));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // The stack goes away: the item hides, never an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::UPDATES).visible);
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
fn an_update_moves_through_the_phases_as_observable_events() {
    let mut adapter = UpdateAdapter::new(MockSystem::present(data(UpdatePhase::Idle, vec![])));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    // A check finds one security update.
    adapter.source_mut().push(data(
        UpdatePhase::Available,
        vec![update("kernel", UpdateSeverity::Security)],
    ));
    adapter.refresh();
    let changes = adapter.drain_changes();
    assert!(changes.contains(&UpdateChange::PhaseChanged {
        from: UpdatePhase::Idle,
        to: UpdatePhase::Available,
    }));
    assert!(changes.contains(&UpdateChange::UpdateListChanged { from: 0, to: 1 }));
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);

    // Installing, then a restart is required.
    adapter.source_mut().push(data(
        UpdatePhase::Installing,
        vec![update("kernel", UpdateSeverity::Security)],
    ));
    adapter.refresh();
    assert!(adapter.snapshot().unwrap().is_busy());

    adapter
        .source_mut()
        .push(data(UpdatePhase::RebootRequired, vec![]));
    adapter.refresh();
    let snapshot = adapter.snapshot().unwrap();
    assert!(snapshot.is_reboot_required());
    assert_eq!(snapshot.label(), "Restart Required");
}

#[test]
fn a_missing_provider_is_a_normal_available_state() {
    // The host identity answers but there is no distribution update provider:
    // the About rows stay live and only the update controls are absent.
    let mut adapter = UpdateAdapter::new(MockSystem::present(SystemData {
        identity: identity(),
        updates: None,
    }));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert_eq!(snapshot.identity.host_name, "dragon");
    assert!(!snapshot.updates_available());
    assert_eq!(snapshot.phase(), None);
    assert!(snapshot.updates().is_empty());
    assert_eq!(snapshot.label(), "Software Update Unavailable");

    // A check while the provider is absent is absent, never an error.
    assert_eq!(adapter.check(), UpdateOutcome::Absent);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No host stack at all: hidden, absent, no error, no panic.
    let mut adapter = UpdateAdapter::new(MockSystem::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
    assert_eq!(adapter.check(), UpdateOutcome::Absent);

    // The stack is present but unreadable: visible, inert, with the message.
    let mut failing =
        UpdateAdapter::new(MockSystem::failing("update provider: cannot read the host"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "update provider: cannot read the host"
    );
}

#[test]
fn the_writes_are_explicit_and_do_not_invent_a_snapshot() {
    let mut adapter = UpdateAdapter::new(MockSystem::present(data(
        UpdatePhase::Available,
        vec![update("glibc", UpdateSeverity::Security)],
    )));
    adapter.refresh();
    let before = adapter.snapshot().unwrap().clone();
    let _ = adapter.drain_events();
    let _ = adapter.drain_changes();

    assert_eq!(adapter.check(), UpdateOutcome::Applied);
    assert_eq!(adapter.install(), UpdateOutcome::Applied);
    assert_eq!(adapter.reboot(), UpdateOutcome::Applied);
    assert_eq!(adapter.source().checks(), 1);
    assert_eq!(adapter.source().installs(), 1);
    assert_eq!(adapter.source().reboots(), 1);

    // The adapter state is unchanged until the host re-reads.
    assert_eq!(adapter.snapshot().unwrap(), &before);
    assert!(adapter.drain_events().is_empty());

    // A denied or failed write is surfaced, not fatal.
    let mut denied = UpdateAdapter::new(
        MockSystem::present(data(UpdatePhase::Available, vec![])).fail_writes("polkit: denied"),
    );
    denied.refresh();
    let outcome = denied.install();
    assert!(!outcome.is_applied());
    assert_eq!(outcome.error().map(|e| e.message()), Some("polkit: denied"));
}

#[test]
fn the_vocabulary_round_trips_through_its_stable_ids() {
    for phase in UpdatePhase::ALL {
        assert_eq!(UpdatePhase::from_id(phase.id()), phase);
    }
    assert_eq!(UpdatePhase::from_id("bogus"), UpdatePhase::Idle);
    for severity in UpdateSeverity::ALL {
        assert_eq!(UpdateSeverity::from_id(severity.id()), severity);
    }
    assert_eq!(UpdateSeverity::from_id("bogus"), UpdateSeverity::Normal);
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    // Constructing the mock touches no process and no socket.
    let source = MockSystem::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // And the trait object path is the same seam the live host will fill.
    let mut boxed: Box<dyn SystemSource> = Box::new(MockSystem::absent());
    assert_eq!(boxed.read(), Ok(None));
}
