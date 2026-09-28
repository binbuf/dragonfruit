// SPDX-License-Identifier: MIT
//! The Mission Control and hot corners adapter: one read path over the shared
//! contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the bridge calls it when the
//! configuration or the overview state changes, so nothing above the adapter
//! polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "bridge absent" default, so a session
//! booted without a compositor bridge renders a hidden item and never blocks.
//!
//! The adapter is read-only for runtime state: the compositor owns the one
//! overview machine and the adapter only mirrors it; the durable trigger
//! configuration is `settingsd`'s and the compositor applies it live. This
//! adapter answers "what triggers Mission Control, and is the overview open?".

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{
    HotCornerAction, HotCornerTrigger, MissionControlChange, MissionControlSnapshot,
};
use crate::source::MissionControlSource;

/// The Mission Control and hot corners adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionControlAdapter<S> {
    source: S,
    state: AdapterState<MissionControlSnapshot>,
    previous: Option<MissionControlSnapshot>,
    changes: VecDeque<MissionControlChange>,
    triggers: VecDeque<HotCornerTrigger>,
    subscription: Subscription,
}

impl<S: MissionControlSource> MissionControlAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        MissionControlAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            triggers: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the host stack once and update the state and the event streams.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports a [`MissionControlChange`] per field that moved.
    /// Corner triggers observed since the last call are queued as
    /// [`HotCornerTrigger`]s.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = MissionControlSnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot.clone());
                self.state = AdapterState::available(snapshot);
                let corners = self.source.take_triggers();
                for corner in corners {
                    let action = self
                        .state
                        .snapshot()
                        .map(|snapshot| snapshot.corner(corner))
                        .unwrap_or(HotCornerAction::None);
                    self.triggers.push_back(HotCornerTrigger { corner, action });
                }
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
    pub fn snapshot(&self) -> Option<&MissionControlSnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<MissionControlChange> {
        self.changes.drain(..).collect()
    }

    /// Take the hot-corner triggers pushed since the last drain, in order.
    pub fn drain_triggers(&mut self) -> Vec<HotCornerTrigger> {
        self.triggers.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }

    /// The trigger count not yet drained.
    pub fn pending_triggers(&self) -> usize {
        self.triggers.len()
    }
}

impl<S: MissionControlSource> Adapter for MissionControlAdapter<S> {
    type Snapshot = MissionControlSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::MISSION_CONTROL
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
    use crate::model::HotCorner;
    use crate::source::{MissionControlData, MockMissionControl};

    fn data() -> MissionControlData {
        MissionControlData::default()
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = MissionControlAdapter::new(MockMissionControl::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::MISSION_CONTROL).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = MissionControlAdapter::new(MockMissionControl::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(
            snapshot.corner(HotCorner::TopLeft),
            HotCornerAction::MissionControl
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
        let mut adapter = MissionControlAdapter::new(MockMissionControl::present(data()));
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
        let mut adapter =
            MissionControlAdapter::new(MockMissionControl::failing("bridge: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "bridge: timeout"
        );
    }

    #[test]
    fn a_config_change_shows_up_on_the_next_read() {
        let mut adapter = MissionControlAdapter::new(MockMissionControl::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.corners[HotCorner::TopLeft.index()] = HotCornerAction::DesktopReveal;
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![MissionControlChange::CornerAssignment {
                corner: HotCorner::TopLeft,
                from: HotCornerAction::MissionControl,
                to: HotCornerAction::DesktopReveal,
            }]
        );
        // The change is drained exactly once.
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn opening_the_overview_is_an_event() {
        let mut adapter = MissionControlAdapter::new(MockMissionControl::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.overview_active = true;
        next.selected_window = Some("app:2".to_owned());
        adapter.source_mut().push(next);
        adapter.refresh();

        let changes = adapter.drain_changes();
        assert!(changes.contains(&MissionControlChange::OverviewOpened));
        assert!(changes.contains(&MissionControlChange::SelectionChanged {
            from: None,
            to: Some("app:2".to_owned()),
        }));
    }

    #[test]
    fn a_corner_trigger_carries_its_assigned_action() {
        let mut adapter = MissionControlAdapter::new(MockMissionControl::present(data()));
        adapter.refresh();
        adapter.source_mut().trigger(HotCorner::TopLeft);
        adapter.source_mut().trigger(HotCorner::BottomRight);
        adapter.refresh();
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
    }
}
