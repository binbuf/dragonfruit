// SPDX-License-Identifier: MIT
//! The Storage half of the bridge host (T-15.2b).
//!
//! UDisks2 is reached by the `dragonfruit-storage` adapter; the shell and the
//! Settings app never link it. This module owns the one projection from the
//! typed [`StorageSnapshot`] to the flat JSON view the two consumers draw,
//! plus the three explicit writes the pane and tile raise (mount, unmount,
//! eject). It mirrors the Bluetooth half in [`crate::storage`], but UDisks2 is
//! on the **system** bus and its adapter is not one of the three menu-bar
//! adapters (ADR 0119/0120).
//!
//! A write never invents a snapshot: the host calls
//! [`StorageAdapter::refresh`](dragonfruit_storage::StorageAdapter::refresh)
//! after a successful action (or when UDisks2 signals), and the read state
//! stays the single source of truth.

use dragonfruit_storage::{StorageAdapter, StorageOutcome, StorageSnapshot, StorageSource};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_STORAGE: &str = "storage";

/// The bridge host for the storage adapter: one state path and the explicit
/// mount/unmount/eject writes the Settings pane and Control Center tile offer.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageHost<S> {
    adapter: StorageAdapter<S>,
}

impl<S: StorageSource> StorageHost<S> {
    /// A host over a storage source.
    pub fn new(source: S) -> Self {
        StorageHost {
            adapter: StorageAdapter::new(source),
        }
    }

    /// Re-read UDisks2 once. Called on startup, when UDisks2 signals, and
    /// after an explicit action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The storage view the pane and tile render.
    pub fn view(&self) -> Value {
        storage_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Mount the volume at `volume_path`. One explicit write.
    pub fn mount(&mut self, volume_path: &str) -> Value {
        storage_report(self.adapter.mount(volume_path))
    }

    /// Unmount the volume at `volume_path`. One explicit write.
    pub fn unmount(&mut self, volume_path: &str) -> Value {
        storage_report(self.adapter.unmount(volume_path))
    }

    /// Eject the drive at `drive_path`. One explicit write.
    pub fn eject(&mut self, drive_path: &str) -> Value {
        storage_report(self.adapter.eject(drive_path))
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &StorageAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut StorageAdapter<S> {
        &mut self.adapter
    }
}

/// Build the storage view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the live
/// state. A daemon with no mountable volume is still `available` with
/// `present: false`, and the consumer hides the item then too — exactly the
/// battery's and Bluetooth's second hide rule.
pub fn storage_view<S: StorageSource>(adapter: &StorageAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_STORAGE, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_STORAGE,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_STORAGE, "state": "unavailable" });
    };
    storage_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn storage_snapshot_view(snapshot: &StorageSnapshot) -> Value {
    let drives: Vec<Value> = snapshot.drives().iter().map(drive_view).collect();
    let volumes: Vec<Value> = snapshot.volumes().iter().map(volume_view).collect();
    json!({
        "kind": KIND_STORAGE,
        "state": "available",
        "present": snapshot.present(),
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "mountedCount": snapshot.mounted_count(),
        "volumeCount": volumes.len(),
        "removableCount": snapshot.removable_volumes().len(),
        "drives": drives,
        "volumes": volumes,
    })
}

fn drive_view(drive: &dragonfruit_storage::StorageDrive) -> Value {
    json!({
        "path": drive.path,
        "id": drive.id,
        "name": drive.display_name(),
        "vendor": drive.vendor,
        "model": drive.model,
        "media": drive.media,
        "connectionBus": drive.connection_bus,
        "removable": drive.removable,
        "ejectable": drive.ejectable,
        "optical": drive.optical,
        "mediaAvailable": drive.media_available,
        "size": drive.size,
        "glyph": drive.glyph(),
    })
}

fn volume_view(volume: &dragonfruit_storage::StorageVolume) -> Value {
    json!({
        "path": volume.path,
        "drivePath": volume.drive_path,
        "device": volume.device,
        "name": volume.display_name(),
        "label": volume.label,
        "filesystem": volume.filesystem,
        "size": volume.size,
        "mounted": volume.is_mounted(),
        "mountPoint": volume.mount_point(),
        "removable": volume.removable,
        "system": volume.is_system(),
        "ejectable": volume.ejectable,
        "readOnly": volume.read_only,
        "hintAuto": volume.hint_auto,
        "driveName": volume.drive_name,
        "glyph": volume.glyph(),
    })
}

/// The JSON report for a storage write (`accepted`/`denied`/`absent`/
/// `failed`), shaped like the Wi-Fi/Bluetooth reports so one shell decode path
/// reads every action.
pub fn storage_report(outcome: StorageOutcome) -> Value {
    match outcome {
        StorageOutcome::Accepted => json!({ "outcome": "accepted" }),
        StorageOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        StorageOutcome::Absent => json!({ "outcome": "absent" }),
        StorageOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_storage::{MockStorage, StorageData, StorageDriveData, StorageVolumeData};

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
    fn an_absent_daemon_projects_a_hidden_slot() {
        let mut host = StorageHost::new(MockStorage::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "storage");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_daemon_projects_drives_and_volumes() {
        let mut host = StorageHost::new(MockStorage::present(data(true)));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], true);
        assert_eq!(view["glyph"], "drive-harddisk");
        assert_eq!(view["mountedCount"], 1);
        assert_eq!(view["volumeCount"], 1);
        assert_eq!(view["removableCount"], 1);
        assert_eq!(view["volumes"][0]["name"], "Photos");
        assert_eq!(view["volumes"][0]["mounted"], true);
        assert_eq!(view["volumes"][0]["mountPoint"], "/run/media/user/Photos");
        assert_eq!(view["volumes"][0]["removable"], true);
        assert_eq!(view["volumes"][0]["ejectable"], true);
        assert_eq!(view["drives"][0]["name"], "Flash Drive");
        assert_eq!(view["drives"][0]["removable"], true);
    }

    #[test]
    fn a_daemon_with_no_volumes_is_available_but_not_present() {
        let mut host = StorageHost::new(MockStorage::present(StorageData::default()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], false);
        assert_eq!(view["label"], "Storage unavailable");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = StorageHost::new(MockStorage::failing("UDisks: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "UDisks: timeout");
    }

    #[test]
    fn writes_apply_once_through_the_adapter() {
        let mut host = StorageHost::new(MockStorage::present(data(false)));
        host.refresh();
        assert_eq!(host.mount("/b")["outcome"], "accepted");
        assert_eq!(host.unmount("/b")["outcome"], "accepted");
        assert_eq!(host.eject("/d")["outcome"], "accepted");
        assert_eq!(host.adapter().source().writes(), 3);
        // A write invents no snapshot; the pushed state is the truth.
        assert!(!host.adapter().snapshot().unwrap().volumes()[0].is_mounted());
    }

    #[test]
    fn a_denial_and_a_write_while_absent_are_reported() {
        let mut host = StorageHost::new(MockStorage::present(data(false)));
        host.refresh();
        host.adapter_mut()
            .source_mut()
            .deny_writes("UDisks: not authorized");
        assert_eq!(host.mount("/b")["outcome"], "denied");
        assert_eq!(host.mount("/b")["note"], "UDisks: not authorized");
        // The read state stays live after a denial.
        assert_eq!(host.view()["state"], "available");

        let mut absent = StorageHost::new(MockStorage::absent());
        assert_eq!(absent.mount("/b")["outcome"], "absent");
        assert_eq!(absent.view()["state"], "unavailable");
    }
}
