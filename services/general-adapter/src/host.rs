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
    BackupBackend, BackupData, GeneralSnapshot, LocaleData, LoginData, LoginItem, SharingData,
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
        Ok(LoginData {
            autostart,
            user_units,
            extensions_available: false,
            extensions_note: "Extensions are managed by the desktop portal and installed apps."
                .to_owned(),
        })
    }

    fn read_sharing(&self) -> Result<SharingData, String> {
        let mut services = Vec::new();
        let mut saw_systemctl = false;
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
            });
        }
        if services.is_empty() && !saw_systemctl {
            return Err("systemctl is not available".to_owned());
        }
        Ok(SharingData { services })
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
        Ok(disk)
    }

    fn write(&self, program: &str, args: &[&str]) -> GeneralOutcome {
        match run(program, args) {
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
        }
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

    fn run_backup(&mut self, destination: &str) -> GeneralOutcome {
        let backup = match self.read_backup() {
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
    fn unit_description_parses() {
        assert_eq!(
            parse_unit_description("[Unit]\nDescription=Backup user files\n[Service]\n"),
            "Backup user files"
        );
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
