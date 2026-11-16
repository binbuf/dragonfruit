// SPDX-License-Identifier: MIT
//! The live General host read/write source (T226).
//!
//! Every command and file the General group needs lives here, behind the
//! [`GeneralSource`] seam: `timedatectl`, `localectl`, the XDG autostart
//! directories, `systemctl --user`, the system-unit inventory, the optional
//! backup tools, `df`, and `findmnt`. The Settings host and the QML never name
//! a distribution tool.
//!
//! # Absence is per sub-pane
//!
//! A missing tool is `Ok(None)` for that sub-pane and an entry in
//! [`GeneralSnapshot::errors`]; the other sub-panes keep working. Only when no
//! sub-pane answers at all does the adapter become `Unavailable`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use dragonfruit_system_adapters::AdapterError;

use crate::model::{
    BackupBackend, BackupData, DiskEntry, ExtensionEntry, GeneralSnapshot, LocaleData, LoginApp,
    LoginData, LoginItem, PasswordData, PasswordProvider, ServiceDetail, SharingData,
    SharingService, StartupDiskData, StorageCategory, StorageUsageData, TimeData, UserUnit,
    VolumeUsage,
};
use crate::source::{GeneralOutcome, GeneralSource};

/// One command's captured answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Output {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Output {
    fn success(&self) -> bool {
        self.status == Some(0)
    }

    fn error_text(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            self.stdout.trim()
        } else {
            self.stderr.trim()
        };
        if text.is_empty() {
            format!("command exited with {:?}", self.status)
        } else {
            text.lines().last().unwrap_or(text).to_owned()
        }
    }
}

fn run(program: &str, args: &[&str]) -> Result<Output, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    Ok(Output {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn program_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// The live General source.
#[derive(Debug, Clone, Default)]
pub struct HostGeneral {
    home: PathBuf,
}

impl HostGeneral {
    /// A source reading the real user's home and the live host tools.
    pub fn new() -> Self {
        HostGeneral {
            home: std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/root")),
        }
    }

    /// A source reading `home` for the autostart and systemd-user files.
    pub fn with_home(home: impl Into<PathBuf>) -> Self {
        HostGeneral { home: home.into() }
    }

    fn config_home(&self) -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.home.join(".config"))
    }

    fn read_time(&self) -> Result<TimeData, String> {
        let output = run(
            "timedatectl",
            &[
                "show",
                "-p",
                "Timezone",
                "-p",
                "NTP",
                "-p",
                "NTPSynchronized",
                "-p",
                "CanNTP",
            ],
        )?;
        if !output.success() {
            return Err(output.error_text());
        }
        let mut time = parse_timedatectl(&output.stdout);
        if time.timezone.is_empty() {
            return Err("timedatectl: no time zone reported".to_owned());
        }
        time.local_time_ms = now_ms();
        if let Ok(zones) = run("timedatectl", &["list-timezones"]) {
            if zones.success() {
                time.timezones = zones
                    .stdout
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
        }
        Ok(time)
    }

    fn read_locale(&self) -> Result<LocaleData, String> {
        let output = run("localectl", &["status"])?;
        if !output.success() {
            return Err(output.error_text());
        }
        let mut locale = parse_localectl(&output.stdout);
        if locale.locale.is_empty() {
            return Err("localectl: no system locale reported".to_owned());
        }
        if let Ok(locales) = run("localectl", &["list-locales"]) {
            if locales.success() {
                locale.available_locales = locales
                    .stdout
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
        }
        Ok(locale)
    }

    fn read_login(&self) -> Result<LoginData, String> {
        let mut autostart = read_autostart(&self.config_home(), Path::new("/etc/xdg/autostart"));
        for item in &mut autostart {
            let (broken, reason) = broken_login_item(&item.command);
            item.broken = broken;
            item.broken_reason = reason;
        }
        let mut errors = Vec::new();
        let user_units = match read_user_units(&self.home) {
            Ok(units) => units,
            Err(error) => {
                errors.push(error);
                Vec::new()
            }
        };
        if autostart.is_empty() && user_units.is_empty() && !errors.is_empty() {
            return Err(errors.join("; "));
        }
        autostart.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        let available_apps = read_login_apps(&self.config_home(), &autostart);
        let extensions = read_portal_extensions();
        Ok(LoginData {
            autostart,
            user_units,
            available_apps,
            extensions,
            extensions_available: false,
            extensions_note: "Extensions are managed by the desktop portal and installed apps."
                .to_owned(),
        })
    }

    fn read_sharing(&self) -> Result<SharingData, String> {
        let mut services = Vec::new();
        let mut saw_systemctl = false;
        let hostname = read_hostname();
        let hostname_editable = program_path("hostnamectl").is_some();
        let hostname_reason = if hostname_editable {
            String::new()
        } else {
            "The hostname is read-only because hostnamectl is not installed.".to_owned()
        };
        for candidate in SHARING_CANDIDATES {
            let mut unit = String::new();
            for name in candidate.units {
                match run("systemctl", &["cat", name]) {
                    Ok(output) => {
                        saw_systemctl = true;
                        if output.success() {
                            unit = (*name).to_owned();
                            break;
                        }
                    }
                    Err(_) => {}
                }
            }
            let binary = candidate
                .binaries
                .iter()
                .find(|binary| program_path(binary).is_some());
            if unit.is_empty() && binary.is_none() {
                continue;
            }
            if unit.is_empty() {
                // A binary with no unit is still an installed service the
                // pane can describe, but there is nothing to toggle.
                services.push(SharingService {
                    id: candidate.id.to_owned(),
                    name: candidate.name.to_owned(),
                    description: candidate.description.to_owned(),
                    unit: String::new(),
                    installed: true,
                    enabled: false,
                    active: false,
                    can_toggle: false,
                    needs_auth: true,
                    details: sharing_details(candidate.id, "", &hostname),
                });
                continue;
            }
            let enabled = run("systemctl", &["is-enabled", &unit])
                .ok()
                .map(|output| output.stdout.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_owned());
            let active = run("systemctl", &["is-active", &unit])
                .ok()
                .map(|output| output.stdout.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_owned());
            services.push(SharingService {
                id: candidate.id.to_owned(),
                name: candidate.name.to_owned(),
                description: candidate.description.to_owned(),
                unit: unit.clone(),
                installed: true,
                enabled: matches!(enabled.as_str(), "enabled" | "enabled-runtime" | "alias"),
                active: active == "active",
                can_toggle: !matches!(
                    enabled.as_str(),
                    "static" | "masked" | "generated" | "transient" | "indirect"
                ),
                needs_auth: true,
                details: sharing_details(candidate.id, &unit, &hostname),
            });
        }
        if services.is_empty() && !saw_systemctl {
            return Err("systemctl is not available".to_owned());
        }
        Ok(SharingData {
            services,
            hostname,
            hostname_editable,
            hostname_reason,
        })
    }

    fn read_backup(&self) -> Result<BackupData, String> {
        let restic = program_path("restic").is_some();
        let pika = program_path("pika-backup").is_some();
        let timeshift = program_path("timeshift").is_some();
        let timeshift_gui = program_path("timeshift-gtk").is_some();
        let candidates = vec![
            BackupBackend {
                id: "restic".to_owned(),
                label: "restic".to_owned(),
                installed: restic,
                can_run: restic,
                can_restore: false,
            },
            BackupBackend {
                id: "pika".to_owned(),
                label: "Pika Backup".to_owned(),
                installed: pika,
                can_run: false,
                can_restore: pika,
            },
            BackupBackend {
                id: "timeshift".to_owned(),
                label: "Timeshift".to_owned(),
                installed: timeshift,
                can_run: timeshift,
                can_restore: timeshift_gui,
            },
        ];
        let selected = candidates
            .iter()
            .find(|candidate| candidate.installed)
            .cloned()
            .unwrap_or_default();
        let (detected_destination, detected_schedule, last_run) = if timeshift {
            read_timeshift_config()
        } else {
            (String::new(), String::new(), String::new())
        };
        let running = ["timeshift", "restic", "pika-backup"]
            .iter()
            .any(|program| pgrep(program));
        let message = if selected.id.is_empty() {
            "No backup backend is installed.".to_owned()
        } else {
            String::new()
        };
        Ok(BackupData {
            backend: selected.id,
            backend_label: selected.label,
            candidates,
            detected_destination,
            detected_schedule,
            last_run,
            running,
            can_run: selected.can_run,
            can_restore: selected.can_restore,
            message,
        })
    }

    fn read_storage(&self) -> Result<StorageUsageData, String> {
        let output = run(
            "df",
            &[
                "-B1", "-P", "-x", "tmpfs", "-x", "devtmpfs", "-x", "squashfs", "-x", "overlay",
            ],
        )?;
        if !output.success() {
            return Err(output.error_text());
        }
        let usage = parse_df_usage(&output.stdout, &self.home);
        if usage.volumes.is_empty() {
            return Err("df: no mounted filesystems".to_owned());
        }
        Ok(usage)
    }

    fn read_startup(&self) -> Result<StartupDiskData, String> {
        let output = run(
            "findmnt",
            &["-n", "-o", "SOURCE,TARGET,FSTYPE,SIZE", "-b", "/"],
        )?;
        if !output.success() {
            return Err(output.error_text());
        }
        let mut disk = parse_startup(&output.stdout);
        if disk.device.is_empty() {
            return Err("findmnt: no root device reported".to_owned());
        }
        if let Ok(parent) = run("lsblk", &["-no", "PKNAME", &disk.device]) {
            let parent = parent.stdout.trim();
            if !parent.is_empty() {
                disk.disk = if parent.starts_with('/') {
                    parent.to_owned()
                } else {
                    format!("/dev/{parent}")
                };
            }
        }
        if let Ok(label) = run("lsblk", &["-no", "LABEL", &disk.device]) {
            disk.label = label.stdout.trim().to_owned();
        }
        disk.boot_mode = if Path::new("/sys/firmware/efi").exists() {
            "UEFI".to_owned()
        } else {
            "BIOS".to_owned()
        };
        disk.secure_boot = read_secure_boot();
        if let Ok(output) = run(
            "lsblk",
            &[
                "-J",
                "-b",
                "-o",
                "NAME,SIZE,TYPE,MODEL,LABEL,MOUNTPOINT",
            ],
        ) {
            if output.success() {
                disk.disks = parse_lsblk_disks(&output.stdout, &disk.disk);
            }
        }
        if disk.disks.is_empty() {
            disk.disks.push(DiskEntry {
                device: disk.disk.clone(),
                label: disk.label.clone(),
                model: String::new(),
                size_bytes: disk.total_bytes,
                boot: true,
                mounted: true,
            });
        }
        Ok(disk)
    }

    // The Autofill & Passwords inventory. It reads the Secret Service name
    // owner and the installed manager executables; it never opens a secret
    // item, reads a secret value, or stores one. A host with no provider is a
    // valid answer with an explanatory message.
    fn read_passwords(&self) -> Result<PasswordData, String> {
        let owner = secret_service_owner();
        let secret_service_available = owner.is_some();
        let mut providers = Vec::new();
        for candidate in PASSWORD_CANDIDATES {
            let mut command = String::new();
            for name in candidate.commands {
                if program_path(name).is_some() {
                    command = (*name).to_owned();
                    break;
                }
            }
            let installed = !command.is_empty();
            let active = secret_service_available
                && owner
                    .as_deref()
                    .map(|process| candidate.matches_process(process))
                    .unwrap_or(false);
            let launch = if program_path(candidate.launch).is_some() {
                candidate.launch.to_owned()
            } else {
                String::new()
            };
            providers.push(PasswordProvider {
                id: candidate.id.to_owned(),
                label: candidate.label.to_owned(),
                installed,
                active,
                command,
                launch,
                description: candidate.description.to_owned(),
            });
        }
        let keyring_lock = if secret_service_available {
            read_keyring_lock()
        } else {
            None
        };
        let keyring_label = match keyring_lock {
            Some(true) => "Locked",
            Some(false) => "Unlocked",
            None => "Not reported",
        }
        .to_owned();
        let message = if !providers.iter().any(|provider| provider.installed) {
            "No password provider is installed. Autofill is unavailable until one is installed."
                .to_owned()
        } else if !secret_service_available {
            "The Secret Service is not running, so the installed providers cannot share saved credentials."
                .to_owned()
        } else {
            String::new()
        };
        Ok(PasswordData {
            providers,
            secret_service_available,
            keyring_lock,
            keyring_label,
            message,
        })
    }

    fn write(&self, program: &str, args: &[&str]) -> GeneralOutcome {
        match run(program, args) {
            Ok(output) if output.success() => GeneralOutcome::Applied,
            Ok(output) => {
                let text = output.error_text();
                if is_denied(&text) {
                    // T282: a normal user's direct write is refused with
                    // "Interactive authentication required". When pkexec is
                    // installed it is the documented polkit delegate; the
                    // shell's agent answers the prompt. Without it (or when
                    // the second attempt is refused too) the denial stays
                    // visible as a `Denied` outcome.
                    if let Some(privileged) = self.write_via_pkexec(program, args) {
                        return privileged;
                    }
                    GeneralOutcome::Denied(text)
                } else {
                    GeneralOutcome::Failed(AdapterError::new(text))
                }
            }
            Err(error) => GeneralOutcome::Failed(AdapterError::new(error)),
        }
    }

    /// Retry one refused system write through `pkexec`. `None` when pkexec is
    /// not installed, so the caller keeps the original denial message.
    fn write_via_pkexec(&self, program: &str, args: &[&str]) -> Option<GeneralOutcome> {
        program_path("pkexec")?;
        let mut command = vec![program];
        command.extend_from_slice(args);
        Some(match run("pkexec", &command) {
            Ok(output) if output.success() => GeneralOutcome::Applied,
            Ok(output) => {
                let text = output.error_text();
                if is_denied(&text) {
                    GeneralOutcome::Denied(text)
                } else {
                    GeneralOutcome::Failed(AdapterError::new(text))
                }
            }
            Err(error) => GeneralOutcome::Failed(AdapterError::new(error)),
        })
    }
}

impl GeneralSource for HostGeneral {
    fn read(&mut self) -> Result<Option<GeneralSnapshot>, AdapterError> {
        let mut snapshot = GeneralSnapshot::default();
        let mut errors = BTreeMap::new();
        macro_rules! read_part {
            ($id:literal, $method:ident, $field:ident) => {
                match self.$method() {
                    Ok(value) => snapshot.$field = Some(value),
                    Err(error) => {
                        errors.insert($id.to_owned(), error);
                    }
                }
            };
        }
        read_part!("time", read_time, time);
        read_part!("locale", read_locale, locale);
        read_part!("login", read_login, login);
        read_part!("sharing", read_sharing, sharing);
        read_part!("backup", read_backup, backup);
        read_part!("storage", read_storage, storage);
        read_part!("startup", read_startup, startup);
        read_part!("passwords", read_passwords, passwords);
        if !snapshot.has_data() && errors.is_empty() {
            return Ok(None);
        }
        snapshot.errors = errors;
        Ok(Some(snapshot))
    }

    fn set_timezone(&mut self, timezone: &str) -> GeneralOutcome {
        if timezone.is_empty() {
            return GeneralOutcome::Failed(AdapterError::new("time zone is empty"));
        }
        self.write("timedatectl", &["set-timezone", timezone])
    }

    fn set_ntp(&mut self, enabled: bool) -> GeneralOutcome {
        self.write(
            "timedatectl",
            &["set-ntp", if enabled { "true" } else { "false" }],
        )
    }

    fn set_time(&mut self, value: &str) -> GeneralOutcome {
        if value.trim().is_empty() {
            return GeneralOutcome::Failed(AdapterError::new("date and time are empty"));
        }
        self.write("timedatectl", &["set-time", value.trim()])
    }

    fn set_locale(&mut self, locale: &str) -> GeneralOutcome {
        if locale.is_empty() {
            return GeneralOutcome::Failed(AdapterError::new("locale is empty"));
        }
        let assignment = format!("LANG={locale}");
        self.write("localectl", &["set-locale", &assignment])
    }

    fn set_autostart_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        match set_autostart(
            &self.config_home(),
            Path::new("/etc/xdg/autostart"),
            id,
            enabled,
        ) {
            Ok(()) => GeneralOutcome::Applied,
            Err(error) => GeneralOutcome::Failed(AdapterError::new(error)),
        }
    }

    fn add_autostart(&mut self, app_id: &str) -> GeneralOutcome {
        match add_autostart(&self.config_home(), app_id) {
            Ok(()) => GeneralOutcome::Applied,
            Err(error) => GeneralOutcome::Failed(AdapterError::new(error)),
        }
    }

    fn remove_autostart(&mut self, id: &str) -> GeneralOutcome {
        match remove_autostart(
            &self.config_home(),
            Path::new("/etc/xdg/autostart"),
            id,
        ) {
            Ok(()) => GeneralOutcome::Applied,
            Err(error) => GeneralOutcome::Failed(AdapterError::new(error)),
        }
    }

    fn set_user_unit_enabled(&mut self, unit: &str, enabled: bool) -> GeneralOutcome {
        self.write(
            "systemctl",
            &["--user", if enabled { "enable" } else { "disable" }, unit],
        )
    }

    fn set_sharing_enabled(&mut self, id: &str, enabled: bool) -> GeneralOutcome {
        let Some(candidate) = SHARING_CANDIDATES
            .iter()
            .find(|candidate| candidate.id == id)
        else {
            return GeneralOutcome::Absent;
        };
        let mut unit = String::new();
        for name in candidate.units {
            if let Ok(output) = run("systemctl", &["cat", name]) {
                if output.success() {
                    unit = (*name).to_owned();
                    break;
                }
            }
        }
        if unit.is_empty() {
            return GeneralOutcome::Absent;
        }
        self.write(
            "systemctl",
            &[if enabled { "enable" } else { "disable" }, "--now", &unit],
        )
    }

    fn set_hostname(&mut self, hostname: &str) -> GeneralOutcome {
        let hostname = hostname.trim();
        if let Err(error) = validate_hostname(hostname) {
            return GeneralOutcome::Failed(AdapterError::new(error));
        }
        if program_path("hostnamectl").is_none() {
            return GeneralOutcome::Absent;
        }
        self.write("hostnamectl", &["set-hostname", hostname])
    }

    fn restart_system(&mut self) -> GeneralOutcome {
        if program_path("systemctl").is_none() {
            return GeneralOutcome::Absent;
        }
        self.write("systemctl", &["reboot"])
    }

    fn run_backup(&mut self, destination: &str) -> GeneralOutcome {        let backup = match self.read_backup() {
            Ok(backup) => backup,
            Err(error) => return GeneralOutcome::Failed(AdapterError::new(error)),
        };
        match backup.backend.as_str() {
            "timeshift" => spawn(
                "pkexec",
                &[
                    "timeshift",
                    "--create",
                    "--scripted",
                    "--comments",
                    "Dragonfruit backup",
                ],
            ),
            "restic" => {
                if destination.is_empty() {
                    return GeneralOutcome::Failed(AdapterError::new(
                        "Choose a backup destination first.",
                    ));
                }
                if std::env::var_os("RESTIC_PASSWORD").is_none()
                    && std::env::var_os("RESTIC_PASSWORD_FILE").is_none()
                {
                    return GeneralOutcome::Failed(AdapterError::new(
                        "restic needs RESTIC_PASSWORD or RESTIC_PASSWORD_FILE.",
                    ));
                }
                spawn(
                    "restic",
                    &["-r", destination, "backup", &self.home.to_string_lossy()],
                )
            }
            "pika" => spawn("pika-backup", &[]),
            _ => GeneralOutcome::Absent,
        }
    }

    fn open_restore(&mut self) -> GeneralOutcome {
        if program_path("timeshift-gtk").is_some() {
            return spawn("timeshift-gtk", &[]);
        }
        if program_path("pika-backup").is_some() {
            return spawn("pika-backup", &[]);
        }
        GeneralOutcome::Absent
    }

    fn open_password_manager(&mut self, id: &str) -> GeneralOutcome {
        let Some(candidate) = PASSWORD_CANDIDATES
            .iter()
            .find(|candidate| candidate.id == id)
        else {
            return GeneralOutcome::Absent;
        };
        if program_path(candidate.launch).is_none() {
            return GeneralOutcome::Absent;
        }
        spawn(candidate.launch, &[])
    }
}

fn spawn(program: &str, args: &[&str]) -> GeneralOutcome {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => GeneralOutcome::Applied,
        Err(error) => GeneralOutcome::Failed(AdapterError::new(format!("{program}: {error}"))),
    }
}

fn is_denied(text: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("not authorized")
        || text.contains("authentication")
        || text.contains("dismissed")
        || text.contains("cancelled")
        || text.contains("canceled")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn pgrep(program: &str) -> bool {
    run("pgrep", &["-x", program])
        .map(|output| output.success())
        .unwrap_or(false)
}

fn read_secure_boot() -> String {
    let Ok(entries) = std::fs::read_dir("/sys/firmware/efi/efivars") else {
        return "Unknown".to_owned();
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("SecureBoot-") {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else {
            return "Unknown".to_owned();
        };
        // The first four bytes are the EFI variable attributes; the value
        // follows. SecureBoot is one byte: 1 on, 0 off.
        return match bytes.get(4) {
            Some(1) => "On".to_owned(),
            Some(0) => "Off".to_owned(),
            _ => "Unknown".to_owned(),
        };
    }
    "Unknown".to_owned()
}

/// Parse `timedatectl show`'s `Key=Value` lines.
pub fn parse_timedatectl(text: &str) -> TimeData {
    let mut time = TimeData::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Timezone" => time.timezone = value.to_owned(),
            "NTP" => time.ntp_enabled = truthy(value),
            "NTPSynchronized" => time.ntp_synchronized = truthy(value),
            "CanNTP" => time.can_ntp = truthy(value),
            _ => {}
        }
    }
    time
}

/// Parse `localectl status`.
pub fn parse_localectl(text: &str) -> LocaleData {
    let mut locale = LocaleData::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("System Locale:") {
            for assignment in rest.split_whitespace() {
                if let Some(value) = assignment.strip_prefix("LANG=") {
                    locale.locale = value.to_owned();
                } else if let Some(value) = assignment.strip_prefix("LC_TIME=") {
                    locale.formats_locale = value.to_owned();
                }
            }
        } else if let Some(rest) = line.strip_prefix("VC Keymap:") {
            locale.keymap = rest.trim().to_owned();
        }
    }
    if locale.formats_locale.is_empty() {
        locale.formats_locale = locale.locale.clone();
    }
    locale.language = locale
        .locale
        .split(['_', '.', '@'])
        .next()
        .unwrap_or("")
        .to_owned();
    locale
}

fn truthy(value: &str) -> bool {
    matches!(value, "yes" | "true" | "1" | "on")
}

/// Parse one desktop file's `[Desktop Entry]` keys.
fn parse_desktop(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    map
}

fn desktop_item(path: &Path, system: bool) -> Option<LoginItem> {
    let text = std::fs::read_to_string(path).ok()?;
    let map = parse_desktop(&text);
    if map.get("NoDisplay").map(String::as_str) == Some("true") {
        return None;
    }
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let enabled = map.get("Hidden").map(String::as_str) != Some("true")
        && map.get("X-GNOME-Autostart-enabled").map(String::as_str) != Some("false"); // df-allow-desktop-name: desktop-file key
    Some(LoginItem {
        id: id.clone(),
        name: map.get("Name").cloned().unwrap_or(id),
        command: map.get("Exec").cloned().unwrap_or_default(),
        source: if system { "system" } else { "user" }.to_owned(),
        path: path.to_string_lossy().into_owned(),
        enabled,
        system,
        can_toggle: true,
        broken: false,
        broken_reason: String::new(),
    })
}

/// Read the user and system autostart entries, with user overrides winning.
pub fn read_autostart(user_dir: &Path, system_dir: &Path) -> Vec<LoginItem> {
    let mut by_id: BTreeMap<String, LoginItem> = BTreeMap::new();
    if let Ok(entries) = std::fs::read_dir(system_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(item) = desktop_item(&path, true) {
                by_id.insert(item.id.clone(), item);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(item) = desktop_item(&path, false) {
                // A user file overrides the system entry of the same id, but
                // it remains a system entry for the toggle semantics (the
                // override is what makes enable/disable work).
                let system = by_id.get(&item.id).map(|old| old.system).unwrap_or(false);
                let mut item = item;
                item.system = system;
                item.source = if system { "system" } else { "user" }.to_owned();
                by_id.insert(item.id.clone(), item);
            }
        }
    }
    by_id.into_values().collect()
}

fn read_user_units(home: &Path) -> Result<Vec<UserUnit>, String> {
    let probe = run("systemctl", &["--user", "show-environment"])?;
    if !probe.success() {
        return Err("systemd user session is not available".to_owned());
    }
    let dirs = [
        home.join(".config/systemd/user"),
        PathBuf::from("/etc/systemd/user"),
    ];
    let mut units = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("service") {
                continue;
            }
            let Some(id) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            let name = std::fs::read_to_string(&path)
                .ok()
                .map(|text| parse_unit_description(&text))
                .filter(|description| !description.is_empty())
                .unwrap_or_else(|| id.clone());
            let enabled = run("systemctl", &["--user", "is-enabled", &id])
                .ok()
                .map(|output| output.stdout.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_owned());
            let active = run("systemctl", &["--user", "is-active", &id])
                .ok()
                .map(|output| output.stdout.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_owned());
            units.push(UserUnit {
                id: id.clone(),
                name,
                enabled: enabled.clone(),
                active,
                can_toggle: !matches!(
                    enabled.as_str(),
                    "static" | "masked" | "generated" | "transient"
                ),
            });
        }
    }
    units.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(units)
}

/// Parse a systemd unit file's `[Unit] Description=`.
pub fn parse_unit_description(text: &str) -> String {
    let mut in_unit = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_unit = line == "[Unit]";
            continue;
        }
        if in_unit {
            if let Some(value) = line.strip_prefix("Description=") {
                return value.trim().to_owned();
            }
        }
    }
    String::new()
}

fn set_autostart(
    config_home: &Path,
    system_dir: &Path,
    id: &str,
    enabled: bool,
) -> Result<(), String> {
    let user_dir = config_home.join("autostart");
    std::fs::create_dir_all(&user_dir).map_err(|error| error.to_string())?;
    let user_path = user_dir.join(format!("{id}.desktop"));
    let system_path = system_dir.join(format!("{id}.desktop"));
    let source_path = if user_path.exists() {
        user_path.clone()
    } else if system_path.exists() {
        system_path.clone()
    } else {
        return Err(format!("autostart entry {id} is gone"));
    };
    let text = std::fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
    let updated = set_desktop_key(&text, "Hidden", if enabled { "false" } else { "true" });
    let updated = set_desktop_key(
        &updated,
        "X-GNOME-Autostart-enabled", // df-allow-desktop-name: desktop-file key
        if enabled {
            // df-allow-desktop-name: desktop-file key
            "true"
        } else {
            "false"
        },
    );
    // A system entry is never edited in place: write the user override that
    // shadows it. A user entry is edited in place.
    let target = if source_path == system_path {
        user_path
    } else {
        source_path
    };
    std::fs::write(target, updated).map_err(|error| error.to_string())
}

/// Set one key in the `[Desktop Entry]` group, appending it when missing.
pub fn set_desktop_key(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_entry = false;
    let mut replaced = false;
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if in_entry {
            if let Some((existing, _)) = trimmed.split_once('=') {
                if existing.trim() == key {
                    *line = format!("{key}={value}");
                    replaced = true;
                }
            }
        }
    }
    if !replaced {
        // Append inside [Desktop Entry]: after the group header, or at the
        // end when the group is missing.
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim() == "[Desktop Entry]")
        {
            lines.insert(index + 1, format!("{key}={value}"));
        } else {
            lines.push("[Desktop Entry]".to_owned());
            lines.push(format!("{key}={value}"));
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Parse the `df -B1 -P` body into categories and volumes.
pub fn parse_df_usage(text: &str, home: &Path) -> StorageUsageData {
    let home = home.to_string_lossy();
    let mut volumes = Vec::new();
    let mut seen = BTreeSet::new();
    let mut categories: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_bytes = 0u64;
    let mut used_bytes = 0u64;
    let mut free_bytes = 0u64;
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            continue;
        }
        let device = fields[0];
        let total = fields[1].parse::<u64>().unwrap_or(0);
        let used = fields[2].parse::<u64>().unwrap_or(0);
        let available = fields[3].parse::<u64>().unwrap_or(0);
        let mount_point = fields[5..].join(" ");
        if skip_mount(&mount_point) {
            continue;
        }
        let category = category_for_mount(&mount_point, &home);
        let removable = mount_point.starts_with("/media")
            || mount_point.starts_with("/run/media")
            || mount_point.starts_with("/mnt");
        if seen.insert(device.to_owned()) {
            total_bytes = total_bytes.saturating_add(total);
            used_bytes = used_bytes.saturating_add(used);
            free_bytes = free_bytes.saturating_add(available);
            *categories.entry(category.clone()).or_insert(0) += used;
        }
        volumes.push(VolumeUsage {
            device: device.to_owned(),
            mount_point,
            category,
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            removable,
        });
    }
    let category_meta = [
        ("system", "System", "hard-drives"),
        ("home", "Home", "house"),
        ("media", "Media", "image"),
        ("other", "Other", "folder"),
    ];
    let categories = category_meta
        .into_iter()
        .filter_map(|(id, label, glyph)| {
            categories
                .get(id)
                .filter(|bytes| **bytes > 0)
                .map(|bytes| StorageCategory {
                    id: id.to_owned(),
                    label: label.to_owned(),
                    bytes: *bytes,
                    glyph: glyph.to_owned(),
                })
        })
        .collect();
    StorageUsageData {
        categories,
        volumes,
        total_bytes,
        used_bytes,
        free_bytes,
    }
}

fn skip_mount(mount_point: &str) -> bool {
    mount_point == "/proc"
        || mount_point.starts_with("/proc/")
        || mount_point == "/sys"
        || mount_point.starts_with("/sys/")
        || mount_point == "/dev"
        || mount_point.starts_with("/dev/")
        || mount_point == "/run"
        || mount_point.starts_with("/run/") && !mount_point.starts_with("/run/media")
}

fn category_for_mount(mount_point: &str, home: &str) -> String {
    if mount_point == home || mount_point.starts_with(&format!("{home}/")) {
        return "home".to_owned();
    }
    if mount_point == "/home" || mount_point.starts_with("/home/") {
        return "home".to_owned();
    }
    if mount_point.starts_with("/media")
        || mount_point.starts_with("/run/media")
        || mount_point.starts_with("/mnt")
    {
        return "media".to_owned();
    }
    if matches!(
        mount_point,
        "/" | "/boot" | "/boot/efi" | "/usr" | "/var" | "/etc" | "/opt" | "/srv"
    ) {
        return "system".to_owned();
    }
    "other".to_owned()
}

/// Parse `findmnt -n -o SOURCE,TARGET,FSTYPE,SIZE -b /`.
pub fn parse_startup(text: &str) -> StartupDiskData {
    let line = text.lines().next().unwrap_or("");
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return StartupDiskData::default();
    }
    // A btrfs root reports `SOURCE` as `/dev/mapper/luks-…[/root]`; the
    // bracketed suffix is the subvolume, not part of the block device.
    let device = fields[0].split('[').next().unwrap_or(fields[0]);
    StartupDiskData {
        device: device.to_owned(),
        mount_point: fields[1].to_owned(),
        filesystem: fields[2].to_owned(),
        total_bytes: fields[3].parse::<u64>().unwrap_or(0),
        boot_mode: "BIOS".to_owned(),
        secure_boot: "Unknown".to_owned(),
        ..StartupDiskData::default()
    }
}

fn read_timeshift_config() -> (String, String, String) {
    let Ok(text) = std::fs::read_to_string("/etc/timeshift/timeshift.json") else {
        return (String::new(), String::new(), String::new());
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (String::new(), String::new(), String::new());
    };
    let destination = json
        .get("backup_device")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            json.get("backup_device_uuid")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("")
        .to_owned();
    let schedule = if json
        .get("schedule_hourly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        "hourly"
    } else if json
        .get("schedule_daily")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        "daily"
    } else if json
        .get("schedule_weekly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        "weekly"
    } else if json
        .get("schedule_monthly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        "monthly"
    } else {
        "off"
    };
    let last_run = json
        .get("last_backup")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_owned();
    (destination, schedule.to_owned(), last_run)
}

struct PasswordCandidate {
    id: &'static str,
    label: &'static str,
    commands: &'static [&'static str],
    process_prefixes: &'static [&'static str],
    launch: &'static str,
    description: &'static str,
}

impl PasswordCandidate {
    fn matches_process(&self, process: &str) -> bool {
        let process = process.to_ascii_lowercase();
        self.process_prefixes
            .iter()
            .any(|prefix| process.starts_with(prefix))
    }
}

// The password managers the host can inventory. Detection is executable
// presence only; the pane never guesses that a manager has credentials.
const PASSWORD_CANDIDATES: &[PasswordCandidate] = &[
    PasswordCandidate {
        id: "keyring",
        label: "Desktop Keyring",
        commands: &["gnome-keyring-daemon"], // df-allow-desktop-name: executable detection
        process_prefixes: &["gnome-keyring"], // df-allow-desktop-name: process detection
        launch: "seahorse",
        description: "The desktop keyring and Secret Service provider.",
    },
    PasswordCandidate {
        id: "kwallet",
        label: "KWallet",
        commands: &["kwalletd6", "kwalletd5"],
        process_prefixes: &["kwalletd"],
        launch: "kwalletmanager5",
        description: "The desktop wallet and Secret Service provider.",
    },
    PasswordCandidate {
        id: "keepassxc",
        label: "KeePassXC",
        commands: &["keepassxc"],
        process_prefixes: &["keepassxc"],
        launch: "keepassxc",
        description: "A local password database with browser integration.",
    },
    PasswordCandidate {
        id: "bitwarden",
        label: "Bitwarden",
        commands: &["bitwarden"],
        process_prefixes: &["bitwarden"],
        launch: "bitwarden",
        description: "A hosted password manager with a desktop client.",
    },
];

/// The Secret Service name owner's process name, when the session bus
/// publishes it.
pub fn secret_service_owner() -> Option<String> {
    let output = run("busctl", &["--user", "--no-pager", "list", "--no-legend"]).ok()?;
    if !output.success() {
        return None;
    }
    parse_secret_service_owner(&output.stdout)
}

/// Parse `busctl --user list` output for the Secret Service owner.
pub fn parse_secret_service_owner(text: &str) -> Option<String> {
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("org.freedesktop.secrets") {
            continue;
        }
        // NAME PID PROCESS USER CONNECTION UNIT ACTIVE
        let _pid = fields.next();
        return fields.next().map(str::to_owned);
    }
    None
}

/// Read the Secret Service collections' lock state. `Some(true)` means every
/// collection is locked, `Some(false)` means at least one is unlocked, and
/// `None` means the service did not answer.
fn read_keyring_lock() -> Option<bool> {
    let collections = run(
        "gdbus",
        &[
            "call",
            "--session",
            "--dest",
            "org.freedesktop.secrets",
            "--object-path",
            "/org/freedesktop/secrets",
            "--method",
            "org.freedesktop.DBus.Properties.Get",
            "org.freedesktop.Secret.Service",
            "Collections",
        ],
    )
    .ok()?;
    if !collections.success() {
        return None;
    }
    let paths = collection_paths(&collections.stdout);
    if paths.is_empty() {
        return None;
    }
    let mut saw_answer = false;
    for path in paths {
        let locked = run(
            "gdbus",
            &[
                "call",
                "--session",
                "--dest",
                "org.freedesktop.secrets",
                "--object-path",
                &path,
                "--method",
                "org.freedesktop.DBus.Properties.Get",
                "org.freedesktop.Secret.Collection",
                "Locked",
            ],
        );
        let Ok(locked) = locked else { continue };
        if !locked.success() {
            continue;
        }
        saw_answer = true;
        if locked.stdout.contains("<false>") {
            return Some(false);
        }
    }
    saw_answer.then_some(true)
}

/// The collection object paths in a `gdbus` `Collections` reply.
pub fn collection_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for part in text.split('\'') {
        if part.starts_with("/org/freedesktop/secrets/collection/") {
            paths.push(part.to_owned());
        }
    }
    paths
}

struct SharingCandidate {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    units: &'static [&'static str],
    binaries: &'static [&'static str],
}

const SHARING_CANDIDATES: &[SharingCandidate] = &[
    SharingCandidate {
        id: "file",
        name: "File Sharing",
        description: "Share folders with other computers using SMB.",
        units: &["smb.service", "smbd.service"],
        binaries: &["smbd"],
    },
    SharingCandidate {
        id: "screen",
        name: "Screen Sharing",
        description: "Share this desktop with a remote viewer.",
        units: &["gnome-remote-desktop.service", "wayvnc.service"], // df-allow-desktop-name: third-party service id
        binaries: &["gnome-remote-desktop", "wayvnc"], // df-allow-desktop-name: third-party executable
    },
    SharingCandidate {
        id: "media",
        name: "Media Sharing",
        description: "Share media with DLNA devices.",
        units: &["rygel.service", "minidlna.service"],
        binaries: &["rygel", "minidlnad"],
    },
    SharingCandidate {
        id: "printer",
        name: "Printer Sharing",
        description: "Share printers with other computers.",
        units: &["cups.service"],
        binaries: &["cupsd"],
    },
    SharingCandidate {
        id: "remoteLogin",
        name: "Remote Login",
        description: "Allow remote login with SSH.",
        units: &["sshd.service"],
        binaries: &["sshd"],
    },
    SharingCandidate {
        id: "remoteManagement",
        name: "Remote Management",
        description: "Manage this computer with Cockpit.",
        units: &["cockpit.socket"],
        binaries: &["cockpit-bridge"],
    },
];

/// The reason an autostart entry cannot run, or `(false, "")`.
///
/// A command is executable when its first token is an absolute/relative path
/// that exists or a bare program on `PATH`. A desktop file may also use the
/// `sh -c` form; the first token is still the interpreter, so the entry is
/// only broken when the interpreter itself is missing.
pub fn broken_login_item(command: &str) -> (bool, String) {
    let program = command.split_whitespace().next().unwrap_or("");
    if program.is_empty() {
        return (true, "The entry has no command.".to_owned());
    }
    if program.contains('/') {
        let path = Path::new(program);
        if path.is_file() {
            return (false, String::new());
        }
        return (
            true,
            format!("The command {program} is missing."),
        );
    }
    if program_path(program).is_some() {
        (false, String::new())
    } else {
        (
            true,
            format!("The command {program} is not installed."),
        )
    }
}

/// The installed applications that are not already login items.
///
/// The picker offers the same `.desktop` applications the session already
/// knows; it never invents a name. System and user application directories are
/// read in the same precedence order the desktop uses.
pub fn read_login_apps(config_home: &Path, autostart: &[LoginItem]) -> Vec<LoginApp> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        config_home.join("../local/share/applications"),
    ];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    }
    let existing: BTreeSet<&str> = autostart.iter().map(|item| item.id.as_str()).collect();
    let mut by_id: BTreeMap<String, LoginApp> = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let map = parse_desktop(&text);
            if map.get("Type").map(String::as_str) != Some("Application") {
                continue;
            }
            if map.get("NoDisplay").map(String::as_str) == Some("true") {
                continue;
            }
            if map.get("Hidden").map(String::as_str) == Some("true") {
                continue;
            }
            let Some(id) = path.file_stem().map(|stem| stem.to_string_lossy().into_owned())
            else {
                continue;
            };
            if existing.contains(id.as_str()) || by_id.contains_key(&id) {
                continue;
            }
            let Some(command) = map.get("Exec").filter(|exec| !exec.trim().is_empty())
            else {
                continue;
            };
            by_id.insert(
                id.clone(),
                LoginApp {
                    id,
                    name: map.get("Name").cloned().unwrap_or_default(),
                    command: command.clone(),
                },
            );
        }
    }
    let mut apps: Vec<LoginApp> = by_id.into_values().collect();
    apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    apps
}

/// The portal backend services the Extensions group lists, grouped by the
/// surface each one serves. Only services actually on the session bus are
/// returned; an absent portal is an empty list with the pane's note.
pub fn read_portal_extensions() -> Vec<ExtensionEntry> {
    let output = match run("busctl", &["--user", "--no-pager", "list", "--no-legend"]) {
        Ok(output) if output.success() => output,
        _ => return Vec::new(),
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in output.stdout.lines() {
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        let Some(suffix) = name.strip_prefix("org.freedesktop.impl.portal.") else {
            continue;
        };
        if suffix.is_empty() || suffix.contains('.') {
            continue;
        }
        if !seen.insert(name.to_owned()) {
            continue;
        }
        out.push(ExtensionEntry {
            id: name.to_owned(),
            name: suffix.to_owned(),
            category: "Portal backends".to_owned(),
            detail: "A desktop portal backend installed on this system.".to_owned(),
        });
    }
    out.sort_by(|left, right| left.name.cmp(&right.name));
    out
}

/// The computer's local hostname. `hostnamectl` is authoritative; the kernel
/// name and `/etc/hostname` are honest fallbacks on a host without it.
pub fn read_hostname() -> String {
    if let Ok(output) = run("hostnamectl", &["hostname"]) {
        if output.success() {
            let value = output.stdout.trim();
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    if let Ok(output) = run("hostname", &[]) {
        if output.success() {
            let value = output.stdout.trim();
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

/// The read-only options a sharing service reports.
pub fn sharing_details(id: &str, unit: &str, hostname: &str) -> Vec<ServiceDetail> {
    let mut details = Vec::new();
    if !unit.is_empty() {
        details.push(ServiceDetail {
            label: "Unit".to_owned(),
            value: unit.to_owned(),
        });
    }
    if !hostname.is_empty() {
        details.push(ServiceDetail {
            label: "Address".to_owned(),
            value: hostname.to_owned(),
        });
    }
    match id {
        "remoteLogin" => details.push(ServiceDetail {
            label: "Port".to_owned(),
            value: read_sshd_port(),
        }),
        "file" => details.push(ServiceDetail {
            label: "Shares".to_owned(),
            value: read_samba_share_count().to_string(),
        }),
        "printer" => details.push(ServiceDetail {
            label: "Queues".to_owned(),
            value: read_cups_queue_count().to_string(),
        }),
        "screen" => details.push(ServiceDetail {
            label: "Port".to_owned(),
            value: "5900".to_owned(),
        }),
        _ => {}
    }
    details
}

fn read_sshd_port() -> String {
    let Ok(text) = std::fs::read_to_string("/etc/ssh/sshd_config") else {
        return "22".to_owned();
    };
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("Port ") {
            let value = value.trim();
            if !value.is_empty() {
                return value.to_owned();
            }
        }
    }
    "22".to_owned()
}

fn read_samba_share_count() -> usize {
    let Ok(text) = std::fs::read_to_string("/etc/samba/smb.conf") else {
        return 0;
    };
    text.lines()
        .filter(|line| {
            let line = line.trim();
            line.starts_with('[')
                && !line.starts_with("[global]")
                && !line.starts_with("[homes]")
                && !line.starts_with("[printers]")
                && !line.starts_with("[print$]")
        })
        .count()
}

fn read_cups_queue_count() -> usize {
    let Ok(output) = run("lpstat", &["-a"]) else {
        return 0;
    };
    output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

/// Parse `lsblk -J -b -o NAME,SIZE,TYPE,MODEL,LABEL,MOUNTPOINT` into the
/// Startup Disk list, boot disk first.
pub fn parse_lsblk_disks(text: &str, boot_disk: &str) -> Vec<DiskEntry> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(devices) = json.get("blockdevices").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for device in devices {
        if device.get("type").and_then(|value| value.as_str()) != Some("disk") {
            continue;
        }
        let Some(name) = device.get("name").and_then(|value| value.as_str()) else {
            continue;
        };
        let device_path = format!("/dev/{name}");
        let size_bytes = device
            .get("size")
            .and_then(|value| value.as_u64())
            .or_else(|| {
                device
                    .get("size")
                    .and_then(|value| value.as_str())
                    .and_then(|value| value.parse::<u64>().ok())
            })
            .unwrap_or(0);
        let model = device
            .get("model")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_owned();
        let label = device
            .get("label")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_owned();
        let mut mounted = device
            .get("mountpoint")
            .and_then(|value| value.as_str())
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if let Some(children) = device.get("children").and_then(|value| value.as_array()) {
            mounted = mounted
                || children.iter().any(|child| {
                    child
                        .get("mountpoint")
                        .and_then(|value| value.as_str())
                        .map(|value| !value.is_empty())
                        .unwrap_or(false)
                });
        }
        out.push(DiskEntry {
            boot: device_path == boot_disk,
            device: device_path,
            label: if label.is_empty() { model.clone() } else { label },
            model,
            size_bytes,
            mounted,
        });
    }
    out.sort_by(|left, right| right.boot.cmp(&left.boot));
    out
}

/// Write the user autostart entry that opens an installed application.
pub fn add_autostart(config_home: &Path, app_id: &str) -> Result<(), String> {
    if app_id.trim().is_empty() || app_id.contains('/') {
        return Err("choose an application first".to_owned());
    }
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    }
    let path = dirs
        .iter()
        .map(|dir| dir.join(format!("{app_id}.desktop")))
        .find(|path| path.is_file())
        .ok_or_else(|| format!("application {app_id} is not installed"))?;
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let map = parse_desktop(&text);
    let Some(command) = map.get("Exec").filter(|exec| !exec.trim().is_empty()) else {
        return Err(format!("application {app_id} has no command"));
    };
    let name = map.get("Name").cloned().unwrap_or_else(|| app_id.to_owned());
    let user_dir = config_home.join("autostart");
    std::fs::create_dir_all(&user_dir).map_err(|error| error.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={command}\n\
         X-GNOME-Autostart-enabled=true\n" // df-allow-desktop-name: desktop-file key
    );
    std::fs::write(user_dir.join(format!("{app_id}.desktop")), entry)
        .map_err(|error| error.to_string())
}

/// Remove a login item. A user entry is deleted; a system entry gets a user
/// override with `Hidden=true`, which is how XDG autostart removes an entry the
/// user does not own.
pub fn remove_autostart(
    config_home: &Path,
    system_dir: &Path,
    id: &str,
) -> Result<(), String> {
    if id.trim().is_empty() || id.contains('/') {
        return Err("no login item selected".to_owned());
    }
    let user_dir = config_home.join("autostart");
    let user_path = user_dir.join(format!("{id}.desktop"));
    let system_path = system_dir.join(format!("{id}.desktop"));
    if system_path.is_file() {
        std::fs::create_dir_all(&user_dir).map_err(|error| error.to_string())?;
        let text = std::fs::read_to_string(&user_path).unwrap_or_else(|_| {
            std::fs::read_to_string(&system_path).unwrap_or_default()
        });
        let updated = set_desktop_key(&text, "Hidden", "true");
        return std::fs::write(user_path, updated).map_err(|error| error.to_string());
    }
    if user_path.is_file() {
        return std::fs::remove_file(user_path).map_err(|error| error.to_string());
    }
    Err(format!("login item {id} is gone"))
}

/// A hostname is a single DNS label: 1-63 characters, letters/digits/hyphens,
/// and it cannot start or end with a hyphen.
pub fn validate_hostname(hostname: &str) -> Result<(), String> {
    let hostname = hostname.trim();
    if hostname.is_empty() {
        return Err("The hostname cannot be empty.".to_owned());
    }
    if hostname.len() > 63 {
        return Err("The hostname cannot be longer than 63 characters.".to_owned());
    }
    if hostname.starts_with('-') || hostname.ends_with('-') {
        return Err("The hostname cannot start or end with a hyphen.".to_owned());
    }
    if !hostname
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("The hostname can only contain letters, digits, and hyphens.".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timedatectl_parses_the_four_properties() {
        let time =
            parse_timedatectl("Timezone=Europe/Berlin\nNTP=yes\nNTPSynchronized=no\nCanNTP=yes\n");
        assert_eq!(time.timezone, "Europe/Berlin");
        assert!(time.ntp_enabled);
        assert!(!time.ntp_synchronized);
        assert!(time.can_ntp);
    }

    #[test]
    fn localectl_parses_the_locale_and_keymap() {
        let locale = parse_localectl(
            "   System Locale: LANG=de_DE.UTF-8 LC_TIME=en_US.UTF-8\n\
                 VC Keymap: de\n\
                X11 Layout: de\n",
        );
        assert_eq!(locale.locale, "de_DE.UTF-8");
        assert_eq!(locale.formats_locale, "en_US.UTF-8");
        assert_eq!(locale.language, "de");
        assert_eq!(locale.keymap, "de");
    }

    #[test]
    fn a_desktop_entry_is_read_and_toggled() {
        let map = parse_desktop(
            "[Desktop Entry]\nType=Application\nName=Agent\nExec=agent --start\nHidden=true\n",
        );
        assert_eq!(map.get("Name").map(String::as_str), Some("Agent"));
        let updated = set_desktop_key("[Desktop Entry]\nName=Agent\n", "Hidden", "true");
        assert!(updated.contains("Hidden=true"));
        let updated = set_desktop_key(&updated, "Hidden", "false");
        assert!(updated.contains("Hidden=false"));
        assert!(!updated.contains("Hidden=true"));
    }

    #[test]
    fn df_usage_is_grouped_and_deduplicates_devices() {
        let usage = parse_df_usage(
            "Filesystem 1-blocks Used Available Capacity Mounted on\n\
             /dev/nvme0n1p3 512000000000 480000000000 32000000000 94% /\n\
             /dev/nvme0n1p3 512000000000 480000000000 32000000000 94% /home\n\
             /dev/sdb1 20000000000 10000000000 10000000000 50% /run/media/user/Photos\n",
            Path::new("/home/user"),
        );
        assert_eq!(usage.volumes.len(), 3);
        assert_eq!(usage.total_bytes, 532_000_000_000);
        assert_eq!(usage.used_bytes, 490_000_000_000);
        // `/home` is on the same device as `/`, so it is not a second
        // category total: the chart never double-counts one filesystem.
        let ids: Vec<&str> = usage.categories.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["system", "media"]);
        assert!(usage.volumes[2].removable);

        let split = parse_df_usage(
            "Filesystem 1-blocks Used Available Capacity Mounted on\n\
             /dev/nvme0n1p3 512000000000 300000000000 212000000000 59% /\n\
             /dev/nvme0n1p4 400000000000 100000000000 300000000000 25% /home\n",
            Path::new("/home/user"),
        );
        let ids: Vec<&str> = split.categories.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["system", "home"]);
    }

    #[test]
    fn startup_disk_parses_findmnt() {
        let disk = parse_startup("/dev/nvme0n1p3 / btrfs 509332160512\n");
        assert_eq!(disk.device, "/dev/nvme0n1p3");
        assert_eq!(disk.filesystem, "btrfs");
        assert_eq!(disk.total_bytes, 509_332_160_512);
    }

    #[test]
    fn startup_disk_strips_a_btrfs_subvolume_source() {
        let disk = parse_startup(
            "/dev/mapper/luks-8bb932c4-a425-4856-96c0-689339748a0e[/root] / btrfs 509315383296\n",
        );
        assert_eq!(
            disk.device,
            "/dev/mapper/luks-8bb932c4-a425-4856-96c0-689339748a0e"
        );
        assert_eq!(disk.mount_point, "/");
        assert_eq!(disk.total_bytes, 509_315_383_296);
    }

    #[test]
    fn startup_disk_lists_lsblk_disks_boot_first() {
        let disks = parse_lsblk_disks(
            r#"{"blockdevices":[
                {"name":"sdb","size":"1000000000000","type":"disk","model":"USB Disk","label":"Backup","mountpoint":null},
                {"name":"nvme0n1","size":"512000000000","type":"disk","model":"Example NVMe","label":null,
                 "children":[{"name":"nvme0n1p3","mountpoint":"/"}]}
            ]}"#,
            "/dev/nvme0n1",
        );
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].device, "/dev/nvme0n1");
        assert!(disks[0].boot);
        assert!(disks[0].mounted);
        assert_eq!(disks[0].size_bytes, 512_000_000_000);
        assert_eq!(disks[1].device, "/dev/sdb");
        assert!(!disks[1].boot);
    }

    #[test]
    fn a_hostname_is_one_dns_label() {
        assert!(validate_hostname("dragon").is_ok());
        assert!(validate_hostname("dragon-fruit-2").is_ok());
        assert!(validate_hostname("").is_err());
        assert!(validate_hostname("-dragon").is_err());
        assert!(validate_hostname("dragon-").is_err());
        assert!(validate_hostname("dra gon").is_err());
        assert!(validate_hostname("dragon.example").is_err());
    }

    #[test]
    fn a_missing_command_marks_a_login_item_broken() {
        let (broken, reason) = broken_login_item("/definitely/not/a/program --start");
        assert!(broken);
        assert!(reason.contains("missing"));
        let (broken, reason) = broken_login_item("");
        assert!(broken);
        assert!(reason.contains("no command"));
    }

    #[test]
    fn a_user_login_item_is_removed() {
        let root = std::env::temp_dir().join(format!(
            "df-general-t270-{}",
            std::process::id()
        ));
        let config = root.join("config");
        let system = root.join("system-autostart");
        std::fs::create_dir_all(&system).unwrap();
        let config_home = config.clone();
        let user_dir = config_home.join("autostart");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::write(
            user_dir.join("org.example.Old.desktop"),
            "[Desktop Entry]\nType=Application\nName=Old\nExec=/usr/bin/old\n",
        )
        .unwrap();
        remove_autostart(&config_home, &system, "org.example.Old").unwrap();
        assert!(!user_dir.join("org.example.Old.desktop").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unit_description_parses() {        assert_eq!(
            parse_unit_description("[Unit]\nDescription=Backup user files\n[Service]\n"),
            "Backup user files"
        );
    }

    #[test]
    fn the_secret_service_owner_is_read_from_busctl() {
        let owner = parse_secret_service_owner(
            "NAME PID PROCESS USER CONNECTION UNIT ACTIVE\n\
             org.freedesktop.secrets 421 keyring-d user :1.5 - active\n\
             org.example.Other 7 other user :1.9 - active\n",
        );
        assert_eq!(owner.as_deref(), Some("keyring-d"));
        assert_eq!(parse_secret_service_owner("NAME PID PROCESS\n"), None);
    }

    #[test]
    fn collection_paths_are_extracted_from_the_gdbus_reply() {
        let paths = collection_paths(
            "(<[objectpath '/org/freedesktop/secrets/collection/login', \
             objectpath '/org/freedesktop/secrets/collection/session']>,)",
        );
        assert_eq!(
            paths,
            vec![
                "/org/freedesktop/secrets/collection/login".to_owned(),
                "/org/freedesktop/secrets/collection/session".to_owned()
            ]
        );
        assert!(collection_paths("(<>,)").is_empty());
    }

    #[test]
    fn a_password_candidate_matches_its_process_prefix() {
        let keepass = &PASSWORD_CANDIDATES[2];
        assert!(keepass.matches_process("keepassxc"));
        assert!(!keepass.matches_process("keyring-d"));
    }

    #[test]
    fn the_live_read_is_independent_per_sub_pane() {
        // The real host has at least one of the tools; a missing one must not
        // hide the others. This is a live smoke test, so it only asserts the
        // shape, not a specific host's values.
        let mut host = HostGeneral::new();
        let snapshot = host.read().unwrap();
        if let Some(snapshot) = snapshot {
            assert!(snapshot.has_data());
        }
    }
}
