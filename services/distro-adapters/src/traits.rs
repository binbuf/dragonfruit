// SPDX-License-Identifier: MIT
//! The eight administration traits and their typed data.
//!
//! Every trait extends [`CapabilityAdapter`], so a pane can ask
//! `available()` / `status()` without knowing the distribution. The reads and
//! writes are typed; a backend never returns a distro command string to the
//! caller and the UI never shells out.

use std::fmt;

use crate::capability::{BackendId, CapabilityAdapter, DistroError};

// ===========================================================================
// Updates
// ===========================================================================

/// How serious an available update is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UpdateSeverity {
    /// A routine update.
    #[default]
    Normal,
    /// An important update the provider flags for attention.
    Important,
    /// A security update.
    Security,
}

impl UpdateSeverity {
    /// Every severity, most routine first.
    pub const ALL: [UpdateSeverity; 3] = [
        UpdateSeverity::Normal,
        UpdateSeverity::Important,
        UpdateSeverity::Security,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            UpdateSeverity::Normal => "normal",
            UpdateSeverity::Important => "important",
            UpdateSeverity::Security => "security",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            UpdateSeverity::Normal => "Update",
            UpdateSeverity::Important => "Important",
            UpdateSeverity::Security => "Security Update",
        }
    }

    /// Parse the stable id back to a severity; an unknown id is `Normal`.
    pub fn from_id(id: &str) -> UpdateSeverity {
        UpdateSeverity::ALL
            .into_iter()
            .find(|severity| severity.id() == id)
            .unwrap_or(UpdateSeverity::Normal)
    }
}

/// The distribution provider's update phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum UpdatePhase {
    /// No check has run.
    #[default]
    Idle,
    /// A check is in progress.
    Checking,
    /// The last check found no updates.
    UpToDate,
    /// The last check found updates.
    Available,
    /// An install is in progress.
    Installing,
    /// Updates were installed and the host must restart.
    RebootRequired,
    /// The last check or install failed.
    Failed,
}

impl UpdatePhase {
    /// Every phase.
    pub const ALL: [UpdatePhase; 7] = [
        UpdatePhase::Idle,
        UpdatePhase::Checking,
        UpdatePhase::UpToDate,
        UpdatePhase::Available,
        UpdatePhase::Installing,
        UpdatePhase::RebootRequired,
        UpdatePhase::Failed,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            UpdatePhase::Idle => "idle",
            UpdatePhase::Checking => "checking",
            UpdatePhase::UpToDate => "up-to-date",
            UpdatePhase::Available => "available",
            UpdatePhase::Installing => "installing",
            UpdatePhase::RebootRequired => "reboot-required",
            UpdatePhase::Failed => "failed",
        }
    }

    /// Parse the stable id back to a phase; an unknown id is `Idle`.
    pub fn from_id(id: &str) -> UpdatePhase {
        UpdatePhase::ALL
            .into_iter()
            .find(|phase| phase.id() == id)
            .unwrap_or(UpdatePhase::Idle)
    }

    /// Whether an operation is in flight.
    pub const fn is_busy(self) -> bool {
        matches!(self, UpdatePhase::Checking | UpdatePhase::Installing)
    }
}

/// One update the provider offers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Update {
    /// The provider's stable identifier (a package name or nevra).
    pub id: String,
    /// The display name.
    pub name: String,
    /// A one-line summary.
    pub summary: String,
    /// The installed version.
    pub current_version: String,
    /// The version the update would install.
    pub available_version: String,
    /// The severity badge.
    pub severity: UpdateSeverity,
}

/// The provider's stable update id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct UpdateId(String);

impl UpdateId {
    /// A new update id.
    pub fn new(id: impl Into<String>) -> Self {
        UpdateId(id.into())
    }

    /// The id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UpdateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for UpdateId {
    fn from(value: &str) -> Self {
        UpdateId::new(value)
    }
}

impl From<String> for UpdateId {
    fn from(value: String) -> Self {
        UpdateId::new(value)
    }
}

/// The distribution identity the update provider reports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DistributionInfo {
    /// The distribution id (`fedora`, `ubuntu`, `debian`, `arch`).
    pub id: String,
    /// The display name.
    pub name: String,
    /// The version.
    pub version: String,
    /// The backend that answered.
    pub backend: BackendId,
}

/// The progress of one install request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateProgress {
    /// The phase after the request.
    pub phase: UpdatePhase,
    /// How many updates completed.
    pub completed: u32,
    /// How many updates were requested.
    pub total: u32,
    /// A status or error message.
    pub message: Option<String>,
}

/// Software updates and package management.
pub trait UpdateProvider: CapabilityAdapter {
    /// The distribution identity.
    fn distribution_info(&self) -> Result<DistributionInfo, DistroError>;

    /// Check for updates. One explicit read.
    fn check_updates(&self) -> Result<Vec<Update>, DistroError>;

    /// Install the updates with the given ids. One explicit write.
    fn install_updates(&self, ids: &[UpdateId]) -> Result<UpdateProgress, DistroError>;

    /// Whether the host must restart to finish an update.
    fn reboot_required(&self) -> Result<bool, DistroError>;
}

// ===========================================================================
// Firewall
// ===========================================================================

/// The firewall manager a backend drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum FirewallBackend {
    /// firewalld over D-Bus.
    Firewalld,
    /// ufw over its CLI.
    Ufw,
    /// nftables directly.
    Nftables,
    /// iptables directly.
    Iptables,
    /// No supported firewall manager.
    #[default]
    None,
}

impl FirewallBackend {
    /// Every backend.
    pub const ALL: [FirewallBackend; 5] = [
        FirewallBackend::Firewalld,
        FirewallBackend::Ufw,
        FirewallBackend::Nftables,
        FirewallBackend::Iptables,
        FirewallBackend::None,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            FirewallBackend::Firewalld => "firewalld",
            FirewallBackend::Ufw => "ufw",
            FirewallBackend::Nftables => "nftables",
            FirewallBackend::Iptables => "iptables",
            FirewallBackend::None => "none",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            FirewallBackend::Firewalld => "firewalld",
            FirewallBackend::Ufw => "ufw",
            FirewallBackend::Nftables => "nftables",
            FirewallBackend::Iptables => "iptables",
            FirewallBackend::None => "No firewall detected",
        }
    }
}

/// A transport protocol for a firewall rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
}

impl Protocol {
    /// Every protocol.
    pub const ALL: [Protocol; 2] = [Protocol::Tcp, Protocol::Udp];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

/// One open firewall port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Port {
    /// The port number.
    pub port: u16,
    /// The transport protocol.
    pub protocol: Protocol,
}

impl Port {
    /// A new port rule.
    pub const fn new(port: u16, protocol: Protocol) -> Self {
        Port { port, protocol }
    }

    /// The stable `port/protocol` id.
    pub fn id(self) -> String {
        format!("{}/{}", self.port, self.protocol.id())
    }
}

/// The firewall state the pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FirewallStatus {
    /// The manager the backend drives.
    pub backend: FirewallBackend,
    /// Whether the firewall is enabled.
    pub enabled: bool,
    /// The default zone, when the manager has one.
    pub default_zone: Option<String>,
    /// The open port rules.
    pub open_ports: Vec<Port>,
}

/// Firewall state and the two explicit writes.
pub trait FirewallProvider: CapabilityAdapter {
    /// The live firewall state.
    fn firewall_status(&self) -> Result<FirewallStatus, DistroError>;

    /// Enable or disable the firewall. One explicit write.
    fn set_enabled(&self, enabled: bool) -> Result<(), DistroError>;

    /// Open a port. One explicit write.
    fn open_port(&self, port: Port) -> Result<(), DistroError>;

    /// Close a port. One explicit write.
    fn close_port(&self, port: Port) -> Result<(), DistroError>;
}

// ===========================================================================
// Users and groups
// ===========================================================================

/// A user to create through the distribution provider.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewUser {
    /// The login name.
    pub user_name: String,
    /// The display name.
    pub real_name: String,
    /// Whether the user joins an administrator group.
    pub administrator: bool,
    /// The login shell, when non-default.
    pub shell: Option<String>,
}

/// One group from the distribution provider.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupRecord {
    /// The group name.
    pub name: String,
    /// The numeric group id.
    pub gid: u32,
    /// The member login names.
    pub members: Vec<String>,
    /// A system group, not a user-created one.
    pub system: bool,
}

/// A password that is never logged or printed.
///
/// `Debug` is redacted on purpose; the only way to read the value is
/// [`Secret::expose`], which a backend calls immediately before handing it to
/// the host's PAM/passwd tool.
pub struct Secret(String);

impl Secret {
    /// Wrap a password.
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }

    /// The password value. Never log the result.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// Users, groups, and administrator membership.
///
/// AccountsService owns the user **read**; this trait is the distribution
/// **write** path and the group list, exactly as the design splits them.
pub trait UserGroupProvider: CapabilityAdapter {
    /// The administrator group names present on this host.
    fn admin_groups(&self) -> Result<Vec<String>, DistroError>;

    /// The group list.
    fn list_groups(&self) -> Result<Vec<GroupRecord>, DistroError>;

    /// Create a user. One explicit write.
    fn create_user(&self, user: &NewUser) -> Result<(), DistroError>;

    /// Delete a user. One explicit write.
    fn delete_user(&self, user_name: &str) -> Result<(), DistroError>;

    /// Set a user's password. One explicit write; the secret is never logged.
    fn set_password(&self, user_name: &str, password: &Secret) -> Result<(), DistroError>;

    /// Create a group. One explicit write.
    fn create_group(&self, name: &str) -> Result<(), DistroError>;

    /// Delete a group. One explicit write.
    fn delete_group(&self, name: &str) -> Result<(), DistroError>;

    /// Replace a group's membership. One explicit write.
    fn set_group_members(&self, name: &str, members: &[String]) -> Result<(), DistroError>;
}

// ===========================================================================
// Time
// ===========================================================================

/// The host's time and NTP state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimeStatus {
    /// The IANA time zone.
    pub timezone: String,
    /// Whether NTP is enabled.
    pub ntp_enabled: bool,
    /// Whether the clock is synchronized.
    pub synchronized: bool,
    /// The NTP source, when the host reports one.
    pub source: Option<String>,
}

/// Time zone and NTP.
pub trait TimeProvider: CapabilityAdapter {
    /// The live time state.
    fn time_status(&self) -> Result<TimeStatus, DistroError>;

    /// Set the time zone. One explicit write.
    fn set_timezone(&self, timezone: &str) -> Result<(), DistroError>;

    /// Enable or disable NTP. One explicit write.
    fn set_ntp(&self, enabled: bool) -> Result<(), DistroError>;
}

// ===========================================================================
// Locale
// ===========================================================================

/// The host's locale state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocaleStatus {
    /// The full locale (`en_US.UTF-8`).
    pub locale: String,
    /// The language half (`en`).
    pub language: String,
    /// The formatting locale, when distinct.
    pub formats_locale: Option<String>,
    /// The keyboard layout, when reported.
    pub keymap: Option<String>,
}

/// System locale and language.
pub trait LocaleProvider: CapabilityAdapter {
    /// The live locale state.
    fn locale_status(&self) -> Result<LocaleStatus, DistroError>;

    /// The locales the host can install.
    fn available_locales(&self) -> Result<Vec<String>, DistroError>;

    /// Set the system locale. One explicit write.
    fn set_locale(&self, locale: &str) -> Result<(), DistroError>;
}

// ===========================================================================
// Autologin
// ===========================================================================

/// The display manager's automatic-login state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AutologinStatus {
    /// Whether the detected display manager supports automatic login.
    pub supported: bool,
    /// The automatic-login user, when enabled.
    pub user: Option<String>,
    /// The session entry, when the display manager names one.
    pub session: Option<String>,
    /// The detected display manager (`gdm`, `sddm`, `lightdm`, `greetd`).
    pub display_manager: Option<String>,
}

/// Display-manager automatic login.
pub trait AutologinProvider: CapabilityAdapter {
    /// The live automatic-login state.
    fn autologin_status(&self) -> Result<AutologinStatus, DistroError>;

    /// Enable automatic login for `user`, or disable it with `None`. One
    /// explicit write to the detected display manager's own file.
    fn set_autologin(&self, user: Option<&str>) -> Result<(), DistroError>;
}

// ===========================================================================
// Default apps
// ===========================================================================

/// One default-application role.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DefaultApp {
    /// The role (`browser`, `mail`, `media`, `file-manager`).
    pub role: String,
    /// The desktop id, when a handler is set.
    pub desktop_id: Option<String>,
    /// The MIME types the role covers.
    pub mime_types: Vec<String>,
}

/// Default applications and MIME handlers.
pub trait DefaultAppsProvider: CapabilityAdapter {
    /// The roles the pane can edit.
    fn roles(&self) -> Result<Vec<DefaultApp>, DistroError>;

    /// The desktop id registered for `role`.
    fn default_app(&self, role: &str) -> Result<Option<String>, DistroError>;

    /// Set the default desktop id for `role`. One explicit write.
    fn set_default_app(&self, role: &str, desktop_id: &str) -> Result<(), DistroError>;

    /// The desktop id registered for a MIME type.
    fn mime_handler(&self, mime: &str) -> Result<Option<String>, DistroError>;
}

// ===========================================================================
// Application discovery
// ===========================================================================

/// Where a desktop file comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppSourceKind {
    /// A system-wide XDG application directory.
    #[default]
    System,
    /// A user XDG application directory.
    User,
    /// A Flatpak export directory.
    Flatpak,
    /// A Snap desktop directory.
    Snap,
    /// A user-managed AppImage/desktop file.
    AppImage,
    /// A Nix profile.
    Nix,
    /// A distribution-specific extra path.
    Distro,
}

impl AppSourceKind {
    /// Every source kind.
    pub const ALL: [AppSourceKind; 7] = [
        AppSourceKind::System,
        AppSourceKind::User,
        AppSourceKind::Flatpak,
        AppSourceKind::Snap,
        AppSourceKind::AppImage,
        AppSourceKind::Nix,
        AppSourceKind::Distro,
    ];

    /// The stable id used by the wire and diagnostics.
    pub const fn id(self) -> &'static str {
        match self {
            AppSourceKind::System => "system",
            AppSourceKind::User => "user",
            AppSourceKind::Flatpak => "flatpak",
            AppSourceKind::Snap => "snap",
            AppSourceKind::AppImage => "appimage",
            AppSourceKind::Nix => "nix",
            AppSourceKind::Distro => "distro",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            AppSourceKind::System => "System",
            AppSourceKind::User => "User",
            AppSourceKind::Flatpak => "Flatpak",
            AppSourceKind::Snap => "Snap",
            AppSourceKind::AppImage => "AppImage",
            AppSourceKind::Nix => "Nix",
            AppSourceKind::Distro => "Distribution",
        }
    }
}

/// One discovery path and its source kind.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppSource {
    /// The source kind.
    pub kind: AppSourceKind,
    /// The directory path.
    pub path: String,
}

/// One way applications can be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstallSourceKind {
    /// The distribution repository.
    #[default]
    DistroRepository,
    /// Flathub/Flatpak.
    Flatpak,
    /// Snapd.
    Snap,
    /// A user-managed AppImage.
    AppImage,
    /// A local package file.
    LocalPackage,
}

impl InstallSourceKind {
    /// Every install source.
    pub const ALL: [InstallSourceKind; 5] = [
        InstallSourceKind::DistroRepository,
        InstallSourceKind::Flatpak,
        InstallSourceKind::Snap,
        InstallSourceKind::AppImage,
        InstallSourceKind::LocalPackage,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            InstallSourceKind::DistroRepository => "distro",
            InstallSourceKind::Flatpak => "flatpak",
            InstallSourceKind::Snap => "snap",
            InstallSourceKind::AppImage => "appimage",
            InstallSourceKind::LocalPackage => "local-package",
        }
    }

    /// The human label.
    pub const fn label(self) -> &'static str {
        match self {
            InstallSourceKind::DistroRepository => "Distribution repository",
            InstallSourceKind::Flatpak => "Flathub",
            InstallSourceKind::Snap => "Snap Store",
            InstallSourceKind::AppImage => "AppImage",
            InstallSourceKind::LocalPackage => "Local package",
        }
    }
}

/// One install source and whether it is available on this host.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstallSource {
    /// The source kind.
    pub kind: InstallSourceKind,
    /// Whether the host can install from it.
    pub available: bool,
    /// The label the pane draws.
    pub label: String,
}

/// Application discovery paths and install sources.
///
/// `app-index` owns discovery; this adapter only declares the extra paths and
/// sources a distribution adds. It never scans desktop files itself.
pub trait AppDiscoveryProvider: CapabilityAdapter {
    /// The discovery directories, in precedence order.
    fn discovery_paths(&self) -> Result<Vec<AppSource>, DistroError>;

    /// The install sources the host exposes.
    fn install_sources(&self) -> Result<Vec<InstallSource>, DistroError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_ids_and_phases_round_trip() {
        for phase in UpdatePhase::ALL {
            assert_eq!(UpdatePhase::from_id(phase.id()), phase);
        }
        assert_eq!(UpdatePhase::from_id("bogus"), UpdatePhase::Idle);
        assert!(UpdatePhase::Installing.is_busy());
        assert!(!UpdatePhase::Available.is_busy());

        for severity in UpdateSeverity::ALL {
            assert_eq!(UpdateSeverity::from_id(severity.id()), severity);
        }
        assert_eq!(UpdateSeverity::from_id("bogus"), UpdateSeverity::Normal);
        assert_eq!(UpdateId::from("glibc").to_string(), "glibc");
    }

    #[test]
    fn firewall_vocabulary_is_stable() {
        assert_eq!(FirewallBackend::Firewalld.id(), "firewalld");
        assert_eq!(FirewallBackend::None.label(), "No firewall detected");
        assert_eq!(Protocol::Tcp.id(), "tcp");
        assert_eq!(Port::new(443, Protocol::Tcp).id(), "443/tcp");
    }

    #[test]
    fn a_secret_never_prints_its_value() {
        let secret = Secret::new("hunter2");
        let debug = format!("{secret:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert_eq!(secret.expose(), "hunter2");
    }

    #[test]
    fn app_source_ids_are_stable() {
        assert_eq!(AppSourceKind::Flatpak.id(), "flatpak");
        assert_eq!(InstallSourceKind::LocalPackage.id(), "local-package");
        assert_eq!(InstallSourceKind::Flatpak.label(), "Flathub");
    }
}
