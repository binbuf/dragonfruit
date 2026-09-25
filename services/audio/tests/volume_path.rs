// SPDX-License-Identifier: MIT
//! T-07.3 acceptance: a volume or mute change reflects in the adapter state
//! within one event. No PipeWire and no daemon are involved.

use dragonfruit_audio::{AudioAdapter, AudioData, AudioSnapshot, MockAudio, SetOutcome};
use dragonfruit_system_adapters::{Adapter, AdapterEvent, AdapterId};

/// A fixture captured from a real `pw-dump` and shaped exactly like the raw
/// read the live source decodes.
fn fixture() -> AudioData {
    AudioData::from_pw_dump(include_str!("fixtures/pw-dump-office.json")).expect("valid fixture")
}

fn connected_adapter() -> AudioAdapter<MockAudio> {
    let mut adapter = AudioAdapter::new(MockAudio::present(fixture()));
    adapter.refresh();
    adapter
}

#[test]
fn a_volume_change_reflects_within_one_event() {
    let mut adapter = connected_adapter();
    let _ = adapter.drain_events();
    assert_eq!(adapter.snapshot().unwrap().volume_percent(), 60);

    // The user drags the slider: one explicit write.
    assert_eq!(adapter.set_volume(0.25), SetOutcome::Applied);
    assert_eq!(adapter.source().volume_writes(), 1);

    // The daemon pushes once; one refresh reflects it.
    adapter.refresh();
    assert_eq!(adapter.snapshot().unwrap().volume_percent(), 25);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    // Exactly one write and no poll loop.
    assert_eq!(adapter.source().volume_writes(), 1);
}

#[test]
fn a_mute_change_reflects_within_one_event() {
    let mut adapter = connected_adapter();
    let _ = adapter.drain_events();
    assert!(!adapter.snapshot().unwrap().muted());

    assert_eq!(adapter.set_mute(true), SetOutcome::Applied);
    adapter.refresh();

    let snapshot = adapter.snapshot().unwrap();
    assert!(snapshot.muted());
    assert_eq!(snapshot.glyph(), "volume-muted");
    assert_eq!(snapshot.label(), "Muted");
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    assert_eq!(adapter.source().mute_writes(), 1);
}

#[test]
fn toggle_mute_reflects_within_one_event() {
    let mut adapter = connected_adapter();
    let _ = adapter.drain_events();

    assert_eq!(adapter.toggle_mute(), SetOutcome::Applied);
    adapter.refresh();
    assert!(adapter.snapshot().unwrap().muted());
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);

    assert_eq!(adapter.toggle_mute(), SetOutcome::Applied);
    adapter.refresh();
    assert!(!adapter.snapshot().unwrap().muted());
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn a_filtered_graph_still_names_its_default_sink() {
    // The fixture's default is the real first sink; the synthesized USB sink
    // is the non-default list entry. Rebuilding from raw data is stable.
    let snapshot = AudioSnapshot::from_data(&fixture());
    assert_eq!(snapshot.sink_count(), 2);
    assert_eq!(
        snapshot.default_sink().unwrap().name,
        "alsa_output.pci-0000_02_01.0.analog-stereo"
    );
    assert!(!snapshot.sinks[1].default);
    assert_eq!(snapshot.sinks[1].volume_percent(), 50);
}

#[test]
fn a_write_against_an_absent_daemon_reports_absence_and_never_errors() {
    let mut adapter = AudioAdapter::new(MockAudio::absent());
    assert_eq!(adapter.set_volume(0.5), SetOutcome::Absent);
    assert_eq!(adapter.set_mute(true), SetOutcome::Absent);
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert!(!adapter.state().slot(AdapterId::AUDIO).visible);
}

#[test]
fn a_failed_write_keeps_the_live_list_and_the_last_snapshot() {
    let mut adapter = connected_adapter();
    let before = adapter.snapshot().unwrap().clone();
    adapter.source_mut().fail_writes("wpctl: no such node");

    let outcome = adapter.set_volume(0.9);
    assert!(matches!(outcome, SetOutcome::Failed(_)));
    assert!(adapter.state().is_available());
    let slot = adapter.state().slot(AdapterId::AUDIO);
    assert!(slot.visible && slot.enabled);
    assert_eq!(adapter.snapshot().unwrap(), &before);
}
