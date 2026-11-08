// SPDX-License-Identifier: MIT
//! The typed General sub-pane data (T226).
//!
//! One snapshot carries the five independent host reads behind the General
//! group: Date & Time, Language & Region, Login Items, Sharing, Backup, plus
//! the Storage usage categories and the read-only Startup Disk rows. Each half
//! is `Option`: an absent owner disables only its own sub-pane and leaves the
//! others live. Nothing here is invented; an unknown string is empty and an
//! unknown size is `0`.

use std::collections::BTreeMap;

/// The live Date & Time state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimeData {
    /// The IANA time zone (`America/Los_Angeles`).
    pub timezone: String,
    /// `timedatectl` reports NTP enabled.
    pub ntp_enabled: bool,
    /// The clock is currently synchronized by NTP.
    pub ntp_synchronized: bool,
    /// The host supports NTP (`CanNTP`).
    pub can_ntp: bool,
    /// The host's current wall clock (milliseconds since the Unix epoch).
    pub local_time_ms: i64,
    /// The zones the host offers (`timedatectl list-timezones`).
    pub timezones: Vec<String>,
}

/// The live locale state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocaleData {
    /// The `LANG` value (`en_US.UTF-8`).
    pub locale: String,
    /// The language half (`en`).
    pub language: String,
    /// The formatting locale (`LC_TIME`, else `LANG`).
    pub formats_locale: String,
    /// The console keymap (`VC Keymap`), when reported.
    pub keymap: String,
    /// The locales the host has generated.
    pub available_locales: Vec<String>,
}

/// One XDG autostart entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginItem {
    /// The desktop-file stem (`org.example.Agent`).
    pub id: String,
    /// The display name.
    pub name: String,
    /// The command the entry runs.
    pub command: String,
    /// `user` or `system` (the directory the entry was read from).
    pub source: String,
    /// The desktop file path.
    pub path: String,
    /// The entry runs at login.
    pub enabled: bool,
    /// The entry comes from `/etc/xdg/autostart` (it is disabled with a user
    /// override rather than edited in place).
    pub system: bool,
    /// The entry can be toggled.
    pub can_toggle: bool,
}

/// One systemd user unit the Login Items pane lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserUnit {
    /// The unit name (`backup.service`).
    pub id: String,
    /// The `[Unit] Description=`, else the unit name.
    pub name: String,
    /// `systemctl --user is-enabled` output (`enabled`, `disabled`, `static`).
    pub enabled: String,
    /// `systemctl --user is-active` output (`active`, `inactive`, `failed`).
    pub active: String,
    /// The unit can be enabled/disabled.
    pub can_toggle: bool,
}

/// The Login Items data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginData {
    /// The XDG autostart entries.
    pub autostart: Vec<LoginItem>,
    /// The systemd user units the user can enable.
    pub user_units: Vec<UserUnit>,
    /// Extensions are a delegated portal surface; `false` shows the note.
    pub extensions_available: bool,
    /// The extension hand-off note.
    pub extensions_note: String,
}

/// One sharing service the host exposes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharingService {
    /// The stable id (`file`, `screen`, `media`, `printer`, `remoteLogin`,
    /// `remoteManagement`).
    pub id: String,
    /// The display name.
    pub name: String,
    /// The one-line description.
    pub description: String,
    /// The systemd unit that owns the service.
    pub unit: String,
    /// The service is installed on this host.
    pub installed: bool,
    /// The unit is enabled at boot.
    pub enabled: bool,
    /// The unit is running now.
    pub active: bool,
    /// The toggle can be applied (a static/masked unit cannot).
    pub can_toggle: bool,
    /// The write needs administrator authentication.
    pub needs_auth: bool,
}

/// The Sharing data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharingData {
    /// Only the installed services; an absent service is not a row.
    pub services: Vec<SharingService>,
}

/// One detected backup backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupBackend {
    /// `restic`, `pika`, or `timeshift`.
    pub id: String,
    /// The display label.
    pub label: String,
    /// The tool is installed.
    pub installed: bool,
    /// The backend can be run non-interactively.
    pub can_run: bool,
    /// The backend has a restore entry.
    pub can_restore: bool,
}

/// The Backup data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupData {
    /// The selected backend id, empty when none is installed.
    pub backend: String,
    /// The selected backend label.
    pub backend_label: String,
    /// Every backend the detection found.
    pub candidates: Vec<BackupBackend>,
    /// The destination the backend's own config reports, when it has one.
    pub detected_destination: String,
    /// The schedule the backend's own config reports, when it has one.
    pub detected_schedule: String,
    /// The backend's last-run text, when it has one.
    pub last_run: String,
    /// A backup is currently running.
    pub running: bool,
    /// The selected backend can be run from the pane.
    pub can_run: bool,
    /// The selected backend has a restore entry.
    pub can_restore: bool,
    /// The backend's status or absence message.
    pub message: String,
}

/// One storage category the usage chart draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageCategory {
    /// `system`, `home`, `media`, or `other`.
    pub id: String,
    /// The display label.
    pub label: String,
    /// The category's used bytes.
    pub bytes: u64,
    /// The design-system glyph.
    pub glyph: String,
}

/// One mounted filesystem's usage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VolumeUsage {
    /// The device node.
    pub device: String,
    /// The mount point.
    pub mount_point: String,
    /// The category id.
    pub category: String,
    /// The filesystem size in bytes.
    pub total_bytes: u64,
    /// The used bytes.
    pub used_bytes: u64,
    /// The available bytes.
    pub available_bytes: u64,
    /// The mount point is removable media.
    pub removable: bool,
}

/// The Storage usage data.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StorageUsageData {
    /// The category totals.
    pub categories: Vec<StorageCategory>,
    /// The mounted filesystems.
    pub volumes: Vec<VolumeUsage>,
    /// The unique-device total bytes.
    pub total_bytes: u64,
    /// The unique-device used bytes.
    pub used_bytes: u64,
    /// The unique-device free bytes.
    pub free_bytes: u64,
}

/// The read-only Startup Disk rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StartupDiskData {
    /// The root device (`/dev/nvme0n1p3`).
    pub device: String,
    /// The parent disk (`/dev/nvme0n1`).
    pub disk: String,
    /// The root filesystem type.
    pub filesystem: String,
    /// The root mount point.
    pub mount_point: String,
    /// The root filesystem size in bytes.
    pub total_bytes: u64,
    /// `UEFI` or `BIOS`.
    pub boot_mode: String,
    /// `On`, `Off`, or `Unknown`.
    pub secure_boot: String,
    /// The filesystem label, when one is set.
    pub label: String,
}

/// One read of every General sub-pane.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GeneralSnapshot {
    /// Date & Time, when `timedatectl` answers.
    pub time: Option<TimeData>,
    /// Language & Region, when `localectl` answers.
    pub locale: Option<LocaleData>,
    /// Login Items, when the autostart directories or systemd answer.
    pub login: Option<LoginData>,
    /// Sharing, when at least one candidate service is installed.
    pub sharing: Option<SharingData>,
    /// Backup backend detection.
    pub backup: Option<BackupData>,
    /// Storage usage, when `df` answers.
    pub storage: Option<StorageUsageData>,
    /// Startup Disk, when `findmnt` answers.
    pub startup: Option<StartupDiskData>,
    /// Per-sub-pane read errors, keyed by `time`, `locale`, `login`,
    /// `sharing`, `backup`, `storage`, or `startup`.
    pub errors: BTreeMap<String, String>,
}

impl GeneralSnapshot {
    /// Whether any sub-pane answered.
    pub fn has_data(&self) -> bool {
        self.time.is_some()
            || self.locale.is_some()
            || self.login.is_some()
            || self.sharing.is_some()
            || self.backup.is_some()
            || self.storage.is_some()
            || self.startup.is_some()
    }

    /// The read error for one sub-pane, when it failed.
    pub fn error(&self, id: &str) -> Option<&str> {
        self.errors.get(id).map(String::as_str)
    }
}
