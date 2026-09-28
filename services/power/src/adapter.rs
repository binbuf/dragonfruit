// SPDX-License-Identifier: MIT
//! The power adapter: one read path over the shared contract.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{PowerChange, PowerProfile, PowerSnapshot};
use crate::source::{PowerSource, ProfileOutcome};

/// The battery and power-profiles adapter.
///
/// It holds the last snapshot the daemon pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the daemons; the host calls it when UPower or
/// power-profiles-daemon signals a change, so nothing above the adapter polls.
/// A read is diffed against the previous one into a [`PowerChange`] stream,
/// and [`set_active_profile`](Self::set_active_profile) is the one write.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a
/// session booted without either daemon renders a hidden item and never
/// blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerAdapter<S> {
    source: S,
    state: AdapterState<PowerSnapshot>,
    previous: Option<PowerSnapshot>,
    changes: VecDeque<PowerChange>,
    subscription: Subscription,
}

impl<S: PowerSource> PowerAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        PowerAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the daemons once and update the state and the event streams.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent (both daemons) → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// A machine with no battery still answers `Available` with an empty
    /// battery snapshot; [`PowerSnapshot::present`] is what hides the item
    /// then. A missing power-profiles-daemon is carried inside the snapshot
    /// (`profiles` is `None`) and is a normal state; it does not make the
    /// adapter unavailable. The first successful read is the baseline and
    /// reports no domain change; each later read reports a [`PowerChange`] per
    /// field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = PowerSnapshot::from_data(&data);
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

    /// Select the active power profile. One explicit write.
    ///
    /// A successful write invents no snapshot: the daemon pushes the resulting
    /// state and the host calls [`refresh`](Self::refresh), so the snapshot
    /// stays the single source of truth. The caller reacts to the
    /// [`ProfileOutcome`] and [refreshes](Self::refresh) on success.
    pub fn set_active_profile(&mut self, profile: PowerProfile) -> ProfileOutcome {
        let outcome = self.source.set_active_profile(profile);
        if matches!(outcome, ProfileOutcome::Absent) {
            // The profiles daemon is gone; re-read so the snapshot drops the
            // profile half and the state stays truthful.
            self.refresh();
        }
        outcome
    }

    /// The live snapshot, when a daemon answered.
    pub fn snapshot(&self) -> Option<&PowerSnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<PowerChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: PowerSource> Adapter for PowerAdapter<S> {
    type Snapshot = PowerSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::POWER
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
    use crate::source::{MockPower, PowerData, PowerDeviceData, DEVICE_TYPE_BATTERY};

    fn data(percentage: f64) -> PowerData {
        PowerData {
            on_battery: false,
            devices: vec![PowerDeviceData {
                path: "/org/freedesktop/UPower/devices/battery_BAT0".to_owned(),
                kind: DEVICE_TYPE_BATTERY,
                present: true,
                power_supply: true,
                percentage,
                state: 1,
                battery_level: 4,
                capacity: 96.0,
                charge_cycles: 0,
                time_to_empty: 0,
                time_to_full: 5400,
            }],
            profiles: None,
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = PowerAdapter::new(MockPower::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::POWER).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = PowerAdapter::new(MockPower::present(data(55.0)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().percentage_percent(), 55);
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = PowerAdapter::new(MockPower::present(data(55.0)));
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
    fn a_present_daemon_with_no_battery_is_available_but_has_no_battery() {
        let mut adapter = PowerAdapter::new(MockPower::present(PowerData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "No battery");
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = PowerAdapter::new(MockPower::failing("UPower: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_level_change_is_a_domain_event() {
        let mut adapter = PowerAdapter::new(MockPower::present(data(55.0)));
        adapter.refresh();
        // The first read is the baseline: no change reported.
        assert!(adapter.drain_changes().is_empty());

        adapter.source_mut().push(data(54.0));
        adapter.refresh();
        assert_eq!(
            adapter.drain_changes(),
            vec![PowerChange::BatteryLevelChanged { from: 55, to: 54 }]
        );
        // Drained exactly once.
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn a_profile_write_updates_the_snapshot_on_refresh() {
        use crate::source::{PowerProfileData, PowerProfilesData};

        let mut raw = data(55.0);
        raw.profiles = Some(PowerProfilesData {
            active_profile: "balanced".to_owned(),
            profiles: vec![
                PowerProfileData {
                    profile: "power-saver".to_owned(),
                    ..PowerProfileData::default()
                },
                PowerProfileData {
                    profile: "balanced".to_owned(),
                    ..PowerProfileData::default()
                },
            ],
            ..PowerProfilesData::default()
        });
        let mut adapter = PowerAdapter::new(MockPower::present(raw));
        adapter.refresh();
        let _ = adapter.drain_changes();
        assert_eq!(
            adapter.snapshot().unwrap().active_profile(),
            Some(PowerProfile::Balanced)
        );

        assert_eq!(
            adapter.set_active_profile(PowerProfile::PowerSaver),
            ProfileOutcome::Applied
        );
        // The write invents no snapshot until the host re-reads.
        adapter.refresh();
        assert_eq!(
            adapter.snapshot().unwrap().active_profile(),
            Some(PowerProfile::PowerSaver)
        );
        assert_eq!(
            adapter.drain_changes(),
            vec![PowerChange::ActiveProfileChanged {
                from: Some(PowerProfile::Balanced),
                to: Some(PowerProfile::PowerSaver),
            }]
        );
    }

    #[test]
    fn an_absent_profile_write_resyncs_to_unavailable() {
        let mut adapter = PowerAdapter::new(MockPower::absent());
        adapter.refresh();
        assert_eq!(
            adapter.set_active_profile(PowerProfile::Performance),
            ProfileOutcome::Absent
        );
        // The resync leaves the item hidden; absence is normal.
        assert!(adapter.state().is_unavailable());
    }
}
