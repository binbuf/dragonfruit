// SPDX-License-Identifier: MIT
//! The input adapter: one read path over the shared contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::InputSnapshot;
use crate::source::InputSource;

/// The keyboard/mouse/trackpad inventory adapter.
///
/// It holds the last snapshot libinput pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the host stack; the host calls it when a device is added or removed,
/// so nothing above the adapter polls.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "stack absent" default, so a session
/// booted without libinput renders a hidden item and never blocks.
///
/// The adapter is read-only by design: libinput keeps no persisted
/// configuration and offers no setter, so the user's choices live in
/// `settingsd` and are applied by the compositor over its own API. This
/// adapter answers "which devices does the session have, and what can they
/// do?".
#[derive(Debug, Clone, PartialEq)]
pub struct InputAdapter<S> {
    source: S,
    state: AdapterState<InputSnapshot>,
    subscription: Subscription,
}

impl<S: InputSource> InputAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        InputAdapter {
            source,
            state: AdapterState::Unavailable,
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
    /// A stack with no devices still answers `Available` with an empty
    /// inventory; [`InputSnapshot::present`] is what hides the item then,
    /// exactly as the battery item hides with no battery.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(InputSnapshot::from_data(&data));
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
    pub fn snapshot(&self) -> Option<&InputSnapshot> {
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

impl<S: InputSource> Adapter for InputAdapter<S> {
    type Snapshot = InputSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::INPUT
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
    use crate::source::{InputData, InputDeviceData, MockInput};

    fn data() -> InputData {
        InputData {
            devices: vec![
                InputDeviceData {
                    name: "AT keyboard".to_owned(),
                    kernel: "/dev/input/event3".to_owned(),
                    capabilities: vec!["keyboard".to_owned()],
                    ..InputDeviceData::default()
                },
                InputDeviceData {
                    name: "Optical Mouse".to_owned(),
                    kernel: "/dev/input/event7".to_owned(),
                    capabilities: vec!["pointer".to_owned()],
                    ..InputDeviceData::default()
                },
            ],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = InputAdapter::new(MockInput::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::INPUT).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = InputAdapter::new(MockInput::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().count(), 2);
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
        let mut adapter = InputAdapter::new(MockInput::present(data()));
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
    fn a_stack_with_no_devices_is_available_but_absent_at_the_snapshot() {
        let mut adapter = InputAdapter::new(MockInput::present(InputData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "No input devices");
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = InputAdapter::new(MockInput::failing("libinput: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "libinput: timeout"
        );
    }

    #[test]
    fn a_device_change_shows_up_on_the_next_read() {
        let mut adapter = InputAdapter::new(MockInput::present(data()));
        adapter.refresh();
        let _ = adapter.drain_events();
        let first = adapter.snapshot().cloned().unwrap();

        let mut next = data();
        next.devices.remove(0); // unplug the keyboard
        adapter.source_mut().push(next);
        adapter.refresh();
        let second = adapter.snapshot().unwrap();
        let changes = second.device_changes(&first);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_removed());
        assert_eq!(changes[0].name(), "AT keyboard");
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    }

    #[test]
    fn the_adapter_is_read_only() {
        // No write methods exist: a compile-time contract, documented here so a
        // later task that wants writes has to add them deliberately.
        let _ = InputAdapter::new(MockInput::present(data()));
    }
}
