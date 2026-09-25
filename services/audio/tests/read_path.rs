// SPDX-License-Identifier: MIT
//! T-07.3 acceptance: the adapter reads a mocked PipeWire/WirePlumber and
//! renders the default sink + per-sink list, and an absent daemon hides the
//! item. No PipeWire and no daemon are involved.

use dragonfruit_audio::{
    AudioAdapter, AudioData, AudioSnapshot, AudioSource, CommandAudio, MockAudio,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture captured from a real `pw-dump` and shaped exactly like the raw
/// read the live source decodes.
fn fixture() -> AudioData {
    AudioData::from_pw_dump(include_str!("fixtures/pw-dump-office.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_the_default_sink_and_the_sink_list() {
    let mut adapter = AudioAdapter::new(MockAudio::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("WirePlumber answered");
    assert_eq!(snapshot.sink_count(), 2);
    let default = snapshot.default_sink().expect("a default sink");
    assert_eq!(default.name, "alsa_output.pci-0000_02_01.0.analog-stereo");
    assert!(default.description.contains("Analog Stereo"));
    // Stored 0.216 = 0.6^3; the menu sees the linear 0.6.
    assert_eq!(snapshot.volume_percent(), 60);
    assert!((snapshot.level() - 0.6).abs() < 0.001);
    assert!(!snapshot.muted());
    assert_eq!(snapshot.glyph(), "volume");
    assert_eq!(snapshot.label(), "60%");

    // The slot is live and the source is the menu-bar status source.
    let slot = adapter.state().slot(AdapterId::AUDIO);
    assert_eq!(slot.id, AdapterId::AUDIO);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::AUDIO);
}

#[test]
fn an_absent_wireplumber_hides_the_item_and_never_errors() {
    let mut adapter = AudioAdapter::new(MockAudio::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::AUDIO);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = AudioAdapter::new(MockAudio::failing("PipeWire: core unavailable"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::AUDIO);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("PipeWire: core unavailable"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = AudioAdapter::new(MockAudio::present(fixture()));
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
    assert!(!adapter.state().slot(AdapterId::AUDIO).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::AUDIO).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert_eq!(adapter.snapshot().unwrap().volume_percent(), 60);
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
    let mut adapter = AudioAdapter::new(MockAudio::present(fixture()));
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

#[test]
fn the_live_wireplumber_reads_when_a_session_is_present() {
    // The live CLI path is exercised only where a PipeWire session exists
    // (the test machine); CI without one reports absence and skips. This is
    // the task's "one real daemon where available" check.
    let mut source = CommandAudio::new();
    match source.read() {
        Ok(None) => {
            // No PipeWire session here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = AudioSnapshot::from_data(&data);
            if !snapshot.sinks.is_empty() {
                assert!(
                    snapshot.default_sink().is_some(),
                    "a non-empty graph resolves a default sink"
                );
            }
            for sink in &snapshot.sinks {
                assert!(!sink.name.is_empty());
                assert!((0.0..=1.0).contains(&sink.volume));
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
