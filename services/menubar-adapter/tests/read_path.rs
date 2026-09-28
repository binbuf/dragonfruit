// SPDX-License-Identifier: MIT
//! T-15.9a acceptance: the adapter reports Menu Bar configuration state and
//! events against a mock, and an absent host stack is a normal hidden state.
//! No shell, compositor, or Wayland connection is involved.

use dragonfruit_menubar_adapter::{
    ClockOption, MenuBarAdapter, MenuBarAutoHide, MenuBarChange, MenuBarControl,
    MenuBarControlState, MenuBarData, MenuBarSource, MockMenuBar,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

#[test]
fn the_mock_drives_the_shipped_configuration_and_controls() {
    let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the bridge answered");
    assert!(!snapshot.is_hidden());
    assert_eq!(snapshot.auto_hide, MenuBarAutoHide::FullScreen);
    assert!(snapshot.show_background);
    assert!(snapshot.global_menu);
    assert!(snapshot.clock_option(ClockOption::ShowDate));
    assert!(!snapshot.clock_option(ClockOption::ShowSeconds));
    assert_eq!(snapshot.glyph(), "menu-bar");
    assert_eq!(snapshot.visible_controls().len(), MenuBarControl::ALL.len());

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::MENU_BAR);
    assert_eq!(slot.id, AdapterId::MENU_BAR);
    assert!(slot.visible && slot.enabled);
    let projected = StatusSource::slot(&adapter);
    assert_eq!(projected.id, AdapterId::MENU_BAR);
    assert!(projected.visible);
}

#[test]
fn the_lifecycle_resubscribes_after_the_bridge_returns() {
    let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
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
    assert!(!adapter.state().slot(AdapterId::MENU_BAR).visible);
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
    let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    let next = MenuBarData {
        auto_hide: MenuBarAutoHide::Always,
        show_background: false,
        global_menu: false,
        clock: dragonfruit_menubar_adapter::ClockOptions {
            show_date: false,
            show_seconds: true,
        },
        ..MenuBarData::default()
    };
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&MenuBarChange::AutoHideChanged {
        from: MenuBarAutoHide::FullScreen,
        to: MenuBarAutoHide::Always,
    }));
    assert!(changes.contains(&MenuBarChange::BackgroundChanged {
        from: true,
        to: false,
    }));
    assert!(changes.contains(&MenuBarChange::GlobalMenuChanged {
        from: true,
        to: false,
    }));
    assert!(changes.contains(&MenuBarChange::ClockOptionChanged {
        option: ClockOption::ShowDate,
        from: true,
        to: false,
    }));
    assert!(changes.contains(&MenuBarChange::ClockOptionChanged {
        option: ClockOption::ShowSeconds,
        from: false,
        to: true,
    }));

    assert_eq!(
        adapter.snapshot().unwrap().auto_hide,
        MenuBarAutoHide::Always
    );
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn a_control_can_come_and_go_within_a_present_bridge() {
    let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
    adapter.refresh();
    let _ = adapter.drain_changes();

    // Bluetooth's daemon is absent: only its slot hides, the adapter stays
    // available and the rest of the bar is unaffected.
    let mut next = MenuBarData::default();
    next.set_control(MenuBarControl::Bluetooth, MenuBarControlState::absent());
    adapter.source_mut().push(next);
    adapter.refresh();

    assert!(adapter.state().is_available());
    let changes = adapter.drain_changes();
    assert_eq!(
        changes,
        vec![MenuBarChange::ControlChanged {
            control: MenuBarControl::Bluetooth,
            from: MenuBarControlState::present(),
            to: MenuBarControlState::absent(),
        }]
    );
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot
        .visible_controls()
        .contains(&MenuBarControl::Bluetooth));

    // It returns, present but inert (its adapter is up but unreadable).
    let mut next = MenuBarData::default();
    next.set_control(MenuBarControl::Bluetooth, MenuBarControlState::inert());
    adapter.source_mut().push(next);
    adapter.refresh();
    let changes = adapter.drain_changes();
    assert!(changes.contains(&MenuBarChange::ControlChanged {
        control: MenuBarControl::Bluetooth,
        from: MenuBarControlState::absent(),
        to: MenuBarControlState::inert(),
    }));
}

#[test]
fn the_runtime_hidden_flag_is_an_event_stream() {
    let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
    adapter.refresh();
    let _ = adapter.drain_changes();

    let next = MenuBarData {
        hidden: true,
        ..MenuBarData::default()
    };
    adapter.source_mut().push(next);
    adapter.refresh();
    assert_eq!(
        adapter.drain_changes(),
        vec![MenuBarChange::VisibilityChanged {
            from: false,
            to: true,
        }]
    );
    assert!(adapter.snapshot().unwrap().is_hidden());
    assert_eq!(adapter.snapshot().unwrap().label(), "Hidden");

    adapter.source_mut().push(MenuBarData::default());
    adapter.refresh();
    assert_eq!(
        adapter.drain_changes(),
        vec![MenuBarChange::VisibilityChanged {
            from: true,
            to: false,
        }]
    );
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No bridge at all: hidden, absent, no error, no panic.
    let mut adapter = MenuBarAdapter::new(MockMenuBar::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());

    // The bridge is present but unreadable: visible, inert, with the message.
    let mut failing =
        MenuBarAdapter::new(MockMenuBar::failing("menu-bar bridge: cannot read the bar"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "menu-bar bridge: cannot read the bar"
    );
}

#[test]
fn the_auto_hide_vocabulary_round_trips_through_its_settings_ids() {
    for mode in MenuBarAutoHide::ALL {
        assert_eq!(MenuBarAutoHide::from_id(mode.id()), Some(mode));
    }
    assert_eq!(MenuBarAutoHide::from_id("bogus"), None);
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    // Constructing the mock touches no process and no socket.
    let source = MockMenuBar::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // And the trait object path is the same seam the live bridge will fill.
    let mut boxed: Box<dyn MenuBarSource> = Box::new(MockMenuBar::absent());
    assert_eq!(boxed.read(), Ok(None));
    // The control ids are the shell status-item ids.
    for control in MenuBarControl::ALL {
        assert!(!control.id().is_empty());
    }
}
