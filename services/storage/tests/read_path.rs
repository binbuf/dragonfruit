// SPDX-License-Identifier: MIT
//! T-15.2a acceptance: the adapter reads a mocked UDisks2 and reports drives,
//! volumes, and mount state; mount/unmount/eject are explicit writes; and an
//! absent daemon is a normal hidden state. No bus and no daemon are involved.

use dragonfruit_storage::{
    DbusUDisks, MockStorage, StorageAdapter, StorageData, StorageOutcome, StorageSnapshot,
    StorageSource,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live D-Bus source builds
/// from `GetManagedObjects`.
fn fixture() -> StorageData {
    serde_json::from_str(include_str!("fixtures/udisks-workstation.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_drives_and_volumes() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("UDisks2 answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.glyph(), "drive-harddisk");

    // Drives sort by path: the internal NVMe then the USB stick.
    assert_eq!(snapshot.drives().len(), 2);
    assert_eq!(snapshot.drives()[0].display_name(), "NVMe SSD");
    assert!(!snapshot.drives()[0].is_removable());
    assert_eq!(snapshot.drives()[1].display_name(), "Flash Drive");
    assert!(snapshot.drives()[1].is_removable());
    assert!(snapshot.drives()[1].can_eject());
    assert_eq!(snapshot.drives()[1].glyph(), "drive-removable-media");

    // Ignorable loop devices and non-mountable part-tables are dropped; the
    // three real filesystems sort mounted first, removable next, then name.
    assert_eq!(snapshot.volumes().len(), 3);
    assert_eq!(snapshot.volumes()[0].display_name(), "Backup");
    assert!(snapshot.volumes()[0].is_mounted());
    assert_eq!(
        snapshot.volumes()[0].mount_point(),
        Some("/run/media/user/Backup")
    );
    assert!(snapshot.volumes()[0].is_removable());
    assert!(snapshot.volumes()[0].ejectable);
    assert_eq!(snapshot.volumes()[0].drive_name, "Flash Drive");
    assert_eq!(snapshot.volumes()[1].display_name(), "Root");
    assert!(snapshot.volumes()[1].is_system());
    assert!(!snapshot.volumes()[1].is_removable());
    assert_eq!(snapshot.volumes()[2].display_name(), "Photos");
    assert!(!snapshot.volumes()[2].is_mounted());
    assert!(snapshot.volumes()[2].is_removable());

    assert_eq!(snapshot.mounted_count(), 2);
    assert_eq!(snapshot.label(), "Storage 2 mounted");
    assert_eq!(snapshot.mounted_volumes().len(), 2);
    assert_eq!(snapshot.removable_volumes().len(), 2);
    assert_eq!(snapshot.internal_volumes().len(), 1);
    assert!(
        snapshot
            .volume_by_device("/dev/sdb1")
            .map(|volume| volume.label.as_str())
            == Some("Backup")
    );
    assert!(snapshot
        .volume_by_path("/org/freedesktop/UDisks2/block_devices/loop0")
        .is_none());

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::STORAGE);
    assert_eq!(slot.id, AdapterId::STORAGE);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::STORAGE);
}

#[test]
fn an_absent_udisks_hides_the_item_and_never_errors() {
    let mut adapter = StorageAdapter::new(MockStorage::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::STORAGE);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_running_daemon_with_no_volumes_has_nothing_to_show() {
    // No storage hardware: UDisks2 is up, but there is no volume. The adapter
    // is available; `present()` is what tells the shell to hide the item.
    let mut adapter = StorageAdapter::new(MockStorage::present(StorageData::default()));
    adapter.refresh();

    assert!(adapter.state().is_available());
    assert!(!adapter.snapshot().unwrap().present());
    assert_eq!(adapter.snapshot().unwrap().label(), "Storage unavailable");
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = StorageAdapter::new(MockStorage::failing("UDisks: timeout"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::STORAGE);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("UDisks: timeout"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));
    adapter.refresh();
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // Mask the daemon: absence hides the item, not an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::STORAGE).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::STORAGE).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert!(adapter.snapshot().unwrap().present());
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: true },
            AdapterEvent::Changed,
        ]
    );
}

#[test]
fn mount_unmount_and_eject_are_explicit_writes_that_do_not_invent_a_snapshot() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));
    adapter.refresh();
    let photos = "/org/freedesktop/UDisks2/block_devices/sdb2";
    let stick = "/org/freedesktop/UDisks2/drives/USB_Stick";
    assert!(!adapter
        .snapshot()
        .unwrap()
        .volume_by_path(photos)
        .unwrap()
        .is_mounted());

    // Each action is one write; the snapshot only changes when UDisks2 pushes
    // and the host re-reads.
    assert!(adapter.mount(photos).is_accepted());
    assert!(adapter.unmount(photos).is_accepted());
    assert!(adapter.eject(stick).is_accepted());
    assert_eq!(adapter.source().writes(), 3);
    assert!(!adapter
        .snapshot()
        .unwrap()
        .volume_by_path(photos)
        .unwrap()
        .is_mounted());

    // UDisks2 pushes the resulting state and the host re-reads.
    let mut pushed = fixture();
    pushed.volumes[1].mount_points = vec!["/run/media/user/Photos".to_owned()];
    adapter.source_mut().push(pushed);
    adapter.refresh();
    assert_eq!(adapter.snapshot().unwrap().mounted_count(), 3);
    assert_eq!(adapter.snapshot().unwrap().label(), "Storage 3 mounted");
}

#[test]
fn a_denied_write_is_reported_and_the_read_state_stays_live() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));
    adapter.refresh();
    adapter.source_mut().deny_writes("UDisks: not authorized");

    let outcome = adapter.mount("/org/freedesktop/UDisks2/block_devices/sdb2");
    assert_eq!(
        outcome,
        StorageOutcome::Denied("UDisks: not authorized".to_owned())
    );
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::STORAGE).visible);
}

#[test]
fn a_busy_device_failure_is_not_a_denial() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));
    adapter.refresh();
    adapter.source_mut().fail_writes("UDisks: device is busy");

    let outcome = adapter.unmount("/org/freedesktop/UDisks2/block_devices/sdb1");
    assert_eq!(outcome.denial_note(), None);
    assert!(matches!(outcome, StorageOutcome::Failed(_)));
    assert!(adapter.state().is_available());
}

#[test]
fn the_adapter_reads_once_per_refresh_never_in_a_poll() {
    let mut adapter = StorageAdapter::new(MockStorage::present(fixture()));
    assert_eq!(adapter.source().reads(), 0);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 1);

    // Re-reading the state is free: a consumer never touches the daemon.
    let _ = adapter.state();
    let _ = adapter.snapshot();
    assert_eq!(adapter.source().reads(), 1);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 2);
}

#[test]
fn the_live_udisks_reads_when_a_session_is_present() {
    // The live D-Bus path is exercised only where a UDisks2 daemon exists (the
    // test machine); CI without one reports absence and skips.
    let mut source = DbusUDisks::new();
    match source.read() {
        Ok(None) => {
            // No UDisks2 here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = StorageSnapshot::from_data(&data);
            for drive in snapshot.drives() {
                assert!(!drive.path.is_empty());
            }
            for volume in snapshot.volumes() {
                assert!(!volume.path.is_empty());
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
