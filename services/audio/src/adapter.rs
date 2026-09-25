// SPDX-License-Identifier: MIT
//! The audio adapter: one read path plus the default-sink volume/mute write.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::AudioSnapshot;
use crate::source::{AudioSource, SetOutcome};

/// The volume/mute status adapter.
///
/// It holds the last snapshot the daemon pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the daemon; the host calls it when WirePlumber signals a change, so
/// nothing above the adapter polls.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a
/// session booted without PipeWire renders a hidden item and never blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioAdapter<S> {
    source: S,
    state: AdapterState<AudioSnapshot>,
    subscription: Subscription,
}

impl<S: AudioSource> AudioAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        AudioAdapter {
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
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(AudioSnapshot::from_data(&data));
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

    /// The live snapshot, when the daemon answered.
    pub fn snapshot(&self) -> Option<&AudioSnapshot> {
        self.state.snapshot()
    }

    /// Set the default sink's linear volume (0..=1). The one volume write.
    ///
    /// A write that lands changes nothing here yet: WirePlumber reports the
    /// resulting state through its subscription, and the host
    /// [`refresh`](Self::refresh)es on that event, so the snapshot stays the
    /// single source of truth. The caller reacts to the [`SetOutcome`] and
    /// re-reads within one event (the acceptance for T-07.3).
    ///
    /// An absent daemon hides the item; a write failure leaves the read state
    /// untouched.
    pub fn set_volume(&mut self, volume: f32) -> SetOutcome {
        let outcome = self.source.set_volume(volume);
        self.note_write(&outcome);
        outcome
    }

    /// Set the default sink's mute state. The one mute write; otherwise the
    /// same contract as [`set_volume`](Self::set_volume).
    pub fn set_mute(&mut self, muted: bool) -> SetOutcome {
        let outcome = self.source.set_mute(muted);
        self.note_write(&outcome);
        outcome
    }

    /// Flip the default sink's mute state. The Control Center/OSD affordance.
    pub fn toggle_mute(&mut self) -> SetOutcome {
        let muted = self
            .snapshot()
            .map(|snapshot| snapshot.muted())
            .unwrap_or(false);
        self.set_mute(!muted)
    }

    /// Update the subscription/state on an absent write, without inventing a
    /// snapshot for a successful or failed one.
    fn note_write(&mut self, outcome: &SetOutcome) {
        if matches!(outcome, SetOutcome::Absent) {
            self.state = AdapterState::Unavailable;
            self.subscription.absent();
        }
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

impl<S: AudioSource> Adapter for AudioAdapter<S> {
    type Snapshot = AudioSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::AUDIO
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
    use crate::source::{AudioData, MockAudio, SinkData};

    fn data(volume: f32, muted: bool) -> AudioData {
        AudioData {
            default_sink: Some("sink".to_owned()),
            sinks: vec![SinkData {
                id: 1,
                name: "sink".to_owned(),
                description: "Speakers".to_owned(),
                volume,
                muted,
            }],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = AudioAdapter::new(MockAudio::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::AUDIO).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().volume(), 0.5);
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
    fn a_volume_write_does_not_invent_a_snapshot() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
        adapter.refresh();
        let _ = adapter.drain_events();

        assert_eq!(adapter.set_volume(0.9), SetOutcome::Applied);
        // The snapshot still holds the last pushed read until a refresh.
        assert_eq!(adapter.snapshot().unwrap().volume(), 0.5);
        assert!(adapter.drain_events().is_empty());

        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().volume(), 0.9);
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
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
    fn a_write_while_absent_hides_the_item() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
        adapter.refresh();
        let _ = adapter.drain_events();
        adapter.source_mut().kill();

        assert_eq!(adapter.set_volume(0.4), SetOutcome::Absent);
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);
    }

    #[test]
    fn toggle_mute_flips_the_last_read() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
        adapter.refresh();
        assert_eq!(adapter.toggle_mute(), SetOutcome::Applied);
        adapter.refresh();
        assert!(adapter.snapshot().unwrap().muted());
        assert_eq!(adapter.toggle_mute(), SetOutcome::Applied);
        adapter.refresh();
        assert!(!adapter.snapshot().unwrap().muted());
    }

    #[test]
    fn a_write_failure_leaves_the_read_state_untouched() {
        let mut adapter = AudioAdapter::new(MockAudio::present(data(0.5, false)));
        adapter.refresh();
        adapter.source_mut().fail_writes("wpctl: boom");

        let outcome = adapter.set_volume(0.2);
        assert!(matches!(outcome, SetOutcome::Failed(_)));
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().volume(), 0.5);
    }
}
