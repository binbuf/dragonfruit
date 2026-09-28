// SPDX-License-Identifier: MIT
//! T-15.5a acceptance: the adapter reports Mission Control and hot corners
//! state and events against a mock, and an absent host stack is a normal
//! hidden state. No compositor and no Wayland connection are involved.

use dragonfruit_overview::{
    GestureGating, HotCorner, HotCornerAction, HotCornerTrigger, MissionControlAdapter,
    MissionControlChange, MissionControlData, MissionControlSource, MockMissionControl,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

#[test]
fn the_mock_drives_the_default_configuration_and_runtime() {
    let mut adapter =
        MissionControlAdapter::new(MockMissionControl::present(MissionControlData::default()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the bridge answered");
    assert_eq!(
        snapshot.corner(HotCorner::TopLeft),
        HotCornerAction::MissionControl
    );
    assert_eq!(
        snapshot.corner(HotCorner::TopRight),
        HotCornerAction::NotificationCenter
    );
    assert_eq!(
        snapshot.corner(HotCorner::BottomLeft),
        HotCornerAction::DesktopReveal
    );
    assert_eq!(
        snapshot.corner(HotCorner::BottomRight),
        HotCornerAction::LockScreen
    );
    assert_eq!(snapshot.mission_control_corners(), vec![HotCorner::TopLeft]);
    assert!(snapshot.mission_control_reachable());
    assert_eq!(snapshot.glyph(), "overview");
    assert_eq!(snapshot.dwell.as_millis(), 150);
    assert!(!snapshot.overview.active);

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::MISSION_CONTROL);
    assert_eq!(slot.id, AdapterId::MISSION_CONTROL);
    assert!(slot.visible && slot.enabled);
    let projected = StatusSource::slot(&adapter);
    assert_eq!(projected.id, AdapterId::MISSION_CONTROL);
    assert!(projected.visible);
}

#[test]
fn the_lifecycle_resubscribes_after_the_bridge_returns() {
    let mut adapter =
        MissionControlAdapter::new(MockMissionControl::present(MissionControlData::default()));
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
    assert!(!adapter.state().slot(AdapterId::MISSION_CONTROL).visible);
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
fn a_configuration_change_is_observable_across_reads() {
    let mut adapter =
        MissionControlAdapter::new(MockMissionControl::present(MissionControlData::default()));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    // Move Mission Control from the top-left corner to the bottom-right, and
    // turn the Mission Control gesture off.
    let mut corners = MissionControlData::default().corners;
    corners[HotCorner::TopLeft.index()] = HotCornerAction::NotificationCenter;
    corners[HotCorner::BottomRight.index()] = HotCornerAction::MissionControl;
    let next = MissionControlData {
        corners,
        gesture_mission_control: false,
        ..MissionControlData::default()
    };
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&MissionControlChange::CornerAssignment {
        corner: HotCorner::TopLeft,
        from: HotCornerAction::MissionControl,
        to: HotCornerAction::NotificationCenter,
    }));
    assert!(changes.contains(&MissionControlChange::CornerAssignment {
        corner: HotCorner::BottomRight,
        from: HotCornerAction::LockScreen,
        to: HotCornerAction::MissionControl,
    }));
    assert!(changes.contains(&MissionControlChange::GestureGating {
        from: GestureGating {
            enabled: true,
            space_switch: true,
            mission_control: true,
        },
        to: GestureGating {
            enabled: true,
            space_switch: true,
            mission_control: false,
        },
    }));

    let snapshot = adapter.snapshot().unwrap();
    assert_eq!(
        snapshot.mission_control_corners(),
        vec![HotCorner::BottomRight]
    );
    // No gesture, one corner: still reachable.
    assert!(snapshot.mission_control_reachable());
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn the_overview_runtime_is_an_event_stream() {
    let mut adapter =
        MissionControlAdapter::new(MockMissionControl::present(MissionControlData::default()));
    adapter.refresh();
    let _ = adapter.drain_changes();

    let next = MissionControlData {
        overview_active: true,
        selected_window: Some("org.example.App:1".to_owned()),
        windows: 3,
        ..MissionControlData::default()
    };
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&MissionControlChange::OverviewOpened));
    assert!(changes.contains(&MissionControlChange::SelectionChanged {
        from: None,
        to: Some("org.example.App:1".to_owned()),
    }));
    assert!(changes.contains(&MissionControlChange::WindowCountChanged { from: 0, to: 3 }));
    assert!(adapter.snapshot().unwrap().overview.has_selection());

    // Close it again; the selection clears.
    adapter.source_mut().push(MissionControlData::default());
    adapter.refresh();
    let changes = adapter.drain_changes();
    assert!(changes.contains(&MissionControlChange::OverviewClosed));
    assert!(changes.contains(&MissionControlChange::SelectionChanged {
        from: Some("org.example.App:1".to_owned()),
        to: None,
    }));
}

#[test]
fn a_hot_corner_trigger_is_observable_and_carries_its_action() {
    let mut adapter =
        MissionControlAdapter::new(MockMissionControl::present(MissionControlData::default()));
    adapter.refresh();

    adapter.source_mut().trigger(HotCorner::TopLeft);
    adapter.source_mut().trigger(HotCorner::BottomRight);
    adapter.refresh();

    // A trigger drains exactly once.
    assert_eq!(
        adapter.drain_triggers(),
        vec![
            HotCornerTrigger {
                corner: HotCorner::TopLeft,
                action: HotCornerAction::MissionControl,
            },
            HotCornerTrigger {
                corner: HotCorner::BottomRight,
                action: HotCornerAction::LockScreen,
            },
        ]
    );
    assert_eq!(adapter.pending_triggers(), 0);
    assert!(adapter.drain_triggers().is_empty());
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No bridge at all: hidden, absent, no error, no panic.
    let mut adapter = MissionControlAdapter::new(MockMissionControl::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
    assert!(adapter.drain_triggers().is_empty());

    // The bridge is present but unreadable: visible, inert, with the message.
    let mut failing =
        MissionControlAdapter::new(MockMissionControl::failing("bridge: cannot read overview"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "bridge: cannot read overview"
    );
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    // Constructing the mock touches no process and no socket.
    let source = MockMissionControl::absent();
    assert!(!source.is_present());
    assert_eq!(source.pending_triggers(), 0);
    // And the trait object path is the same seam the live bridge will fill.
    let mut boxed: Box<dyn MissionControlSource> = Box::new(MockMissionControl::absent());
    assert_eq!(boxed.read(), Ok(None));
    // The action vocabulary round-trips through the stable ids the settings
    // key and the wire use.
    for action in HotCornerAction::ALL {
        assert_eq!(HotCornerAction::from_id(action.id()), Some(action));
    }
}
