// SPDX-License-Identifier: MIT
//! The Menu Bar configuration adapter: one read path over the shared contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the shell bridge calls it when the
//! menu bar's configuration or control availability changes, so nothing above
//! the adapter polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "host stack absent" default, so a
//! session booted without a menu-bar bridge renders a hidden item and never
//! blocks.
//!
//! The adapter is read-only: the durable preferences are `settingsd`'s
//! (T-15.9b) and the shell renders the bar. This adapter answers "what is the
//! menu bar configured to do, and which controls is it showing?".

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{MenuBarChange, MenuBarSnapshot};
use crate::source::MenuBarSource;

/// The Menu Bar configuration adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuBarAdapter<S> {
    source: S,
    state: AdapterState<MenuBarSnapshot>,
    previous: Option<MenuBarSnapshot>,
    changes: VecDeque<MenuBarChange>,
    subscription: Subscription,
}

impl<S: MenuBarSource> MenuBarAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        MenuBarAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the host stack once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports a [`MenuBarChange`] per field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = MenuBarSnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot.clone());
                self.state = AdapterState::available(snapshot);
                self.subscription.changed();
            }
            Ok(None) => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
            }
            Err(error) => {
                self.subscription.subscribed();
                self.state = AdapterState::Error(error);
                self.subscription.changed();
            }
        }
    }

    /// The live snapshot, when the host stack answered.
    pub fn snapshot(&self) -> Option<&MenuBarSnapshot> {
        self.state.snapshot()
    }

    /// The transport this adapter reads.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably (for tests and lifecycle control).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed (a restart counts again).
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the domain changes pushed since the last drain, in order.
    pub fn drain_changes(&mut self) -> Vec<MenuBarChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: MenuBarSource> Adapter for MenuBarAdapter<S> {
    type Snapshot = MenuBarSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::MENU_BAR
    }

    fn state(&self) -> &AdapterState<Self::Snapshot> {
        &self.state
    }

    fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClockOption, ClockOptions, MenuBarAutoHide, MenuBarControl};
    use crate::source::{MenuBarControlState, MenuBarData, MockMenuBar};

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = MenuBarAdapter::new(MockMenuBar::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::MENU_BAR).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(
            adapter.snapshot().unwrap().auto_hide,
            MenuBarAutoHide::FullScreen
        );
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.source().reads(), 1);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
        // The first read is the baseline: no domain change.
        assert!(adapter.drain_changes().is_empty());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
        adapter.refresh();
        let _ = adapter.drain_events();

        adapter.source_mut().kill();
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

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
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::failing("menu-bar bridge: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "menu-bar bridge: timeout"
        );
    }

    #[test]
    fn a_configuration_move_is_a_domain_event() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let next = MenuBarData {
            auto_hide: MenuBarAutoHide::Always,
            clock: ClockOptions {
                show_seconds: true,
                ..ClockOptions::default()
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
        assert!(changes.contains(&MenuBarChange::ClockOptionChanged {
            option: ClockOption::ShowSeconds,
            from: false,
            to: true,
        }));
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn a_control_coming_and_going_is_a_domain_event() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::present(MenuBarData::default()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = MenuBarData::default();
        next.set_control(MenuBarControl::Bluetooth, MenuBarControlState::inert());
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![MenuBarChange::ControlChanged {
                control: MenuBarControl::Bluetooth,
                from: MenuBarControlState::present(),
                to: MenuBarControlState::inert(),
            }]
        );
    }

    #[test]
    fn an_absent_write_free_adapter_stays_hidden() {
        let mut adapter = MenuBarAdapter::new(MockMenuBar::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        let slot = adapter.state().slot(AdapterId::MENU_BAR);
        assert!(!slot.visible && !slot.enabled);
    }
}
