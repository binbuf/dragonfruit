// SPDX-License-Identifier: MIT
//! The Bluetooth adapter: one state path and its explicit writes over the
//! shared contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::BluetoothSnapshot;
use crate::source::{BluetoothOutcome, BluetoothSource};

/// The Bluetooth status adapter.
///
/// It holds the last snapshot BlueZ pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the daemon; the host calls it when BlueZ signals a change, so
/// nothing above the adapter polls.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a
/// session booted without `bluetoothd` renders a hidden item and never blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothAdapter<S> {
    source: S,
    state: AdapterState<BluetoothSnapshot>,
    subscription: Subscription,
}

impl<S: BluetoothSource> BluetoothAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        BluetoothAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
        }
    }

    /// Read the daemon once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// A daemon with no controller still answers `Available` with an empty
    /// adapter slot; [`BluetoothSnapshot::present`] is what hides the item
    /// then, exactly as the battery item hides with no battery.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(BluetoothSnapshot::from_data(&data));
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

    /// Power the adapter on or off. One explicit write; a successful call
    /// changes nothing here — BlueZ pushes the new `Powered` and the host
    /// re-reads.
    pub fn set_powered(&mut self, powered: bool) -> BluetoothOutcome {
        let outcome = self.source.set_powered(powered);
        self.after_write(outcome)
    }

    /// Start or stop an inquiry. One explicit write; BlueZ pushes the new
    /// `Discovering`.
    pub fn set_discovering(&mut self, discovering: bool) -> BluetoothOutcome {
        let outcome = self.source.set_discovering(discovering);
        self.after_write(outcome)
    }

    /// Pair (bond) the device at `address`.
    pub fn pair(&mut self, address: &str) -> BluetoothOutcome {
        let outcome = self.source.pair(address);
        self.after_write(outcome)
    }

    /// Connect to or disconnect from the device at `address`.
    pub fn set_connected(&mut self, address: &str, connected: bool) -> BluetoothOutcome {
        let outcome = self.source.set_connected(address, connected);
        self.after_write(outcome)
    }

    /// A write against an absent daemon is absence at the state level too: the
    /// item hides and the subscription drops, never an error.
    fn after_write(&mut self, outcome: BluetoothOutcome) -> BluetoothOutcome {
        if matches!(outcome, BluetoothOutcome::Absent) {
            self.state = AdapterState::Unavailable;
            self.subscription.absent();
        }
        outcome
    }

    /// The live snapshot, when the daemon answered.
    pub fn snapshot(&self) -> Option<&BluetoothSnapshot> {
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
}

impl<S: BluetoothSource> Adapter for BluetoothAdapter<S> {
    type Snapshot = BluetoothSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::BLUETOOTH
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
    use crate::source::{BluetoothAdapterData, BluetoothData, BluetoothDeviceData, MockBluetooth};

    fn data(powered: bool) -> BluetoothData {
        BluetoothData {
            adapters: vec![BluetoothAdapterData {
                path: "/org/bluez/hci0".to_owned(),
                address: "11:22:33:44:55:66".to_owned(),
                name: "workstation".to_owned(),
                alias: "Workstation".to_owned(),
                powered,
                discoverable: false,
                pairable: true,
                discovering: false,
            }],
            devices: vec![BluetoothDeviceData {
                path: "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".to_owned(),
                adapter_path: "/org/bluez/hci0".to_owned(),
                address: "AA:BB:CC:DD:EE:FF".to_owned(),
                name: "Headset".to_owned(),
                alias: "WH-1000".to_owned(),
                paired: true,
                connected: false,
                trusted: true,
                blocked: false,
                rssi: -55,
                icon: "audio-headset".to_owned(),
            }],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = BluetoothAdapter::new(MockBluetooth::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::BLUETOOTH).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::present(data(true)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.snapshot().unwrap().powered());
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
        let mut adapter = BluetoothAdapter::new(MockBluetooth::present(data(true)));
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
    fn a_daemon_with_no_controller_is_available_but_absent_at_the_snapshot() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::present(BluetoothData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "Bluetooth unavailable");
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::failing("BlueZ: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
    }

    #[test]
    fn writes_pass_through_and_do_not_invent_a_snapshot() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::present(data(false)));
        adapter.refresh();

        assert!(adapter.set_powered(true).is_accepted());
        assert!(adapter.set_discovering(true).is_accepted());
        assert!(adapter.pair("AA:BB:CC:DD:EE:FF").is_accepted());
        assert!(adapter
            .set_connected("AA:BB:CC:DD:EE:FF", true)
            .is_accepted());

        // Still the old snapshot until BlueZ pushes and the host re-reads.
        assert!(!adapter.snapshot().unwrap().powered());
        assert_eq!(adapter.source().writes(), 4);
    }

    #[test]
    fn a_denial_is_reported_and_leaves_the_read_state_live() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::present(data(false)));
        adapter.refresh();
        adapter.source_mut().deny_writes("BlueZ: not authorized");

        let outcome = adapter.set_powered(true);
        assert_eq!(outcome.denial_note(), Some("BlueZ: not authorized"));
        assert!(adapter.state().is_available());
        assert!(adapter.state().slot(AdapterId::BLUETOOTH).visible);
    }

    #[test]
    fn a_write_against_an_absent_daemon_hides_the_item() {
        let mut adapter = BluetoothAdapter::new(MockBluetooth::absent());
        let outcome = adapter.set_powered(true);
        assert_eq!(outcome, BluetoothOutcome::Absent);
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::BLUETOOTH).visible);
    }
}
