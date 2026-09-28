// SPDX-License-Identifier: MIT
//! T-15.3a acceptance: the adapter reports the output and input device lists
//! plus their default routing, switches the routed device, and treats an
//! absent daemon as a normal state. No PipeWire and no daemon are involved.

use dragonfruit_audio::{
    AudioAdapter, AudioData, AudioSnapshot, AudioSource, CommandAudio, MockAudio, SetOutcome,
};
use dragonfruit_system_adapters::{Adapter, AdapterEvent, AdapterId, ConnectionState};

/// A fixture captured from a real `pw-dump`, extended with an input node and
/// both default-routing metadata entries.
fn fixture() -> AudioData {
    AudioData::from_pw_dump(include_str!("fixtures/pw-dump-office.json")).expect("valid fixture")
}

fn connected(volume: f32) -> AudioAdapter<MockAudio> {
    let mut adapter = AudioAdapter::new(MockAudio::present(fixture()));
    adapter.refresh();
    adapter.set_volume(volume);
    adapter.refresh();
    adapter
}

#[test]
fn the_fixture_reports_output_and_input_devices_and_defaults() {
    let adapter = connected(0.6);
    let snapshot = adapter.snapshot().expect("WirePlumber answered");

    assert_eq!(snapshot.sink_count(), 2);
    assert_eq!(snapshot.source_count(), 1);
    assert_eq!(
        snapshot.default_sink.as_deref(),
        Some("alsa_output.pci-0000_02_01.0.analog-stereo")
    );
    assert_eq!(
        snapshot.default_source_name(),
        Some("alsa_input.pci-0000_02_01.0.analog-stereo")
    );

    let input = snapshot.default_source_device().expect("a default source");
    assert_eq!(input.id, 150);
    assert!(input.description.contains("Built-in Audio"));
    assert!((input.volume - 0.4).abs() < 0.001);
    assert!(input.default);

    // Both device lists sort the default first.
    assert!(snapshot.sinks[0].default);
    assert!(snapshot.sources[0].default);
    assert!(adapter.state().is_available());
    assert_eq!(adapter.state().slot(AdapterId::AUDIO).error, None);
}

#[test]
fn switching_the_default_output_device_reads_back_within_one_event() {
    let mut adapter = connected(0.6);
    let _ = adapter.drain_events();

    // The USB headset is the second fixture sink, node id 149.
    let target = adapter.snapshot().unwrap().sinks[1].clone();
    assert!(!target.default);
    assert_eq!(adapter.set_default_sink(target.id), SetOutcome::Applied);
    assert_eq!(adapter.source().default_sink_writes(), 1);

    // The switch does not invent a snapshot; the push + refresh does.
    assert_eq!(
        adapter.snapshot().unwrap().default_sink.as_deref(),
        Some("alsa_output.pci-0000_02_01.0.analog-stereo")
    );
    adapter.refresh();
    assert_eq!(
        adapter.snapshot().unwrap().default_sink.as_deref(),
        Some("alsa_output.usb-Headset-00.analog-stereo")
    );
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    // Volume now follows the newly-routed output.
    assert_eq!(adapter.snapshot().unwrap().volume_percent(), 50);
}

#[test]
fn switching_the_default_input_device_reads_back_within_one_event() {
    let mut adapter = connected(0.6);
    let _ = adapter.drain_events();

    // One input in the fixture: re-selecting it is idempotent but still a
    // real write whose result the daemon pushes.
    assert_eq!(adapter.set_default_source(150), SetOutcome::Applied);
    assert_eq!(adapter.source().default_source_writes(), 1);
    adapter.refresh();
    assert_eq!(
        adapter.snapshot().unwrap().default_source_name(),
        Some("alsa_input.pci-0000_02_01.0.analog-stereo")
    );
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn routing_to_an_unknown_node_is_a_failed_write_not_a_panic() {
    let mut adapter = connected(0.6);
    let before = adapter.snapshot().unwrap().clone();

    let outcome = adapter.set_default_sink(9999);
    assert!(matches!(outcome, SetOutcome::Failed(_)));
    assert!(adapter.state().is_available());
    assert_eq!(adapter.snapshot().unwrap(), &before);
}

#[test]
fn an_absent_daemon_is_a_normal_state_for_routing() {
    let mut adapter = AudioAdapter::new(MockAudio::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert!(!adapter.state().slot(AdapterId::AUDIO).visible);

    assert_eq!(adapter.set_default_sink(1), SetOutcome::Absent);
    assert_eq!(adapter.set_default_source(150), SetOutcome::Absent);
    assert!(adapter.state().is_unavailable());
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn the_live_source_reads_both_device_lists_when_a_session_is_present() {
    // Exercised only where a PipeWire session exists; CI without one reports
    // absence and skips. This is the task's "one real daemon where available"
    // check.
    let mut source = CommandAudio::new();
    match source.read() {
        Ok(None) => {}
        Ok(Some(data)) => {
            let snapshot = AudioSnapshot::from_data(&data);
            for sink in &snapshot.sinks {
                assert!(!sink.name.is_empty());
                assert!((0.0..=1.0).contains(&sink.volume));
            }
            for input in &snapshot.sources {
                assert!(!input.name.is_empty());
                assert!((0.0..=1.0).contains(&input.volume));
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
