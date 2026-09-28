// SPDX-License-Identifier: MIT
//! T-15.4a acceptance: the adapter reads a mocked libinput and reports the
//! keyboard/mouse/trackpad inventory and its per-device state, a device change
//! is observable, and an absent host stack is a normal hidden state. No
//! hardware and no `libinput` tool are involved.

use dragonfruit_input::{
    CommandLibinput, DeviceChange, DeviceKind, InputAdapter, InputData, InputSource, MockInput,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like `libinput list-devices` output (see the
/// adapter's `libinput.rs` parser tests and ADR 0124).
fn fixture() -> InputData {
    InputData::from_list_devices(include_str!("fixtures/libinput-list-devices.txt"))
}

#[test]
fn the_fixture_renders_the_input_inventory() {
    let mut adapter = InputAdapter::new(MockInput::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("libinput answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.count(), 4);
    assert_eq!(snapshot.keyboards().len(), 1);
    assert_eq!(snapshot.mice().len(), 1);
    assert_eq!(snapshot.touchpads().len(), 1);
    assert_eq!(snapshot.pointers().len(), 2);
    assert_eq!(snapshot.glyph(), "keyboard");
    assert_eq!(snapshot.label(), "1 keyboards, 2 pointing devices");

    // Keyboards sort first, then the mouse, then the touchpad, then the
    // touchscreen.
    let kinds: Vec<DeviceKind> = snapshot
        .devices()
        .iter()
        .map(|device| device.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            DeviceKind::Keyboard,
            DeviceKind::Mouse,
            DeviceKind::Touchpad,
            DeviceKind::Touchscreen
        ]
    );

    // The touchpad carries its size and libinput's built-in defaults.
    let touchpad = &snapshot.touchpads()[0];
    assert_eq!(touchpad.name, "SynPS/2 Synaptics TouchPad");
    assert_eq!(touchpad.size_mm, Some((97.33, 66.86)));
    assert_eq!(touchpad.tap_to_click, Some(true));
    assert_eq!(touchpad.natural_scroll, Some(true));
    assert_eq!(touchpad.disable_while_typing, Some(true));
    assert_eq!(touchpad.default_accel_profile.as_deref(), Some("adaptive"));
    assert_eq!(
        touchpad.default_scroll_method.as_deref(),
        Some("two-finger")
    );
    assert_eq!(
        touchpad.default_click_method.as_deref(),
        Some("button-areas")
    );
    assert_eq!(touchpad.seat_name(), "seat0");

    // A keyboard has no pointer defaults; a mouse has no tapping.
    assert_eq!(snapshot.keyboards()[0].tap_to_click, None);
    assert_eq!(snapshot.mice()[0].tap_to_click, None);
    assert_eq!(
        snapshot.mice()[0].default_scroll_method.as_deref(),
        Some("button")
    );

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::INPUT);
    assert_eq!(slot.id, AdapterId::INPUT);
    assert!(slot.visible && slot.enabled);
    let projected = StatusSource::slot(&adapter);
    assert_eq!(projected.id, AdapterId::INPUT);
    assert!(projected.visible);
}

#[test]
fn the_lifecycle_resubscribes_after_the_stack_returns() {
    let mut adapter = InputAdapter::new(MockInput::present(fixture()));
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
    assert!(!adapter.state().slot(AdapterId::INPUT).visible);
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
fn a_device_change_is_observable_across_reads() {
    let mut adapter = InputAdapter::new(MockInput::present(fixture()));
    adapter.refresh();
    let before = adapter.snapshot().cloned().unwrap();
    assert_eq!(before.count(), 4);
    let _ = adapter.drain_events();

    // Unplug the mouse and plug in a drawing tablet.
    let mut next = fixture();
    next.devices
        .retain(|device| !device.name.contains("Optical Mouse"));
    next.devices.push(dragonfruit_input::InputDeviceData {
        name: "Wacom Pen Tablet".to_owned(),
        kernel: "/dev/input/event9".to_owned(),
        capabilities: vec!["tablet".to_owned()],
        ..dragonfruit_input::InputDeviceData::default()
    });
    adapter.source_mut().push(next);
    adapter.refresh();

    let after = adapter.snapshot().unwrap();
    assert_eq!(after.count(), 4);
    let changes = after.device_changes(&before);
    assert_eq!(changes.len(), 2);
    assert!(matches!(&changes[0], DeviceChange::Added { name, .. } if name == "Wacom Pen Tablet"));
    assert!(
        matches!(&changes[1], DeviceChange::Removed { name, .. } if name.contains("Optical Mouse"))
    );
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No libinput at all: hidden, absent, no error, no panic.
    let mut adapter = InputAdapter::new(MockInput::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());

    // libinput present but no recognized device: available, present false.
    let mut empty = InputAdapter::new(MockInput::present(InputData::default()));
    empty.refresh();
    assert!(empty.state().is_available());
    assert!(!empty.snapshot().unwrap().present());
    assert_eq!(empty.snapshot().unwrap().label(), "No input devices");

    // libinput present but unreadable: visible, inert, with the message.
    let mut failing = InputAdapter::new(MockInput::failing("libinput: cannot open seat"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "libinput: cannot open seat"
    );
}

#[test]
fn the_live_source_is_free_to_construct() {
    // Constructing the live source touches no process and no device.
    let source = CommandLibinput::new();
    let _ = source;
    // The parser is total: an empty run is a present stack with no devices.
    assert!(InputData::from_list_devices("").devices.is_empty());
    // And the trait object path is the same seam the mock drives.
    let mut boxed: Box<dyn InputSource> = Box::new(MockInput::absent());
    assert_eq!(boxed.read(), Ok(None));
}
