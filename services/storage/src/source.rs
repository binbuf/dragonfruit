// SPDX-License-Identifier: MIT
//! The transport seam: the raw UDisks2 read and its mock.
//!
//! A [`StorageSource`] is the only thing that talks to the daemon. The real
//! source is the D-Bus client ([`crate::DbusUDisks`]); tests and CI use
//! [`MockStorage`], which serves a fixture with no bus on the machine. The
//! adapter ([`crate::StorageAdapter`]) turns one raw read into the typed
//! [`StorageSnapshot`](crate::StorageSnapshot) and drives the shared
//! `Subscription`.
//!
//! UDisks2 is reached over the **system bus** as `org.freedesktop.UDisks2`.
//! One read is its `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at
//! `/org/freedesktop/UDisks2`: every drive (`org.freedesktop.UDisks2.Drive`)
//! and every block device (`org.freedesktop.UDisks2.Block`, with an optional
//! `org.freedesktop.UDisks2.Filesystem`) is a managed object. The raw shape
//! below flattens that into two lists — the daemon's hierarchy stays in
//! [`crate::udisks`], and nothing above the adapter ever sees it.
//!
//! The write halves are explicit user actions, never polls: mount a volume,
//! unmount it, eject the drive that owns it. Each is a single D-Bus method
//! call. A successful write invents no snapshot; UDisks2 pushes the resulting
//! property changes, the host re-reads, and the snapshot stays the single
//! source of truth. UDisks2 does not itself emit ObjectManager signals for
//! every change on every version, so the host may also re-read on its own
//! cadence; the adapter contract is unchanged either way — it never polls.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The raw result of one UDisks2 read, flattened from `GetManagedObjects`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct StorageData {
    /// Every `org.freedesktop.UDisks2.Drive` object.
    #[serde(default)]
    pub drives: Vec<StorageDriveData>,
    /// Every `org.freedesktop.UDisks2.Block` object (whole disks, partitions,
    /// and loop devices alike).
    #[serde(default)]
    pub volumes: Vec<StorageVolumeData>,
}

/// One `org.freedesktop.UDisks2.Drive` object.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct StorageDriveData {
    /// The object path, e.g. `/org/freedesktop/UDisks2/drives/USB_Stick`.
    pub path: String,
    /// `Id`: UDisks2's stable identifier for the drive.
    #[serde(default)]
    pub id: String,
    /// `Vendor`.
    #[serde(default)]
    pub vendor: String,
    /// `Model`.
    #[serde(default)]
    pub model: String,
    /// `Serial`.
    #[serde(default)]
    pub serial: String,
    /// `Media`: the media kind (`flash`, `optical`, `disk`, …).
    #[serde(default)]
    pub media: String,
    /// `ConnectionBus`: the bus the drive hangs off (`usb`, `sata`, …).
    #[serde(default)]
    pub connection_bus: String,
    /// `Removable`: the drive can be removed while powered.
    #[serde(default)]
    pub removable: bool,
    /// `Ejectable`: the drive supports `Eject`.
    #[serde(default)]
    pub ejectable: bool,
    /// `Optical`: the drive has optical media.
    #[serde(default)]
    pub optical: bool,
    /// `MediaAvailable`: media is present (an empty card reader is not).
    #[serde(default)]
    pub media_available: bool,
    /// `Size` in bytes.
    #[serde(default)]
    pub size: u64,
}

/// One `org.freedesktop.UDisks2.Block` object, with its optional filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct StorageVolumeData {
    /// The object path, e.g. `/org/freedesktop/UDisks2/block_devices/sdb1`.
    pub path: String,
    /// `Drive`: the object path of the owning drive, or `/` when there is
    /// none (loop devices, dm-mapper).
    #[serde(default)]
    pub drive_path: String,
    /// `Device`: the device node, e.g. `/dev/sdb1`.
    #[serde(default)]
    pub device: String,
    /// `PreferredDevice`: the node UDisks2 prefers for display.
    #[serde(default)]
    pub preferred_device: String,
    /// `Id`: UDisks2's stable identifier for the block object.
    #[serde(default)]
    pub id: String,
    /// `IdLabel`: the filesystem label, when one is set.
    #[serde(default)]
    pub label: String,
    /// `IdUUID`: the filesystem UUID, when one is set.
    #[serde(default)]
    pub uuid: String,
    /// `IdType`: the filesystem type (`ext4`, `vfat`, …).
    #[serde(default)]
    pub filesystem: String,
    /// `IdUsage`: what the block object holds (`filesystem`, `crypto`,
    /// `parttable`, `other`).
    #[serde(default)]
    pub usage: String,
    /// `Size` in bytes.
    #[serde(default)]
    pub size: u64,
    /// `ReadOnly`: the block device is mounted/declared read-only.
    #[serde(default)]
    pub read_only: bool,
    /// `HintAuto`: user interfaces should offer to auto-mount it.
    #[serde(default)]
    pub hint_auto: bool,
    /// `HintSystem`: it holds the running system (root, `/boot`, swap) and
    /// should not be presented as removable media.
    #[serde(default)]
    pub hint_system: bool,
    /// `HintIgnore`: user interfaces should ignore it entirely.
    #[serde(default)]
    pub hint_ignore: bool,
    /// Whether the block object has an `org.freedesktop.UDisks2.Filesystem`
    /// interface. Only a mountable block object is a volume.
    #[serde(default)]
    pub mountable: bool,
    /// `Filesystem.MountPoints`: the mount points, empty when unmounted.
    #[serde(default)]
    pub mount_points: Vec<String>,
}

/// What happened to one write (mount, unmount, eject).
///
/// A polkit denial is deliberately distinct from a general failure: UDisks2
/// authorizes mounting, unmounting, and ejecting through polkit, and the pane
/// should surface the refusal without treating the daemon as broken. It is
/// never a read error — the adapter can still see the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageOutcome {
    /// UDisks2 accepted the request. The daemon pushes the resulting state;
    /// the host re-reads and the snapshot updates.
    Accepted,
    /// polkit refused the request. `note` records the daemon's message.
    Denied(String),
    /// There is no UDisks2 (or no such volume/drive) to act on.
    Absent,
    /// The request failed for a reason other than authorization (including a
    /// busy device that refuses to unmount).
    Failed(AdapterError),
}

impl StorageOutcome {
    /// Whether the daemon accepted the request.
    pub fn is_accepted(&self) -> bool {
        matches!(self, StorageOutcome::Accepted)
    }

    /// The recorded denial note, when polkit refused.
    pub fn denial_note(&self) -> Option<&str> {
        match self {
            StorageOutcome::Denied(note) => Some(note),
            _ => None,
        }
    }
}

/// Reads UDisks2 over some transport and drives its write methods.
///
/// The read result is a three-way answer, exactly as the adapter contract
/// needs it:
///
/// * `Ok(Some(data))` — the daemon answered; `data` is the live read.
/// * `Ok(None)` — the daemon is absent. A normal state; the slot hides.
/// * `Err(error)` — the daemon is present but the read failed; the slot shows
///   visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`StorageAdapter::refresh`](crate::StorageAdapter::refresh) when UDisks2
/// signals a change (or on its own re-read cadence). Each write is one
/// explicit call, addressed by the object path the snapshot carries.
pub trait StorageSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<StorageData>, AdapterError>;

    /// Mount the volume (Filesystem object) at `volume_path`.
    fn mount(&mut self, volume_path: &str) -> StorageOutcome;

    /// Unmount the volume (Filesystem object) at `volume_path`.
    fn unmount(&mut self, volume_path: &str) -> StorageOutcome;

    /// Eject the drive (Drive object) at `drive_path`.
    fn eject(&mut self, drive_path: &str) -> StorageOutcome;
}

/// How the simulated daemon answers write requests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// polkit denies the request; carries the recorded note.
    Deny(String),
    /// The request fails for another reason.
    Fail(AdapterError),
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`StorageData`] with no bus, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would. By default every write is accepted; `deny_writes`/`fail_writes`
/// drive the refusal paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockStorage {
    present: bool,
    data: Option<StorageData>,
    failure: Option<AdapterError>,
    reads: u32,
    writes: u32,
    behavior: WriteBehavior,
}

impl MockStorage {
    /// A daemon that is not on the bus.
    pub fn absent() -> Self {
        MockStorage {
            present: false,
            data: None,
            failure: None,
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// A present daemon that answers with `data`.
    pub fn present(data: StorageData) -> Self {
        MockStorage {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// A present daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockStorage {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// Make every write come back as a polkit denial with `note`.
    pub fn deny_writes(&mut self, note: impl Into<String>) {
        self.behavior = WriteBehavior::Deny(note.into());
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(&mut self, message: impl Into<String>) {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
    }

    /// Restore the default authorized behavior.
    pub fn allow_writes(&mut self) {
        self.behavior = WriteBehavior::Accept;
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: StorageData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The daemon goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The daemon comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated daemon is on the bus.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many write requests the source has served. Lets a test prove a write
    /// is one explicit call, never a loop.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    fn write_outcome(&mut self) -> StorageOutcome {
        self.writes += 1;
        if !self.present {
            return StorageOutcome::Absent;
        }
        // A daemon that cannot answer a read cannot answer a write either.
        if let Some(error) = &self.failure {
            return StorageOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => StorageOutcome::Accepted,
            WriteBehavior::Deny(note) => StorageOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => StorageOutcome::Failed(error.clone()),
        }
    }
}

impl StorageSource for MockStorage {
    fn read(&mut self) -> Result<Option<StorageData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn mount(&mut self, _volume_path: &str) -> StorageOutcome {
        self.write_outcome()
    }

    fn unmount(&mut self, _volume_path: &str) -> StorageOutcome {
        self.write_outcome()
    }

    fn eject(&mut self, _drive_path: &str) -> StorageOutcome {
        self.write_outcome()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> StorageData {
        StorageData {
            drives: vec![StorageDriveData {
                path: "/org/freedesktop/UDisks2/drives/USB_Stick".to_owned(),
                removable: true,
                ..StorageDriveData::default()
            }],
            volumes: vec![],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockStorage::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockStorage::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockStorage::failing("UDisks: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "UDisks: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockStorage::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn writes_are_accepted_by_default_and_counted() {
        let mut mock = MockStorage::present(data());
        assert!(mock.mount("/b").is_accepted());
        assert!(mock.unmount("/b").is_accepted());
        assert!(mock.eject("/d").is_accepted());
        assert_eq!(mock.writes(), 3);
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockStorage::present(data());
        mock.deny_writes("UDisks: not authorized");
        let outcome = mock.mount("/b");
        assert_eq!(
            outcome,
            StorageOutcome::Denied("UDisks: not authorized".to_owned())
        );
        assert_eq!(outcome.denial_note(), Some("UDisks: not authorized"));
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockStorage::present(data());
        mock.fail_writes("UDisks: device is busy");
        assert_eq!(
            mock.unmount("/b"),
            StorageOutcome::Failed(AdapterError::new("UDisks: device is busy"))
        );
    }

    #[test]
    fn a_write_while_absent_is_absent() {
        let mut mock = MockStorage::absent();
        assert_eq!(mock.mount("/b"), StorageOutcome::Absent);
    }
}
