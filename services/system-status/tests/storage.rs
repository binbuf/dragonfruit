// SPDX-License-Identifier: MIT
//! Bridge tests for the storage half of the host (T-15.2b): the view the
//! Settings pane and Control Center tile decode, plus the three explicit
//! writes applied once through the adapter.

use dragonfruit_storage::{MockStorage, StorageData, StorageDriveData, StorageVolumeData};
use dragonfruit_system_status::StorageHost;

fn data(mounted: bool) -> StorageData {
    StorageData {
        drives: vec![StorageDriveData {
            path: "/org/freedesktop/UDisks2/drives/usb".to_owned(),
            id: "USB".to_owned(),
            model: "Flash Drive".to_owned(),
            removable: true,
            ejectable: true,
            media_available: true,
            size: 16_000_000_000,
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
fn the_storage_view_exposes_drives_and_volumes() {
    let mut host = StorageHost::new(MockStorage::present(data(true)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "storage");
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], true);
    assert_eq!(view["mountedCount"], 1);
    assert_eq!(view["volumeCount"], 1);
    assert_eq!(view["volumes"][0]["name"], "Photos");
    assert_eq!(view["volumes"][0]["mounted"], true);
    assert_eq!(view["volumes"][0]["removable"], true);
    assert_eq!(view["drives"][0]["name"], "Flash Drive");
}

#[test]
fn the_three_writes_apply_once_and_never_invent_a_snapshot() {
    let mut host = StorageHost::new(MockStorage::present(data(false)));
    host.refresh();
    assert_eq!(host.mount("/b")["outcome"], "accepted");
    assert_eq!(host.unmount("/b")["outcome"], "accepted");
    assert_eq!(host.eject("/d")["outcome"], "accepted");
    assert_eq!(host.adapter().source().writes(), 3);
    // The old snapshot stands until UDisks2 pushes and the host re-reads.
    assert!(!host.adapter().snapshot().unwrap().volumes()[0].is_mounted());
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = StorageHost::new(MockStorage::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(host.mount("/b")["outcome"], "absent");
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_denied_write_keeps_the_read_state_live() {
    let mut host = StorageHost::new(MockStorage::present(data(true)));
    host.refresh();
    host.adapter_mut()
        .source_mut()
        .deny_writes("UDisks: not authorized");
    let report = host.mount("/b");
    assert_eq!(report["outcome"], "denied");
    assert_eq!(report["note"], "UDisks: not authorized");
    assert_eq!(host.view()["state"], "available");
}
