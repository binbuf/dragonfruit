// SPDX-License-Identifier: MIT
//! The Security half of the Privacy and Security adapter (T228).
//!
//! Security is a **host-stack projection**, not a second policy engine:
//!
//! * **Software sources** — the distribution package manager plus the Flatpak
//!   remotes (`flatpak remotes --show-disabled`). A Flatpak remote is the one
//!   writable source; distribution repositories are read-only system policy.
//! * **Disk encryption** — `lsblk -J` reports whether the running tree sits on
//!   a LUKS/crypt device. Read-only: unlocking and recovery belong to the
//!   distribution's cryptsetup path, never to Settings.
//! * **Hardening** — SELinux/AppArmor enforcement and Secure Boot from the
//!   host tools. Read-only status; there is no fake "Lockdown Mode" toggle.
//! * **Extensions** — the installed `xdg-desktop-portal` backend descriptors
//!   (`.portal` key files), which are the host's closest equivalent to macOS
//!   system extensions. Read-only: backend selection is system policy.
//! * **Advanced** — a compact host-stack inventory (polkit, firewall, package
//!   manager, Flatpak, portal, online accounts).
//!
//! Each sub-read is optional. A missing tool disables only its own row; the
//! adapter is `Unavailable` only when no security owner answers at all. That
//! is a normal hidden state, never a startup blocker. The only write is
//! [`SecuritySource::set_source_enabled`] for a Flatpak remote.

use std::collections::VecDeque;
use std::path::PathBuf;

use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

/// One software source (a distribution repository or a Flatpak remote).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoftwareSourceData {
    /// Stable id (`distro:dnf`, `flatpak:flathub:system`).
    pub id: String,
    /// The display name.
    pub name: String,
    /// `distro`, `flatpak`, …
    pub kind: String,
    /// Whether the source is enabled.
    pub enabled: bool,
    /// Whether the pane may toggle it.
    pub writable: bool,
    /// A one-line note (system/user scope, policy).
    pub note: String,
}

/// The software-sources sub-read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoftwareSourcesData {
    /// Whether any source owner answered.
    pub available: bool,
    /// The distribution package manager (`dnf`, `apt`, `pacman`, or empty).
    pub manager: String,
    /// Every source, distribution first.
    pub sources: Vec<SoftwareSourceData>,
}

/// One encrypted (or LUKS-labelled) block device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncryptionDeviceData {
    /// The device name (`nvme0n1p3`, `luks-…`).
    pub name: String,
    /// `crypt`, `part`, `disk`, …
    pub kind: String,
    /// The filesystem type, when known.
    pub fstype: String,
    /// The mount points, when mounted.
    pub mountpoints: Vec<String>,
}

/// The disk-encryption sub-read. Read-only by design.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncryptionData {
    /// Whether `lsblk` answered.
    pub available: bool,
    /// Whether any LUKS/crypt device is present.
    pub encrypted: bool,
    /// The one-line status (`Encrypted (LUKS)`, `Not encrypted`, `Unknown`).
    pub label: String,
    /// The honest explanation.
    pub note: String,
    /// The encrypted devices, when any.
    pub devices: Vec<EncryptionDeviceData>,
}

/// The hardening sub-read. Read-only status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HardeningData {
    /// Whether any hardening probe answered.
    pub available: bool,
    /// `enforcing`, `permissive`, `disabled`, or `standard`.
    pub mode: String,
    /// The one-line label.
    pub label: String,
    /// The detail line (SELinux/AppArmor/Secure Boot).
    pub detail: String,
}

/// One installed portal backend descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtensionData {
    /// The `.portal` file stem.
    pub id: String,
    /// The display name (the stem unless the file carries one).
    pub name: String,
    /// The backend's D-Bus name.
    pub dbus_name: String,
    /// The portal interfaces it implements.
    pub interfaces: Vec<String>,
    /// The `UseIn` value.
    pub use_in: String,
    /// The descriptor path.
    pub path: String,
}

/// One row in the Advanced sheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdvancedRowData {
    /// Stable id (`polkit`, `firewall`, `secure-boot`, …).
    pub id: String,
    /// The display label.
    pub label: String,
    /// The live value.
    pub value: String,
    /// The detail/owner line.
    pub detail: String,
    /// `ok`, `off`, or `absent`.
    pub state: String,
}

/// The raw Security read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityData {
    /// Software sources.
    pub software_sources: SoftwareSourcesData,
    /// Disk encryption.
    pub encryption: EncryptionData,
    /// Hardening status.
    pub hardening: HardeningData,
    /// Installed portal extensions.
    pub extensions: Vec<ExtensionData>,
    /// The Advanced sheet rows.
    pub advanced: Vec<AdvancedRowData>,
}

/// The result of one security write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityOutcome {
    /// The owner accepted the request.
    Applied,
    /// Policy refused the request.
    Denied(String),
    /// The owner is absent.
    Absent,
    /// The request failed.
    Failed(AdapterError),
}

impl SecurityOutcome {
    /// Whether the owner accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, SecurityOutcome::Applied)
    }

    /// The refusal or failure text.
    pub fn message(&self) -> Option<&str> {
        match self {
            SecurityOutcome::Denied(message) => Some(message),
            SecurityOutcome::Failed(error) => Some(error.message()),
            SecurityOutcome::Applied | SecurityOutcome::Absent => None,
        }
    }
}

/// Reads the Security host stack and performs its one write.
pub trait SecuritySource {
    /// One read of every security sub-owner.
    fn read_security(&mut self) -> Result<Option<SecurityData>, AdapterError>;

    /// Enable or disable one writable software source. One explicit write.
    fn set_source_enabled(&mut self, _id: &str, _enabled: bool) -> SecurityOutcome {
        SecurityOutcome::Absent
    }
}

/// The typed Security snapshot the pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecuritySnapshot {
    data: SecurityData,
}

impl SecuritySnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &SecurityData) -> Self {
        SecuritySnapshot { data: data.clone() }
    }

    /// The software sources.
    pub fn software_sources(&self) -> &SoftwareSourcesData {
        &self.data.software_sources
    }

    /// The disk-encryption status.
    pub fn encryption(&self) -> &EncryptionData {
        &self.data.encryption
    }

    /// The hardening status.
    pub fn hardening(&self) -> &HardeningData {
        &self.data.hardening
    }

    /// The installed portal extensions.
    pub fn extensions(&self) -> &[ExtensionData] {
        &self.data.extensions
    }

    /// The Advanced sheet rows.
    pub fn advanced(&self) -> &[AdvancedRowData] {
        &self.data.advanced
    }

    /// How many software sources are enabled.
    pub fn enabled_source_count(&self) -> usize {
        self.data
            .software_sources
            .sources
            .iter()
            .filter(|source| source.enabled)
            .count()
    }

    /// The pane header label.
    pub fn label(&self) -> String {
        if !self.data.encryption.available {
            return "Security".to_owned();
        }
        self.data.encryption.label.clone()
    }

    /// The design-system glyph the pane header draws.
    pub fn glyph(&self) -> &'static str {
        "shield-check"
    }
}

/// The Security adapter: one read path over the shared contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityAdapter<S> {
    source: S,
    state: AdapterState<SecuritySnapshot>,
    subscription: Subscription,
    changes: VecDeque<()>,
}

impl<S: SecuritySource> SecurityAdapter<S> {
    /// A fresh adapter over `source`, absent until the first refresh.
    pub fn new(source: S) -> Self {
        SecurityAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
            changes: VecDeque::new(),
        }
    }

    /// Read the security host stack once.
    pub fn refresh(&mut self) {
        match self.source.read_security() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(SecuritySnapshot::from_data(&data));
                self.subscription.changed();
                self.changes.push_back(());
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

    /// Enable or disable one writable source.
    pub fn set_source_enabled(&mut self, id: &str, enabled: bool) -> SecurityOutcome {
        self.source.set_source_enabled(id, enabled)
    }

    /// The live snapshot, when an owner answered.
    pub fn snapshot(&self) -> Option<&SecuritySnapshot> {
        self.state.snapshot()
    }

    /// The transport.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed.
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the change count since the last drain.
    pub fn drain_changes(&mut self) -> usize {
        self.changes.drain(..).count()
    }
}

impl<S: SecuritySource> Adapter for SecurityAdapter<S> {
    type Snapshot = SecuritySnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::SECURITY
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

/// Read the live security host stack. Each sub-read is independent; the whole
/// read is `Ok(None)` only when no owner answers.
pub fn read_host_security() -> Result<Option<SecurityData>, AdapterError> {
    let software_sources = read_software_sources();
    let encryption = read_encryption();
    let hardening = read_hardening();
    let extensions = read_extensions();
    let advanced = read_advanced(&software_sources, &encryption, &hardening);

    let any = software_sources.available
        || encryption.available
        || hardening.available
        || !extensions.is_empty()
        || !advanced.is_empty();
    if !any {
        return Ok(None);
    }
    Ok(Some(SecurityData {
        software_sources,
        encryption,
        hardening,
        extensions,
        advanced,
    }))
}

/// A deterministic security fixture for CI, pane tests, and captures.
pub fn security_fixture() -> SecurityData {
    SecurityData {
        software_sources: SoftwareSourcesData {
            available: true,
            manager: "dnf".to_owned(),
            sources: vec![
                SoftwareSourceData {
                    id: "distro:dnf".to_owned(),
                    name: "Fedora repositories".to_owned(),
                    kind: "distro".to_owned(),
                    enabled: true,
                    writable: false,
                    note: "Distribution repositories are managed by system policy.".to_owned(),
                },
                SoftwareSourceData {
                    id: "flatpak:flathub:system".to_owned(),
                    name: "Flathub (system)".to_owned(),
                    kind: "flatpak".to_owned(),
                    enabled: true,
                    writable: true,
                    note: "System Flatpak remote".to_owned(),
                },
            ],
        },
        encryption: EncryptionData {
            available: true,
            encrypted: true,
            label: "Encrypted (LUKS)".to_owned(),
            note: "The running tree is on a LUKS device. Unlocking and recovery stay with the distribution's cryptsetup tools.".to_owned(),
            devices: vec![EncryptionDeviceData {
                name: "luks-8bb932c4".to_owned(),
                kind: "crypt".to_owned(),
                fstype: "btrfs".to_owned(),
                mountpoints: vec!["/".to_owned(), "/home".to_owned()],
            }],
        },
        hardening: HardeningData {
            available: true,
            mode: "enforcing".to_owned(),
            label: "Enforcing".to_owned(),
            detail: "SELinux is enforcing; Secure Boot is disabled.".to_owned(),
        },
        extensions: vec![
            ExtensionData {
                id: "dragonfruit".to_owned(),
                name: "Dragonfruit".to_owned(),
                dbus_name: "org.freedesktop.impl.portal.desktop.dragonfruit".to_owned(),
                interfaces: vec![
                    "org.freedesktop.impl.portal.Settings".to_owned(),
                    "org.freedesktop.impl.portal.FileChooser".to_owned(),
                ],
                use_in: "dragonfruit".to_owned(),
                path: "/usr/share/xdg-desktop-portal/portals/dragonfruit.portal".to_owned(),
            },
            ExtensionData {
                id: "gtk".to_owned(),
                name: "GTK".to_owned(),
                dbus_name: "org.freedesktop.impl.portal.desktop.gtk".to_owned(),
                interfaces: vec!["org.freedesktop.impl.portal.FileChooser".to_owned()],
                use_in: "gnome".to_owned(), // df-allow-desktop-name: fixture UseIn value
                path: "/usr/share/xdg-desktop-portal/portals/gtk.portal".to_owned(),
            },
        ],
        advanced: vec![
            AdvancedRowData {
                id: "polkit".to_owned(),
                label: "Authorization".to_owned(),
                value: "polkit".to_owned(),
                detail: "Privileged actions use the session polkit agent.".to_owned(),
                state: "ok".to_owned(),
            },
            AdvancedRowData {
                id: "firewall".to_owned(),
                label: "Firewall".to_owned(),
                value: "firewalld".to_owned(),
                detail: "The Network pane owns the firewall controls.".to_owned(),
                state: "ok".to_owned(),
            },
            AdvancedRowData {
                id: "secure-boot".to_owned(),
                label: "Secure Boot".to_owned(),
                value: "Disabled".to_owned(),
                detail: "Reported by mokutil.".to_owned(),
                state: "off".to_owned(),
            },
        ],
    }
}

fn run(command: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path).any(|dir| {
                let candidate = dir.join(command);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

fn package_manager() -> &'static str {
    for (command, manager) in [("dnf", "dnf"), ("apt-get", "apt"), ("pacman", "pacman")] {
        if command_exists(command) {
            return manager;
        }
    }
    ""
}

fn read_software_sources() -> SoftwareSourcesData {
    let manager = package_manager();
    let mut sources = Vec::new();
    if !manager.is_empty() {
        let name = match manager {
            "dnf" => "Fedora repositories",
            "apt" => "Debian repositories",
            "pacman" => "Arch repositories",
            _ => "Distribution repositories",
        };
        sources.push(SoftwareSourceData {
            id: format!("distro:{manager}"),
            name: name.to_owned(),
            kind: "distro".to_owned(),
            enabled: true,
            writable: false,
            note: "Distribution repositories are managed by system policy.".to_owned(),
        });
    }

    let mut flatpak_available = false;
    if command_exists("flatpak") {
        if let Some(output) = run(
            "flatpak",
            &["remotes", "--show-disabled", "--columns=name,url,options"],
        ) {
            flatpak_available = true;
            for line in output.lines() {
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() < 3 {
                    continue;
                }
                let name = fields[0].trim();
                let options = fields[2].trim();
                let scope = if options.split(',').any(|option| option == "user") {
                    "user"
                } else {
                    "system"
                };
                let disabled = options.split(',').any(|option| option == "disabled");
                sources.push(SoftwareSourceData {
                    id: format!("flatpak:{name}:{scope}"),
                    name: format!("{name} ({scope})"),
                    kind: "flatpak".to_owned(),
                    enabled: !disabled,
                    writable: true,
                    note: format!("{scope} Flatpak remote"),
                });
            }
        }
    }

    SoftwareSourcesData {
        available: !manager.is_empty() || flatpak_available,
        manager: manager.to_owned(),
        sources,
    }
}

fn read_encryption() -> EncryptionData {
    let Some(output) = run("lsblk", &["-J", "-o", "NAME,TYPE,FSTYPE,MOUNTPOINTS"]) else {
        return EncryptionData {
            available: false,
            label: "Unknown".to_owned(),
            note: "lsblk did not answer, so disk-encryption status is unavailable.".to_owned(),
            ..EncryptionData::default()
        };
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) else {
        return EncryptionData {
            available: false,
            label: "Unknown".to_owned(),
            note: "The lsblk answer could not be decoded.".to_owned(),
            ..EncryptionData::default()
        };
    };
    let mut devices = Vec::new();
    if let Some(blockdevices) = value.get("blockdevices").and_then(|value| value.as_array()) {
        for device in blockdevices {
            walk_lsblk(device, &mut devices);
        }
    }
    let encrypted = !devices.is_empty();
    EncryptionData {
        available: true,
        encrypted,
        label: if encrypted {
            "Encrypted (LUKS)".to_owned()
        } else {
            "Not encrypted".to_owned()
        },
        note: if encrypted {
            "A LUKS/crypt device is present. Unlocking and recovery stay with the distribution's cryptsetup tools.".to_owned()
        } else {
            "No LUKS/crypt device was reported. This pane is read-only; disk encryption is not configured here.".to_owned()
        },
        devices,
    }
}

fn walk_lsblk(node: &serde_json::Value, devices: &mut Vec<EncryptionDeviceData>) {
    let name = node
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let kind = node
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let fstype = node
        .get("fstype")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let encrypted = kind == "crypt" || fstype == "crypto_LUKS";
    if encrypted {
        let mountpoints = node
            .get("mountpoints")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        devices.push(EncryptionDeviceData {
            name: name.to_owned(),
            kind: kind.to_owned(),
            fstype: fstype.to_owned(),
            mountpoints,
        });
    }
    if let Some(children) = node.get("children").and_then(|value| value.as_array()) {
        for child in children {
            walk_lsblk(child, devices);
        }
    }
}

fn read_hardening() -> HardeningData {
    let selinux = run("getenforce", &[]);
    let apparmor = run("aa-status", &[]);
    let secure_boot = run("mokutil", &["--sb-state"]);
    let selinux_mode = selinux.as_deref().unwrap_or_default().to_lowercase();
    let apparmor_enforcing = apparmor
        .as_deref()
        .map(|value| {
            value.contains("apparmor module is loaded")
                && value.contains("profiles are in enforce mode")
        })
        .unwrap_or(false);
    let mode = if selinux_mode == "enforcing" || apparmor_enforcing {
        "enforcing"
    } else if selinux_mode == "permissive" {
        "permissive"
    } else if selinux_mode == "disabled" {
        "disabled"
    } else {
        "standard"
    };
    let label = match mode {
        "enforcing" => "Enforcing",
        "permissive" => "Permissive",
        "disabled" => "Disabled",
        _ => "Standard",
    };
    let mut details = Vec::new();
    if !selinux_mode.is_empty() {
        details.push(format!("SELinux is {selinux_mode}"));
    }
    if let Some(value) = &apparmor {
        let first = value.lines().next().unwrap_or_default().trim();
        if !first.is_empty() {
            details.push(first.to_owned());
        }
    }
    if let Some(value) = &secure_boot {
        let lower = value.to_lowercase();
        if lower.contains("enabled") {
            details.push("Secure Boot is enabled".to_owned());
        } else if lower.contains("disabled") {
            details.push("Secure Boot is disabled".to_owned());
        }
    }
    let available = selinux.is_some() || apparmor.is_some() || secure_boot.is_some();
    HardeningData {
        available,
        mode: mode.to_owned(),
        label: label.to_owned(),
        detail: if details.is_empty() {
            "No SELinux, AppArmor, or Secure Boot probe answered.".to_owned()
        } else {
            details.join("; ") + "."
        },
    }
}

fn portal_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/usr/share/xdg-desktop-portal/portals")];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("xdg-desktop-portal/portals"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/xdg-desktop-portal/portals"));
    }
    dirs
}

fn read_extensions() -> Vec<ExtensionData> {
    let mut extensions = Vec::new();
    for dir in portal_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("portal") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned();
            let mut dbus_name = String::new();
            let mut interfaces = Vec::new();
            let mut use_in = String::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(value) = line.strip_prefix("Name=") {
                    if !value.trim().is_empty() {
                        name = value.trim().to_owned();
                    }
                } else if let Some(value) = line.strip_prefix("DBusName=") {
                    dbus_name = value.trim().to_owned();
                } else if let Some(value) = line.strip_prefix("Interfaces=") {
                    interfaces = value
                        .split(';')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect();
                } else if let Some(value) = line.strip_prefix("UseIn=") {
                    use_in = value.trim().to_owned();
                }
            }
            extensions.push(ExtensionData {
                id: path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_owned(),
                name,
                dbus_name,
                interfaces,
                use_in,
                path: path.display().to_string(),
            });
        }
    }
    extensions.sort_by(|left, right| left.id.cmp(&right.id));
    extensions
}

fn read_advanced(
    software_sources: &SoftwareSourcesData,
    encryption: &EncryptionData,
    hardening: &HardeningData,
) -> Vec<AdvancedRowData> {
    let mut rows = Vec::new();
    let polkit = command_exists("pkcheck") || command_exists("pkaction");
    rows.push(AdvancedRowData {
        id: "polkit".to_owned(),
        label: "Authorization".to_owned(),
        value: if polkit { "polkit" } else { "Unavailable" }.to_owned(),
        detail: "Privileged actions use the session polkit agent.".to_owned(),
        state: if polkit { "ok" } else { "absent" }.to_owned(),
    });
    let firewall = if command_exists("firewall-cmd") {
        Some("firewalld")
    } else if command_exists("ufw") {
        Some("ufw")
    } else if command_exists("nft") {
        Some("nftables")
    } else {
        None
    };
    rows.push(AdvancedRowData {
        id: "firewall".to_owned(),
        label: "Firewall".to_owned(),
        value: firewall.unwrap_or("Not detected").to_owned(),
        detail: "The Network pane owns the firewall controls.".to_owned(),
        state: if firewall.is_some() { "ok" } else { "absent" }.to_owned(),
    });
    rows.push(AdvancedRowData {
        id: "package-manager".to_owned(),
        label: "Package manager".to_owned(),
        value: if software_sources.manager.is_empty() {
            "Not detected".to_owned()
        } else {
            software_sources.manager.clone()
        },
        detail: "Distribution package management stays with the host.".to_owned(),
        state: if software_sources.manager.is_empty() {
            "absent".to_owned()
        } else {
            "ok".to_owned()
        },
    });
    rows.push(AdvancedRowData {
        id: "disk-encryption".to_owned(),
        label: "Disk encryption".to_owned(),
        value: encryption.label.clone(),
        detail: "Read-only; cryptsetup owns unlocking and recovery.".to_owned(),
        state: if !encryption.available {
            "absent".to_owned()
        } else if encryption.encrypted {
            "ok".to_owned()
        } else {
            "off".to_owned()
        },
    });
    rows.push(AdvancedRowData {
        id: "hardening".to_owned(),
        label: "Hardening".to_owned(),
        value: hardening.label.clone(),
        detail: hardening.detail.clone(),
        state: if !hardening.available {
            "absent".to_owned()
        } else if hardening.mode == "enforcing" {
            "ok".to_owned()
        } else {
            "off".to_owned()
        },
    });
    let portal = session_name_has_owner("org.freedesktop.portal.Desktop");
    rows.push(AdvancedRowData {
        id: "portal".to_owned(),
        label: "Desktop portal".to_owned(),
        value: if portal { "Running" } else { "Not running" }.to_owned(),
        detail: "xdg-desktop-portal owns the permission store and file pickers.".to_owned(),
        state: if portal { "ok" } else { "absent" }.to_owned(),
    });
    let goa = session_name_has_owner("org.gnome.OnlineAccounts");
    rows.push(AdvancedRowData {
        id: "online-accounts".to_owned(),
        label: "Online accounts".to_owned(),
        value: if goa { "Available" } else { "Not installed" }.to_owned(),
        detail: "Optional Online Accounts service.".to_owned(),
        state: if goa { "ok" } else { "absent" }.to_owned(),
    });
    rows
}

fn session_name_has_owner(service: &str) -> bool {
    let Ok(connection) = zbus::blocking::Connection::session() else {
        return false;
    };
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(service,),
    );
    reply
        .ok()
        .and_then(|reply| reply.body().deserialize::<bool>().ok())
        .unwrap_or(false)
}

/// A deterministic security source for CI and pane tests.
#[derive(Debug, Clone, PartialEq)]
pub struct MockSecurity {
    present: bool,
    data: Option<SecurityData>,
    write_failure: Option<AdapterError>,
    deny_writes: Option<String>,
    writes: u32,
    last_source: Option<(String, bool)>,
}

impl MockSecurity {
    /// No security owner is reachable.
    pub fn absent() -> Self {
        MockSecurity {
            present: false,
            data: None,
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_source: None,
        }
    }

    /// A security host answering with the deterministic fixture.
    pub fn present() -> Self {
        MockSecurity {
            present: true,
            data: Some(security_fixture()),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_source: None,
        }
    }

    /// A security host answering with `data`.
    pub fn with_data(data: SecurityData) -> Self {
        MockSecurity {
            present: true,
            data: Some(data),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_source: None,
        }
    }

    /// Make every write fail with `message`.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// Make every write a policy denial with `note`.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.deny_writes = Some(note.into());
        self
    }

    /// The security host publishes fresh state.
    pub fn push(&mut self, data: SecurityData) {
        self.present = true;
        self.data = Some(data);
    }

    /// The security host goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The security host comes back.
    pub fn restart(&mut self) {
        self.present = true;
    }

    /// The last source write, for tests.
    pub fn last_source(&self) -> Option<&(String, bool)> {
        self.last_source.as_ref()
    }

    /// The write count.
    pub fn writes(&self) -> u32 {
        self.writes
    }
}

impl SecuritySource for MockSecurity {
    fn read_security(&mut self) -> Result<Option<SecurityData>, AdapterError> {
        if !self.present {
            return Ok(None);
        }
        Ok(self.data.clone())
    }

    fn set_source_enabled(&mut self, id: &str, enabled: bool) -> SecurityOutcome {
        if !self.present {
            return SecurityOutcome::Absent;
        }
        if let Some(note) = &self.deny_writes {
            return SecurityOutcome::Denied(note.clone());
        }
        if let Some(error) = &self.write_failure {
            return SecurityOutcome::Failed(error.clone());
        }
        let Some(data) = self.data.as_mut() else {
            return SecurityOutcome::Absent;
        };
        let Some(source) = data
            .software_sources
            .sources
            .iter_mut()
            .find(|source| source.id == id)
        else {
            return SecurityOutcome::Absent;
        };
        if !source.writable {
            return SecurityOutcome::Denied("This source is managed by system policy.".to_owned());
        }
        source.enabled = enabled;
        self.writes += 1;
        self.last_source = Some((id.to_owned(), enabled));
        SecurityOutcome::Applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_is_deterministic() {
        assert_eq!(security_fixture(), security_fixture());
    }

    #[test]
    fn an_absent_source_is_unavailable() {
        let mut adapter = SecurityAdapter::new(MockSecurity::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_present_source_reports_every_sub_read() {
        let mut adapter = SecurityAdapter::new(MockSecurity::present());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert!(snapshot.software_sources().available);
        assert_eq!(snapshot.software_sources().manager, "dnf");
        assert_eq!(snapshot.software_sources().sources.len(), 2);
        assert_eq!(snapshot.enabled_source_count(), 2);
        assert!(snapshot.encryption().encrypted);
        assert_eq!(snapshot.hardening().mode, "enforcing");
        assert_eq!(snapshot.extensions().len(), 2);
        assert_eq!(snapshot.advanced().len(), 3);
    }

    #[test]
    fn a_source_write_mutates_the_mock_and_counts() {
        let mut adapter = SecurityAdapter::new(MockSecurity::present());
        adapter.refresh();
        let outcome = adapter.set_source_enabled("flatpak:flathub:system", false);
        assert!(outcome.is_applied());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.enabled_source_count(), 1);
        assert_eq!(adapter.source().writes(), 1);
        assert_eq!(
            adapter.source().last_source(),
            Some(&("flatpak:flathub:system".to_owned(), false))
        );
    }

    #[test]
    fn a_read_only_source_is_a_denial() {
        let mut adapter = SecurityAdapter::new(MockSecurity::present());
        adapter.refresh();
        let outcome = adapter.set_source_enabled("distro:dnf", false);
        assert!(matches!(outcome, SecurityOutcome::Denied(_)));
        assert_eq!(adapter.source().writes(), 0);
    }

    #[test]
    fn a_denied_write_keeps_the_state() {
        let mut adapter =
            SecurityAdapter::new(MockSecurity::present().deny_writes("polkit denied"));
        adapter.refresh();
        let outcome = adapter.set_source_enabled("flatpak:flathub:system", false);
        assert_eq!(outcome, SecurityOutcome::Denied("polkit denied".to_owned()));
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().enabled_source_count(), 2);
    }
}
