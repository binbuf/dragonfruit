// SPDX-License-Identifier: MIT
//! T-07.1b acceptance: an adapter re-subscribes when its service restarts, and
//! an absent daemon hides the slot rather than erroring.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, MockAdapter, StatusSource,
};

/// The acceptance test: kill the mock, assert absence hides the slot, restart
/// it and assert the adapter re-subscribed and re-synced.
#[test]
fn absence_hides_the_slot_and_restart_resubscribes() {
    let mut wifi = MockAdapter::with_available(AdapterId::WIFI, 82u8);

    // A daemon that has never restarted is subscribed once, not re-subscribed.
    assert_eq!(wifi.connection(), ConnectionState::Subscribed);
    assert_eq!(wifi.subscriptions(), 1);
    assert_eq!(
        wifi.drain_events(),
        vec![AdapterEvent::Subscribed { resubscribe: false }]
    );

    // Kill the daemon. Absence is a normal state: not an error, slot hidden.
    wifi.kill();
    assert_eq!(wifi.connection(), ConnectionState::Absent);
    assert!(wifi.state().is_unavailable());
    assert!(!wifi.state().is_error());
    let hidden = wifi.slot();
    assert!(!hidden.visible && !hidden.enabled);
    assert_eq!(hidden.error, None);
    assert_eq!(wifi.drain_events(), vec![AdapterEvent::Disconnected]);

    // Restart it. The adapter re-subscribes and re-syncs with no user-visible
    // error; the slot is live again.
    wifi.restart();
    assert_eq!(wifi.connection(), ConnectionState::Subscribed);
    assert_eq!(wifi.subscriptions(), 2);
    assert!(wifi.state().is_available());
    assert_eq!(wifi.state().snapshot(), Some(&82));
    let live = wifi.slot();
    assert!(live.visible && live.enabled);
    assert_eq!(live.error, None);
    assert_eq!(
        wifi.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: true },
            AdapterEvent::Changed,
        ]
    );
}

/// The degradation rule: a daemon absent from the start is hidden and is never
/// an error, and the adapter can still subscribe if it later appears.
#[test]
fn an_absent_daemon_at_startup_is_hidden_and_never_errors() {
    let mut battery = MockAdapter::<u8>::new(AdapterId::POWER);

    assert_eq!(battery.connection(), ConnectionState::Absent);
    assert!(battery.state().is_unavailable());
    assert!(!battery.state().is_error());
    let slot = battery.slot();
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // The daemon starts later; the first subscribe is not a re-subscribe.
    battery.restart();
    assert!(battery.connection().is_subscribed());
    assert_eq!(battery.subscriptions(), 1);
    assert_eq!(
        battery.drain_events(),
        vec![AdapterEvent::Subscribed { resubscribe: false }]
    );
}

/// Several kill/restart cycles keep counting and never surface an error.
#[test]
fn repeated_restarts_keep_resubscribing() {
    let mut audio = MockAdapter::with_available(AdapterId::AUDIO, 40u8);
    let _ = audio.drain_events();

    for cycle in 2..=4 {
        audio.kill();
        assert!(!audio.slot().visible);
        audio.restart();
        assert!(audio.slot().visible && audio.slot().enabled);
        assert_eq!(audio.subscriptions(), cycle);
        assert_eq!(
            audio.drain_events(),
            vec![
                AdapterEvent::Disconnected,
                AdapterEvent::Subscribed { resubscribe: true },
                AdapterEvent::Changed,
            ]
        );
    }
}

/// A present daemon pushing data emits `Changed`; the adapter is never polled
/// from above.
#[test]
fn a_data_push_is_an_event_not_a_poll() {
    let mut adapter = MockAdapter::with_available(AdapterId::AUDIO, 10u8);
    let _ = adapter.drain_events();

    adapter.set_available(70);
    assert_eq!(adapter.state().snapshot(), Some(&70));
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}
