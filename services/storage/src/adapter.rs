// SPDX-License-Identifier: MIT
//! The storage adapter: one state path and its explicit writes over the shared
//! contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::StorageSnapshot;
use crate::source::{StorageOutcome, StorageSource};

/// The storage status adapter.
///
/// It holds the last snapshot UDisks2 pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the daemon; the host calls it when UDisks2 signals a change, so
/// nothing above the adapter polls.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a
/// session booted without UDisks2 renders a hidden item and never blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageAdapter<S> {
    source: S,
    state: AdapterState<StorageSnapshot>,
    subscription: Subscription,
}

impl<S: StorageSource> StorageAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        StorageAdapter {
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
    /// A daemon with no mountable volume still answers `Available` with an
    /// empty snapshot; [`StorageSnapshot::present`] is what hides the item
    /// then, exactly as the Wi-Fi item hides with no Wi-Fi device.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(StorageSnapshot::from_data(&data));
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

    /// Mount the volume at `volume_path`. One explicit write; a successful call
    /// changes nothing here — UDisks2 pushes the new mount points and the host
    /// re-reads.
    pub fn mount(&mut self, volume_path: &str) -> StorageOutcome {
        let outcome = self.source.mount(volume_path);
        self.after_write(outcome)
    }

    /// Unmount the volume at `volume_path`. One explicit write.
    pub fn unmount(&mut self, volume_path: &str) -> StorageOutcome {
        let outcome = self.source.unmount(volume_path);
        self.after_write(outcome)
    }

    /// Eject the drive at `drive_path`. One explicit write.
    pub fn eject(&mut self, drive_path: &str) -> StorageOutcome {
        let outcome = self.source.eject(drive_path);
        self.after_write(outcome)
    }

    /// A write against an absent daemon is absence at the state level too: the
    /// item hides and the subscription drops, never an error.
    fn after_write(&mut self, outcome: StorageOutcome) -> StorageOutcome {
        if matches!(outcome, StorageOutcome::Absent) {
            self.state = AdapterState::Unavailable;
            self.subscription.absent();
        }
        outcome
    }

    /// The live snapshot, when the daemon answered.
    pub fn snapshot(&self) -> Option<&StorageSnapshot> {
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

impl<S: StorageSource> Adapter for StorageAdapter<S> {
    type Snapshot = StorageSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::STORAGE
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
    use crate::source::{MockStorage, StorageData, StorageDriveData, StorageVolumeData};

    fn data(mounted: bool) -> StorageData {
        StorageData {
            drives: vec![StorageDriveData {
                path: "/org/freedesktop/UDisks2/drives/usb".to_owned(),
                id: "USB".to_owned(),
                model: "Flash Drive".to_owned(),
                removable: true,
                ejectable: true,
                media_available: true,
                ..StorageDriveData::default()
            }],
            volumes: vec![StorageVolumeData {
                path: "/org/freedesktop/UDisks2/block_devices/sdb1".to_owned(),
                drive_path: "/org/freedesktop/UDisks2/drives/usb".to_owned(),
                device: "/dev/sdb1".to_owned(),
                preferred_device: "/dev/sdb1".to_owned(),
                label: "Photos".to_owned(),
                uuid: "1234-ABCD".to_owned(),
                filesystem: "vfat".to_owned(),
                usage: "filesystem".to_owned(),
                mountable: true,
                mount_points: if mounted {
                    vec!["/run/media/user/Photos".to_owned()]
                } else {
                    vec![]
                },
                ..StorageVolumeData::default()
            }],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = StorageAdapter::new(MockStorage::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::STORAGE).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = StorageAdapter::new(MockStorage::present(data(true)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().mounted_count(), 1);
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
        let mut adapter = StorageAdapter::new(MockStorage::present(data(true)));
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
    fn a_daemon_with_no_volumes_is_available_but_absent_at_the_snapshot() {
        let mut adapter = StorageAdapter::new(MockStorage::present(StorageData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "Storage unavailable");
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = StorageAdapter::new(MockStorage::failing("UDisks: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
    }

    #[test]
    fn writes_pass_through_and_do_not_invent_a_snapshot() {
        let mut adapter = StorageAdapter::new(MockStorage::present(data(false)));
        adapter.refresh();

        assert!(adapter.mount("/b").is_accepted());
        assert!(adapter.unmount("/b").is_accepted());
        assert!(adapter.eject("/d").is_accepted());

        // Still the old snapshot until UDisks2 pushes and the host re-reads.
        assert!(!adapter.snapshot().unwrap().volumes()[0].is_mounted());
        assert_eq!(adapter.source().writes(), 3);
    }

    #[test]
    fn a_denial_is_reported_and_leaves_the_read_state_live() {
        let mut adapter = StorageAdapter::new(MockStorage::present(data(false)));
        adapter.refresh();
        adapter.source_mut().deny_writes("UDisks: not authorized");

        let outcome = adapter.mount("/b");
        assert_eq!(outcome.denial_note(), Some("UDisks: not authorized"));
        assert!(adapter.state().is_available());
        assert!(adapter.state().slot(AdapterId::STORAGE).visible);
    }

    #[test]
    fn a_write_against_an_absent_daemon_hides_the_item() {
        let mut adapter = StorageAdapter::new(MockStorage::absent());
        let outcome = adapter.mount("/b");
        assert_eq!(outcome, StorageOutcome::Absent);
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::STORAGE).visible);
    }
}
