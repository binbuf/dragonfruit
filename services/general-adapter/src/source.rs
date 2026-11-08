// SPDX-License-Identifier: MIT
//! The General transport seam and its fixture (T226).
//!
//! A [`GeneralSource`] is the only thing that talks to `timedatectl`,
//! `localectl`, the XDG autostart directories, systemd, the optional backup
//! tools, and `df`. The adapter above it owns the three-state lifecycle; the
//! Settings host owns the JSON projection; the QML never shells out.
//!
//! `Ok(Some(data))` is a live read, `Ok(None)` means no sub-pane owner is
//! reachable, and `Err` is a present stack that could not be read. Each
//! sub-pane is independently optional inside `data`, so one absent owner never
//! hides the rest.

use dragonfruit_system_adapters::AdapterError;

use crate::model::{
    BackupData, GeneralSnapshot, LocaleData, LoginData, SharingData, StartupDiskData,
    StorageUsageData, TimeData,
};

/// The result of one General write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneralOutcome {
    /// The owner accepted the request.
    Applied,
    /// The owner is absent; there is nothing to do.
    Absent,
    /// The owner refused the request (polkit/password).
    Denied(String),
    /// The request failed.
    Failed(AdapterError),
}

impl GeneralOutcome {
    /// Whether the owner accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, GeneralOutcome::Applied)
    }

    /// The refusal or failure text.
    pub fn message(&self) -> Option<&str> {
        match self {
            GeneralOutcome::Denied(message) => Some(message),
            GeneralOutcome::Failed(error) => Some(error.message()),
            GeneralOutcome::Applied | GeneralOutcome::Absent => None,
        }
    }
}

/// Reads and writes the General group's host stack.
pub trait GeneralSource {
    /// One read of every sub-pane.
    fn read(&mut self) -> Result<Option<GeneralSnapshot>, AdapterError>;

    /// Set the system time zone.
    fn set_timezone(&mut self, _timezone: &str) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Enable or disable NTP.
    fn set_ntp(&mut self, _enabled: bool) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Set the wall clock from a `YYYY-MM-DD HH:MM:SS` value.
    fn set_time(&mut self, _value: &str) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Set the system locale (`LANG`).
    fn set_locale(&mut self, _locale: &str) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Enable or disable one XDG autostart entry.
    fn set_autostart_enabled(&mut self, _id: &str, _enabled: bool) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Enable or disable one systemd user unit.
    fn set_user_unit_enabled(&mut self, _unit: &str, _enabled: bool) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Enable or disable one sharing service.
    fn set_sharing_enabled(&mut self, _id: &str, _enabled: bool) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Run one backup through the selected backend.
    fn run_backup(&mut self, _destination: &str) -> GeneralOutcome {
        GeneralOutcome::Absent
    }

    /// Open the selected backend's restore entry.
    fn open_restore(&mut self) -> GeneralOutcome {
        GeneralOutcome::Absent
    }
}

/// A deterministic fixture source for CI, pane tests, and captures.
#[derive(Debug, Clone, PartialEq)]
pub struct MockGeneral {
    present: bool,
    data: Option<GeneralSnapshot>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    reads: u32,
    writes: u32,
    last_timezone: String,
    last_time: String,
    last_locale: String,
    last_autostart: Option<(String, bool)>,
    last_user_unit: Option<(String, bool)>,
    last_sharing: Option<(String, bool)>,
    last_backup_destination: String,
    restores: u32,
}

impl MockGeneral {
    /// No host stack is reachable.
    pub fn absent() -> Self {
        MockGeneral {
            present: false,
            data: None,
            failure: None,
            write_failure: None,
            reads: 0,
            writes: 0,
            last_timezone: String::new(),
            last_time: String::new(),
            last_locale: String::new(),
            last_autostart: None,
            last_user_unit: None,
            last_sharing: None,
            last_backup_destination: String::new(),
            restores: 0,
        }
    }

    /// A host stack answering with the deterministic General fixture.
    pub fn present() -> Self {
        MockGeneral {
            present: true,
            data: Some(fixture()),
            failure: None,
            write_failure: None,
            reads: 0,
            writes: 0,
            last_timezone: String::new(),
            last_time: String::new(),
            last_locale: String::new(),
            last_autostart: None,
            last_user_unit: None,
            last_sharing: None,
            last_backup_destination: String::new(),
            restores: 0,
        }
    }

    /// A host stack answering with `data`.
    pub fn with_data(data: GeneralSnapshot) -> Self {
        let mut mock = MockGeneral::present();
        mock.data = Some(data);
        mock
    }

    /// A host stack that fails every read.
    pub fn failing(message: impl Into<String>) -> Self {
        MockGeneral {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            reads: 0,
            writes: 0,
            last_timezone: String::new(),
            last_time: String::new(),
            last_locale: String::new(),
            last_autostart: None,
            last_user_unit: None,
            last_sharing: None,
            last_backup_destination: String::new(),
            restores: 0,
        }
    }

    /// Make every write fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// The fixture goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The fixture comes back.
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// The last snapshot the fixture served.
    pub fn data(&self) -> Option<&GeneralSnapshot> {
        self.data.as_ref()
    }

    /// How many reads the fixture served.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many writes the fixture accepted.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// The last time zone write.
    pub fn last_timezone(&self) -> &str {
        &self.last_timezone
    }

    /// The last manual-time write.
    pub fn last_time(&self) -> &str {
        &self.last_time
    }

    /// The last locale write.
    pub fn last_locale(&self) -> &str {
        &self.last_locale
    }

    /// The last autostart toggle.
    pub fn last_autostart(&self) -> Option<(&str, bool)> {
        self.last_autostart
            .as_ref()
            .map(|(id, enabled)| (id.as_str(), *enabled))
    }

    /// The last user-unit toggle.
    pub fn last_user_unit(&self) -> Option<(&str, bool)> {
        self.last_user_unit
            .as_ref()
            .map(|(id, enabled)| (id.as_str(), *enabled))
    }

    /// The last sharing toggle.
    pub fn last_sharing(&self) -> Option<(&str, bool)> {
        self.last_sharing
            .as_ref()
            .map(|(id, enabled)| (id.as_str(), *enabled))
    }

    /// The destination passed to the last backup run.
    pub fn last_backup_destination(&self) -> &str {
        &self.last_backup_destination
    }

    /// How many restore entries were opened.
    pub fn restores(&self) -> u32 {
        self.restores
    }

    fn outcome(&self) -> GeneralOutcome {
        if !self.present || self.data.is_none() {
            return GeneralOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return GeneralOutcome::Failed(error.clone());
        }
        GeneralOutcome::Applied
    }
}

impl GeneralSource for MockGeneral {
    fn read(&mut self) -> Result<Option<GeneralSnapshot>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(self.data.clone())
    }

    fn set_timezone(&mut self, timezone: &str) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_timezone = timezone.to_owned();
            if let Some(time) = self.data.as_mut().and_then(|data| data.time.as_mut()) {
                time.timezone = timezone.to_owned();
            }
        }
        outcome
    }

    fn set_ntp(&mut self, enabled: bool) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            if let Some(time) = self.data.as_mut().and_then(|data| data.time.as_mut()) {
                time.ntp_enabled = enabled;
            }
        }
        outcome
    }

    fn set_time(&mut self, value: &str) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_time = value.to_owned();
        }
        outcome
    }

    fn set_locale(&mut self, locale: &str) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_locale = locale.to_owned();
            if let Some(data) = self.data.as_mut().and_then(|data| data.locale.as_mut()) {
                data.locale = locale.to_owned();
                data.language = locale
                    .split(['_', '.', '@'])
                    .next()
                    .unwrap_or(locale)
                    .to_owned();
                data.formats_locale = locale.to_owned();
            }
        }
        outcome
    }

    fn set_autostart_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_autostart = Some((id.to_owned(), enabled));
            if let Some(login) = self.data.as_mut().and_then(|data| data.login.as_mut()) {
                if let Some(item) = login.autostart.iter_mut().find(|item| item.id == id) {
                    item.enabled = enabled;
                }
            }
        }
        outcome
    }

    fn set_user_unit_enabled(&mut self, unit: &str, enabled: bool) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_user_unit = Some((unit.to_owned(), enabled));
            if let Some(login) = self.data.as_mut().and_then(|data| data.login.as_mut()) {
                if let Some(item) = login.user_units.iter_mut().find(|item| item.id == unit) {
                    item.enabled = if enabled { "enabled" } else { "disabled" }.to_owned();
                }
            }
        }
        outcome
    }

    fn set_sharing_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_sharing = Some((id.to_owned(), enabled));
            if let Some(sharing) = self.data.as_mut().and_then(|data| data.sharing.as_mut()) {
                if let Some(service) = sharing.services.iter_mut().find(|item| item.id == id) {
                    service.enabled = enabled;
                    service.active = enabled;
                }
            }
        }
        outcome
    }

    fn run_backup(&mut self, destination: &str) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_backup_destination = destination.to_owned();
            if let Some(backup) = self.data.as_mut().and_then(|data| data.backup.as_mut()) {
                backup.message = "Backup started".to_owned();
            }
        }
        outcome
    }

    fn open_restore(&mut self) -> GeneralOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.restores += 1;
        }
        outcome
    }
}

/// The deterministic fixture every General pane test and capture uses.
pub fn fixture() -> GeneralSnapshot {
    GeneralSnapshot {
        time: Some(TimeData {
            timezone: "America/Los_Angeles".to_owned(),
            ntp_enabled: true,
            ntp_synchronized: true,
            can_ntp: true,
            local_time_ms: 1_760_000_000_000,
            timezones: vec![
                "UTC".to_owned(),
                "America/Los_Angeles".to_owned(),
                "Europe/Berlin".to_owned(),
            ],
        }),
        locale: Some(LocaleData {
            locale: "en_US.UTF-8".to_owned(),
            language: "en".to_owned(),
            formats_locale: "en_US.UTF-8".to_owned(),
            keymap: "us".to_owned(),
            available_locales: vec![
                "en_US.UTF-8".to_owned(),
                "de_DE.UTF-8".to_owned(),
                "fr_FR.UTF-8".to_owned(),
            ],
        }),
        login: Some(LoginData {
            autostart: vec![
                crate::model::LoginItem {
                    id: "org.dragonfruit.Backup".to_owned(),
                    name: "Dragonfruit Backup".to_owned(),
                    command: "/usr/bin/dragonfruit-backup".to_owned(),
                    source: "user".to_owned(),
                    path: "/home/user/.config/autostart/org.dragonfruit.Backup.desktop".to_owned(),
                    enabled: true,
                    system: false,
                    can_toggle: true,
                },
                crate::model::LoginItem {
                    id: "org.example.Agent".to_owned(),
                    name: "Example Agent".to_owned(),
                    command: "/usr/bin/example-agent".to_owned(),
                    source: "system".to_owned(),
                    path: "/etc/xdg/autostart/org.example.Agent.desktop".to_owned(),
                    enabled: false,
                    system: true,
                    can_toggle: true,
                },
            ],
            user_units: vec![
                crate::model::UserUnit {
                    id: "backup.service".to_owned(),
                    name: "Backup".to_owned(),
                    enabled: "enabled".to_owned(),
                    active: "active".to_owned(),
                    can_toggle: true,
                },
                crate::model::UserUnit {
                    id: "sync.service".to_owned(),
                    name: "Sync".to_owned(),
                    enabled: "disabled".to_owned(),
                    active: "inactive".to_owned(),
                    can_toggle: true,
                },
            ],
            extensions_available: false,
            extensions_note: "Extensions are managed by the desktop portal and installed apps."
                .to_owned(),
        }),
        sharing: Some(SharingData {
            services: vec![
                crate::model::SharingService {
                    id: "file".to_owned(),
                    name: "File Sharing".to_owned(),
                    description: "Share folders with other computers using SMB.".to_owned(),
                    unit: "smb.service".to_owned(),
                    installed: true,
                    enabled: true,
                    active: false,
                    can_toggle: true,
                    needs_auth: true,
                },
                crate::model::SharingService {
                    id: "remoteLogin".to_owned(),
                    name: "Remote Login".to_owned(),
                    description: "Allow remote login with SSH.".to_owned(),
                    unit: "sshd.service".to_owned(),
                    installed: true,
                    enabled: true,
                    active: true,
                    can_toggle: true,
                    needs_auth: true,
                },
                crate::model::SharingService {
                    id: "printer".to_owned(),
                    name: "Printer Sharing".to_owned(),
                    description: "Share printers with other computers.".to_owned(),
                    unit: "cups.service".to_owned(),
                    installed: true,
                    enabled: true,
                    active: true,
                    can_toggle: true,
                    needs_auth: true,
                },
            ],
        }),
        backup: Some(BackupData {
            backend: "timeshift".to_owned(),
            backend_label: "Timeshift".to_owned(),
            candidates: vec![
                crate::model::BackupBackend {
                    id: "restic".to_owned(),
                    label: "restic".to_owned(),
                    installed: false,
                    can_run: false,
                    can_restore: false,
                },
                crate::model::BackupBackend {
                    id: "pika".to_owned(),
                    label: "Pika Backup".to_owned(),
                    installed: false,
                    can_run: false,
                    can_restore: false,
                },
                crate::model::BackupBackend {
                    id: "timeshift".to_owned(),
                    label: "Timeshift".to_owned(),
                    installed: true,
                    can_run: true,
                    can_restore: true,
                },
            ],
            detected_destination: "/dev/sdb1".to_owned(),
            detected_schedule: "daily".to_owned(),
            last_run: "2026-11-06 22:00".to_owned(),
            running: false,
            can_run: true,
            can_restore: true,
            message: String::new(),
        }),
        storage: Some(StorageUsageData {
            categories: vec![
                crate::model::StorageCategory {
                    id: "system".to_owned(),
                    label: "System".to_owned(),
                    bytes: 120_000_000_000,
                    glyph: "hard-drives".to_owned(),
                },
                crate::model::StorageCategory {
                    id: "home".to_owned(),
                    label: "Home".to_owned(),
                    bytes: 340_000_000_000,
                    glyph: "house".to_owned(),
                },
                crate::model::StorageCategory {
                    id: "media".to_owned(),
                    label: "Media".to_owned(),
                    bytes: 20_000_000_000,
                    glyph: "image".to_owned(),
                },
            ],
            volumes: vec![crate::model::VolumeUsage {
                device: "/dev/nvme0n1p3".to_owned(),
                mount_point: "/".to_owned(),
                category: "system".to_owned(),
                total_bytes: 512_000_000_000,
                used_bytes: 480_000_000_000,
                available_bytes: 32_000_000_000,
                removable: false,
            }],
            total_bytes: 512_000_000_000,
            used_bytes: 480_000_000_000,
            free_bytes: 32_000_000_000,
        }),
        startup: Some(StartupDiskData {
            device: "/dev/nvme0n1p3".to_owned(),
            disk: "/dev/nvme0n1".to_owned(),
            filesystem: "btrfs".to_owned(),
            mount_point: "/".to_owned(),
            total_bytes: 512_000_000_000,
            boot_mode: "UEFI".to_owned(),
            secure_boot: "On".to_owned(),
            label: "fedora".to_owned(),
        }),
        errors: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_covers_every_sub_pane() {
        let data = fixture();
        assert!(data.has_data());
        assert_eq!(data.time.unwrap().timezone, "America/Los_Angeles");
        assert_eq!(data.locale.unwrap().locale, "en_US.UTF-8");
        assert_eq!(data.login.unwrap().autostart.len(), 2);
        assert_eq!(data.sharing.unwrap().services.len(), 3);
        assert_eq!(data.backup.unwrap().backend, "timeshift");
        assert_eq!(data.storage.unwrap().categories.len(), 3);
        assert_eq!(data.startup.unwrap().boot_mode, "UEFI");
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut mock = MockGeneral::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.set_ntp(true), GeneralOutcome::Absent);
        assert_eq!(mock.writes(), 0);
    }

    #[test]
    fn a_present_mock_applies_every_write() {
        let mut mock = MockGeneral::present();
        assert!(mock.set_timezone("Europe/Berlin").is_applied());
        assert_eq!(mock.last_timezone(), "Europe/Berlin");
        assert!(mock.set_ntp(false).is_applied());
        assert!(mock.set_locale("de_DE.UTF-8").is_applied());
        assert_eq!(mock.last_locale(), "de_DE.UTF-8");
        assert!(mock
            .set_autostart_enabled("org.example.Agent", true)
            .is_applied());
        assert_eq!(mock.last_autostart(), Some(("org.example.Agent", true)));
        assert!(mock
            .set_user_unit_enabled("sync.service", true)
            .is_applied());
        assert!(mock.set_sharing_enabled("remoteLogin", false).is_applied());
        assert!(mock.run_backup("/mnt/backup").is_applied());
        assert!(mock.open_restore().is_applied());
        assert_eq!(mock.restores(), 1);
        assert_eq!(mock.writes(), 8);
    }

    #[test]
    fn a_failing_write_reports_its_message() {
        let mut mock = MockGeneral::present().fail_writes("polkit: not authorized");
        let outcome = mock.set_sharing_enabled("file", false);
        assert_eq!(outcome.message(), Some("polkit: not authorized"));
        assert!(!outcome.is_applied());
    }
}
