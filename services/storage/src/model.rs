// SPDX-License-Identifier: MIT
//! The storage snapshot the pane and status slot render, decoded from one raw
//! read.
//!
//! The model owns the aggregation a consumer should not repeat: it joins each
//! block object to its drive, drops the objects UDisks2 marks as ignorable or
//! that carry no filesystem, orders the volumes usefully, and derives the
//! display name and glyph the UI draws. It maps no UDisks2 enums — UDisks2
//! reports strings and booleans directly — so it is a straight projection of
//! the raw objects with a stable ordering.
//!
//! A filesystem that UDisks2 reports but that carries no mount point is still
//! a present volume: the pane shows it with a Mount action. Only *no mountable
//! volume at all* (no hardware, UDisks2 gone) makes
//! [`StorageSnapshot::present`] false, which is the second hide rule beside
//! the adapter-level `Unavailable`.

use crate::source::{StorageData, StorageDriveData, StorageVolumeData};

/// A storage drive the session reports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageDrive {
    /// The object path, e.g. `/org/freedesktop/UDisks2/drives/USB_Stick`.
    pub path: String,
    /// UDisks2's stable identifier for the drive.
    pub id: String,
    /// The drive vendor.
    pub vendor: String,
    /// The drive model.
    pub model: String,
    /// The drive serial number.
    pub serial: String,
    /// The media kind (`flash`, `optical`, `disk`, …).
    pub media: String,
    /// The bus the drive hangs off (`usb`, `sata`, …).
    pub connection_bus: String,
    /// `Removable`: the drive can be removed while powered.
    pub removable: bool,
    /// `Ejectable`: the drive supports `Eject`.
    pub ejectable: bool,
    /// `Optical`: the drive has optical media.
    pub optical: bool,
    /// `MediaAvailable`: media is present (an empty card reader is not).
    pub media_available: bool,
    /// `Size` in bytes.
    pub size: u64,
}

impl StorageDrive {
    fn from_data(data: &StorageDriveData) -> Self {
        StorageDrive {
            path: data.path.clone(),
            id: data.id.clone(),
            vendor: data.vendor.clone(),
            model: data.model.clone(),
            serial: data.serial.clone(),
            media: data.media.clone(),
            connection_bus: data.connection_bus.clone(),
            removable: data.removable,
            ejectable: data.ejectable,
            optical: data.optical,
            media_available: data.media_available,
            size: data.size,
        }
    }

    /// The name to show: the model, else the vendor, else the id.
    pub fn display_name(&self) -> &str {
        first_non_empty(&[&self.model, &self.vendor, &self.id])
    }

    /// Whether the drive can be removed while powered (removable media).
    pub fn is_removable(&self) -> bool {
        self.removable
    }

    /// Whether the drive has optical media.
    pub fn is_optical(&self) -> bool {
        self.optical
    }

    /// Whether the drive offers an Eject action.
    pub fn can_eject(&self) -> bool {
        self.ejectable
    }

    /// The glyph the UI draws for the drive.
    pub fn glyph(&self) -> &'static str {
        if self.optical {
            "drive-optical"
        } else if self.removable {
            "drive-removable-media"
        } else {
            "drive-harddisk"
        }
    }
}

/// One mountable volume (a UDisks2 block object with a filesystem).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageVolume {
    /// The object path, e.g. `/org/freedesktop/UDisks2/block_devices/sdb1`.
    pub path: String,
    /// The owning drive's object path, or `/` when there is none.
    pub drive_path: String,
    /// The device node, e.g. `/dev/sdb1`.
    pub device: String,
    /// The node UDisks2 prefers for display.
    pub preferred_device: String,
    /// UDisks2's stable identifier for the block object.
    pub id: String,
    /// The filesystem label, when one is set.
    pub label: String,
    /// The filesystem UUID, when one is set.
    pub uuid: String,
    /// The filesystem type (`ext4`, `vfat`, …).
    pub filesystem: String,
    /// What the block object holds (`filesystem`, `crypto`, …).
    pub usage: String,
    /// Size in bytes.
    pub size: u64,
    /// The block device is declared read-only.
    pub read_only: bool,
    /// UDisks2 suggests auto-mounting it.
    pub hint_auto: bool,
    /// It holds the running system (root, `/boot`, swap).
    pub hint_system: bool,
    /// The mount points, empty when unmounted.
    pub mount_points: Vec<String>,
    /// Whether the owning drive is removable.
    pub removable: bool,
    /// Whether the owning drive offers an Eject action.
    pub ejectable: bool,
    /// The owning drive's display name, when it has one.
    pub drive_name: String,
}

impl StorageVolume {
    fn from_data(data: &StorageVolumeData, drive: Option<&StorageDrive>) -> Self {
        StorageVolume {
            path: data.path.clone(),
            drive_path: data.drive_path.clone(),
            device: data.device.clone(),
            preferred_device: data.preferred_device.clone(),
            id: data.id.clone(),
            label: data.label.clone(),
            uuid: data.uuid.clone(),
            filesystem: data.filesystem.clone(),
            usage: data.usage.clone(),
            size: data.size,
            read_only: data.read_only,
            hint_auto: data.hint_auto,
            hint_system: data.hint_system,
            mount_points: data.mount_points.clone(),
            removable: drive.map(StorageDrive::is_removable).unwrap_or(false),
            ejectable: drive.map(StorageDrive::can_eject).unwrap_or(false),
            drive_name: drive
                .map(|drive| drive.display_name().to_owned())
                .unwrap_or_default(),
        }
    }

    /// Whether the volume is mounted (it carries at least one mount point).
    pub fn is_mounted(&self) -> bool {
        !self.mount_points.is_empty()
    }

    /// The first mount point, when the volume is mounted.
    pub fn mount_point(&self) -> Option<&str> {
        self.mount_points.first().map(String::as_str)
    }

    /// Whether the volume lives on removable media.
    pub fn is_removable(&self) -> bool {
        self.removable
    }

    /// Whether the volume holds the running system.
    pub fn is_system(&self) -> bool {
        self.hint_system
    }

    /// The name to show: the label, else the preferred device's node, else the
    /// device's node, else the UUID.
    pub fn display_name(&self) -> &str {
        first_non_empty(&[
            &self.label,
            basename(&self.preferred_device),
            basename(&self.device),
            &self.uuid,
        ])
    }

    /// The glyph the UI draws for the volume.
    pub fn glyph(&self) -> &'static str {
        if self.removable {
            "drive-removable-media"
        } else {
            "drive-harddisk"
        }
    }
}

/// The storage snapshot a pane or status slot renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageSnapshot {
    /// Every drive UDisks2 reports.
    pub drives: Vec<StorageDrive>,
    /// Every mountable volume, mounted first, removable next, then by name.
    pub volumes: Vec<StorageVolume>,
}

impl StorageSnapshot {
    /// Build the snapshot from one raw read: join each mountable block object
    /// to its drive, drop the ignorable ones, and order the result.
    pub fn from_data(data: &StorageData) -> Self {
        let drives: Vec<StorageDrive> = data.drives.iter().map(StorageDrive::from_data).collect();

        let mut volumes: Vec<StorageVolume> = data
            .volumes
            .iter()
            // A block object with no Filesystem interface is not a volume a
            // pane can mount; `HintIgnore` is UDisks2 telling UIs to drop it.
            .filter(|volume| volume.mountable && !volume.hint_ignore)
            .map(|volume| {
                let drive = drives.iter().find(|drive| drive.path == volume.drive_path);
                StorageVolume::from_data(volume, drive)
            })
            .collect();
        volumes.sort_by(|left, right| {
            right
                .is_mounted()
                .cmp(&left.is_mounted())
                .then_with(|| right.is_removable().cmp(&left.is_removable()))
                .then_with(|| left.display_name().cmp(right.display_name()))
                .then_with(|| left.path.cmp(&right.path))
        });

        StorageSnapshot { drives, volumes }
    }

    /// Every drive.
    pub fn drives(&self) -> &[StorageDrive] {
        &self.drives
    }

    /// Every mountable volume.
    pub fn volumes(&self) -> &[StorageVolume] {
        &self.volumes
    }

    /// The mounted volumes.
    pub fn mounted_volumes(&self) -> Vec<&StorageVolume> {
        self.volumes
            .iter()
            .filter(|volume| volume.is_mounted())
            .collect()
    }

    /// The volumes on removable media.
    pub fn removable_volumes(&self) -> Vec<&StorageVolume> {
        self.volumes
            .iter()
            .filter(|volume| volume.is_removable())
            .collect()
    }

    /// The volumes that do not live on removable media (internal disks).
    pub fn internal_volumes(&self) -> Vec<&StorageVolume> {
        self.volumes
            .iter()
            .filter(|volume| !volume.is_removable())
            .collect()
    }

    /// How many volumes are mounted right now.
    pub fn mounted_count(&self) -> usize {
        self.volumes
            .iter()
            .filter(|volume| volume.is_mounted())
            .count()
    }

    /// Whether there is at least one mountable volume to show. The item hides
    /// when there is not, even if UDisks2 is present.
    pub fn present(&self) -> bool {
        !self.volumes.is_empty()
    }

    /// The volume with `path`, when it is in the snapshot.
    pub fn volume_by_path(&self, path: &str) -> Option<&StorageVolume> {
        self.volumes.iter().find(|volume| volume.path == path)
    }

    /// The volume whose device node is `device`.
    pub fn volume_by_device(&self, device: &str) -> Option<&StorageVolume> {
        self.volumes
            .iter()
            .find(|volume| volume.device == device || volume.preferred_device == device)
    }

    /// The drive with `path`, when it is in the snapshot.
    pub fn drive_by_path(&self, path: &str) -> Option<&StorageDrive> {
        self.drives.iter().find(|drive| drive.path == path)
    }

    /// The drive that owns `volume`, when the snapshot has it.
    pub fn drive_for(&self, volume: &StorageVolume) -> Option<&StorageDrive> {
        self.drive_by_path(&volume.drive_path)
    }

    /// The glyph the status slot draws.
    pub fn glyph(&self) -> &'static str {
        "drive-harddisk"
    }

    /// A one-line label for the status item / pane header.
    pub fn label(&self) -> String {
        if !self.present() {
            return "Storage unavailable".to_owned();
        }
        let mounted = self.mounted_count();
        if mounted > 0 {
            return format!("Storage {mounted} mounted");
        }
        "Storage".to_owned()
    }
}

/// The first non-empty string in `candidates`, or `""`.
fn first_non_empty<'a>(candidates: &[&'a str]) -> &'a str {
    candidates
        .iter()
        .copied()
        .find(|candidate| !candidate.is_empty())
        .unwrap_or("")
}

/// The last non-empty path component of `path`.
fn basename(path: &str) -> &str {
    path.rsplit('/').find(|part| !part.is_empty()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(path: &str, removable: bool) -> StorageDriveData {
        StorageDriveData {
            path: path.to_owned(),
            id: "USB_STICK".to_owned(),
            vendor: "Acme".to_owned(),
            model: "Flash Drive".to_owned(),
            connection_bus: "usb".to_owned(),
            removable,
            ejectable: removable,
            media_available: true,
            size: 16_000_000_000,
            ..StorageDriveData::default()
        }
    }

    fn volume(path: &str, drive_path: &str, label: &str, mounted: bool) -> StorageVolumeData {
        StorageVolumeData {
            path: path.to_owned(),
            drive_path: drive_path.to_owned(),
            device: format!("/dev/{}", basename(path)),
            preferred_device: format!("/dev/{}", basename(path)),
            id: "id".to_owned(),
            label: label.to_owned(),
            uuid: "abcd-1234".to_owned(),
            filesystem: "vfat".to_owned(),
            usage: "filesystem".to_owned(),
            size: 16_000_000_000,
            mountable: true,
            mount_points: if mounted {
                vec![format!("/run/media/user/{label}")]
            } else {
                vec![]
            },
            ..StorageVolumeData::default()
        }
    }

    #[test]
    fn an_empty_read_has_no_volumes() {
        let snapshot = StorageSnapshot::from_data(&StorageData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.glyph(), "drive-harddisk");
        assert_eq!(snapshot.label(), "Storage unavailable");
        assert_eq!(snapshot.mounted_count(), 0);
    }

    #[test]
    fn volumes_join_to_their_drive_and_order_mounted_first() {
        let data = StorageData {
            drives: vec![drive("/org/freedesktop/UDisks2/drives/usb", true)],
            volumes: vec![
                volume(
                    "/b/backup",
                    "/org/freedesktop/UDisks2/drives/usb",
                    "Backup",
                    false,
                ),
                volume(
                    "/b/photos",
                    "/org/freedesktop/UDisks2/drives/usb",
                    "Photos",
                    true,
                ),
            ],
        };
        let snapshot = StorageSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert_eq!(snapshot.volumes().len(), 2);
        // Mounted first.
        assert_eq!(snapshot.volumes()[0].display_name(), "Photos");
        assert!(snapshot.volumes()[0].is_mounted());
        assert_eq!(
            snapshot.volumes()[0].mount_point(),
            Some("/run/media/user/Photos")
        );
        assert_eq!(snapshot.volumes()[0].drive_name, "Flash Drive");
        assert!(snapshot.volumes()[0].is_removable());
        assert!(snapshot.volumes()[0].ejectable);
        assert_eq!(snapshot.mounted_volumes().len(), 1);
        assert_eq!(snapshot.removable_volumes().len(), 2);
        assert_eq!(snapshot.internal_volumes().len(), 0);
        assert_eq!(
            snapshot.drive_for(&snapshot.volumes()[0]).unwrap().path,
            "/org/freedesktop/UDisks2/drives/usb"
        );
        assert_eq!(snapshot.label(), "Storage 1 mounted");
    }

    #[test]
    fn ignorable_and_unmountable_blocks_are_dropped() {
        let mut ignored = volume(
            "/b/ignore",
            "/org/freedesktop/UDisks2/drives/usb",
            "Ignore",
            false,
        );
        ignored.hint_ignore = true;
        let mut parttable = volume("/b/disk", "", "Disk", false);
        parttable.mountable = false;
        parttable.usage = "parttable".to_owned();
        let data = StorageData {
            drives: vec![],
            volumes: vec![volume("/b/keep", "", "Keep", false), ignored, parttable],
        };
        let snapshot = StorageSnapshot::from_data(&data);
        assert_eq!(snapshot.volumes().len(), 1);
        assert_eq!(snapshot.volumes()[0].display_name(), "Keep");
        // A volume with no resolvable drive is present but not removable.
        assert!(!snapshot.volumes()[0].is_removable());
    }

    #[test]
    fn a_volume_without_a_label_falls_back_to_its_device() {
        let mut raw = volume("/b/nodev", "", "", false);
        raw.label = String::new();
        raw.device = "/dev/sdc1".to_owned();
        raw.preferred_device = String::new();
        let snapshot = StorageSnapshot::from_data(&StorageData {
            drives: vec![],
            volumes: vec![raw],
        });
        assert_eq!(snapshot.volumes()[0].display_name(), "sdc1");
        assert_eq!(snapshot.glyph(), "drive-harddisk");
    }

    #[test]
    fn internal_disks_are_not_removable_and_system_volumes_are_flagged() {
        let data = StorageData {
            drives: vec![drive("/org/freedesktop/UDisks2/drives/nvme", false)],
            volumes: vec![volume(
                "/b/root",
                "/org/freedesktop/UDisks2/drives/nvme",
                "Root",
                true,
            )],
        };
        let snapshot = StorageSnapshot::from_data(&data);
        let root = snapshot.volume_by_path("/b/root").unwrap();
        assert!(!root.is_removable());
        assert!(!root.ejectable);
        assert_eq!(root.glyph(), "drive-harddisk");
        assert_eq!(snapshot.removable_volumes().len(), 0);
        assert_eq!(snapshot.internal_volumes().len(), 1);
    }

    #[test]
    fn a_volume_is_findable_by_path_and_device() {
        let data = StorageData {
            drives: vec![],
            volumes: vec![volume("/b/photos", "", "Photos", true)],
        };
        let snapshot = StorageSnapshot::from_data(&data);
        assert!(snapshot.volume_by_path("/b/photos").is_some());
        assert!(snapshot.volume_by_device("/dev/photos").is_some());
        assert!(snapshot.volume_by_path("/b/missing").is_none());
    }
}
