// SPDX-License-Identifier: MIT
//! T-07.4 acceptance: the adapter reads a mocked UPower and renders battery
//! presence, level, and charging state, and an absent daemon hides the item.
//! No bus and no daemon are involved.

use dragonfruit_power::{
    BatteryHealth, DbusUPower, MockPower, PowerAdapter, PowerChange, PowerData, PowerProfile,
    PowerProfileData, PowerProfilesData, PowerSnapshot, PowerSource, ProfileOutcome,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live D-Bus source builds.
fn fixture() -> PowerData {
    serde_json::from_str(include_str!("fixtures/upower-laptop.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_the_battery_level_and_charge_state() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("UPower answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.percentage(), 82.0);
    assert_eq!(snapshot.percentage_percent(), 82);
    assert!(!snapshot.on_battery());
    assert!(snapshot.charging());
    assert!(snapshot.plugged());
    assert_eq!(snapshot.state().name(), "charging");
    assert_eq!(snapshot.glyph(), "battery");
    assert_eq!(snapshot.label(), "82% charging");
    assert_eq!(snapshot.time_to_full(), Some(5400));
    assert_eq!(snapshot.health(), BatteryHealth::Normal);
    assert_eq!(snapshot.battery().unwrap().charge_cycles, Some(112));

    // The fixture's power-profiles half is live too.
    assert!(snapshot.profiles_available());
    assert_eq!(snapshot.active_profile(), Some(PowerProfile::Balanced));
    assert_eq!(snapshot.profile_label(), "Balanced");
    assert_eq!(
        snapshot.profiles().unwrap().available,
        PowerProfile::ALL.to_vec()
    );

    // The slot is live and the source is the menu-bar status source.
    let slot = adapter.state().slot(AdapterId::POWER);
    assert_eq!(slot.id, AdapterId::POWER);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::POWER);
}

#[test]
fn an_absent_upower_hides_the_item_and_never_errors() {
    let mut adapter = PowerAdapter::new(MockPower::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::POWER);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_present_daemon_with_no_battery_has_no_item_to_show() {
    // A desktop or VM: UPower is up, but no battery is present. The adapter
    // is available; `present()` is what tells the shell to hide the item.
    let mut adapter = PowerAdapter::new(MockPower::present(PowerData::default()));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot.present());
    assert_eq!(snapshot.label(), "No battery");
    assert_eq!(snapshot.level(), 0.0);
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = PowerAdapter::new(MockPower::failing("UPower: timeout"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::POWER);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("UPower: timeout"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));
    adapter.refresh();
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // Mask the daemon: absence hides the item, not an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::POWER).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::POWER).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert_eq!(adapter.snapshot().unwrap().percentage_percent(), 82);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: true },
            AdapterEvent::Changed,
        ]
    );
}

#[test]
fn the_adapter_reads_once_per_refresh_never_in_a_poll() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));
    assert_eq!(adapter.source().reads(), 0);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 1);

    // Re-reading the state is free: a consumer never touches the daemon.
    let _ = adapter.state();
    let _ = adapter.snapshot();
    assert_eq!(adapter.source().reads(), 1);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 2);
}

fn profiles_data(active: &str) -> PowerProfilesData {
    PowerProfilesData {
        active_profile: active.to_owned(),
        profiles: PowerProfile::ALL
            .iter()
            .map(|profile| PowerProfileData {
                profile: profile.id().to_owned(),
                ..PowerProfileData::default()
            })
            .collect(),
        ..PowerProfilesData::default()
    }
}

#[test]
fn a_profile_write_goes_through_and_the_next_read_reports_it() {
    let raw = PowerData {
        profiles: Some(profiles_data("balanced")),
        ..fixture()
    };
    let mut adapter = PowerAdapter::new(MockPower::present(raw));
    adapter.refresh();
    let _ = adapter.drain_changes();
    assert_eq!(
        adapter.snapshot().unwrap().active_profile(),
        Some(PowerProfile::Balanced)
    );

    // A successful write does not invent a snapshot.
    assert_eq!(
        adapter.set_active_profile(PowerProfile::PowerSaver),
        ProfileOutcome::Applied
    );
    assert_eq!(
        adapter.snapshot().unwrap().active_profile(),
        Some(PowerProfile::Balanced)
    );

    // The daemon pushes the result; the host re-reads and the diff reports it.
    adapter.refresh();
    assert_eq!(
        adapter.snapshot().unwrap().active_profile(),
        Some(PowerProfile::PowerSaver)
    );
    assert!(adapter
        .drain_changes()
        .contains(&PowerChange::ActiveProfileChanged {
            from: Some(PowerProfile::Balanced),
            to: Some(PowerProfile::PowerSaver),
        }));
}

#[test]
fn an_absent_profiles_daemon_leaves_the_battery_live() {
    // UPower present, power-profiles-daemon masked: the adapter stays
    // available and the battery half works; the profile control disables.
    let raw = PowerData {
        profiles: None,
        ..fixture()
    };
    let mut adapter = PowerAdapter::new(MockPower::present(raw));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert!(snapshot.present());
    assert!(!snapshot.profiles_available());
    assert_eq!(snapshot.profile_label(), "Unavailable");
    assert_eq!(snapshot.profiles(), None);

    // A profile write has nowhere to go: absent, and the item stays live.
    assert_eq!(
        adapter.set_active_profile(PowerProfile::Performance),
        ProfileOutcome::Absent
    );
    assert!(adapter.state().is_available());
}

#[test]
fn a_profiles_only_machine_is_available_with_no_battery() {
    // A desktop whose UPower is masked but that runs power-profiles-daemon:
    // the profiles half is live, the battery half is empty, and presence is a
    // normal state rather than an adapter error.
    let mut adapter = PowerAdapter::new(MockPower::present(PowerData {
        profiles: Some(profiles_data("performance")),
        ..PowerData::default()
    }));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot.present());
    assert_eq!(snapshot.label(), "No battery");
    assert!(snapshot.profiles_available());
    assert_eq!(snapshot.active_profile(), Some(PowerProfile::Performance));
}

#[test]
fn both_daemons_absent_is_the_only_unavailable_state() {
    let mut adapter = PowerAdapter::new(MockPower::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
}

#[test]
fn battery_moves_between_reads_are_power_changes() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));
    adapter.refresh();
    let _ = adapter.drain_changes();

    // Discharge 82→41 and move the profile to power-saver.
    let mut next = fixture();
    next.on_battery = true;
    next.devices[1].percentage = 41.0;
    next.devices[1].state = 2;
    next.profiles.as_mut().unwrap().active_profile = "power-saver".to_owned();
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&PowerChange::BatteryLevelChanged { from: 82, to: 41 }));
    assert!(changes.contains(&PowerChange::OnBatteryChanged {
        from: false,
        to: true,
    }));
    assert!(changes.contains(&PowerChange::ActiveProfileChanged {
        from: Some(PowerProfile::Balanced),
        to: Some(PowerProfile::PowerSaver),
    }));
    // Drain is exactly once.
    assert_eq!(adapter.pending_changes(), 0);
}

#[test]
fn the_live_upower_reads_when_a_session_is_present() {
    // The live D-Bus path is exercised only where a UPower daemon exists (the
    // test machine); CI without one reports absence and skips. This is the
    // task's "VM/manual where available" check.
    let mut source = DbusUPower::new();
    match source.read() {
        Ok(None) => {
            // No UPower here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = PowerSnapshot::from_data(&data);
            for device in &data.devices {
                assert!(!device.path.is_empty());
            }
            if let Some(battery) = snapshot.battery() {
                assert!((0.0..=100.0).contains(&battery.percentage));
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
