// SPDX-License-Identifier: MIT
//! The per-distribution command/API matrix.
//!
//! Every supported family has one [`BackendSpec`] per capability. The spec is
//! the single place the eventual concrete backend (T254–T259) gets its tool
//! name, exact invocation/API, detection probes, package names, and notes.
//! The T221 stubs expose the same spec in their `Unsupported` status and in
//! the `NotSupported` error, so the UI and diagnostics can name what will be
//! used without shelling out.

use crate::capability::{BackendId, Capability};
use crate::distro::DistroFamily;

/// One capability on one distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSpec {
    /// The capability.
    pub capability: Capability,
    /// The distribution family.
    pub family: DistroFamily,
    /// The backend id the family selects.
    pub backend: BackendId,
    /// The short tool name the backend drives.
    pub command: &'static str,
    /// The exact invocation or API description.
    pub api: &'static str,
    /// Commands whose presence means the host can serve the capability.
    /// Empty means the capability is built in (app discovery).
    pub probe: &'static [&'static str],
    /// Package names a diagnostic or installer can suggest.
    pub packages: &'static [&'static str],
    /// A one-line implementation note for T254–T259.
    pub note: &'static str,
}

impl BackendSpec {
    /// Whether the host probe can detect this capability from a command.
    pub const fn is_builtin(&self) -> bool {
        self.probe.is_empty()
    }
}

const fn row(
    capability: Capability,
    family: DistroFamily,
    command: &'static str,
    api: &'static str,
    probe: &'static [&'static str],
    packages: &'static [&'static str],
    note: &'static str,
) -> BackendSpec {
    BackendSpec {
        capability,
        family,
        backend: family_backend(family),
        command,
        api,
        probe,
        packages,
        note,
    }
}

const fn family_backend(family: DistroFamily) -> BackendId {
    match family {
        DistroFamily::Fedora => BackendId::FEDORA,
        DistroFamily::Ubuntu => BackendId::UBUNTU,
        DistroFamily::Debian => BackendId::DEBIAN,
        DistroFamily::Arch => BackendId::ARCH,
        DistroFamily::Unknown => BackendId::NONE,
    }
}

const FEDORA: DistroFamily = DistroFamily::Fedora;
const UBUNTU: DistroFamily = DistroFamily::Ubuntu;
const DEBIAN: DistroFamily = DistroFamily::Debian;
const ARCH: DistroFamily = DistroFamily::Arch;

const DNF_PROBE: &[&str] = &["dnf5", "dnf", "pkcon"];
const APT_PROBE: &[&str] = &["apt-get", "pkcon"];
const PACMAN_PROBE: &[&str] = &["pacman", "pkcon"];
const USER_PROBE: &[&str] = &["useradd", "usermod", "groupadd", "gpasswd"];
const USER_PROBE_DEB: &[&str] = &["useradd", "adduser", "gpasswd"];
const TIME_PROBE: &[&str] = &["timedatectl"];
const LOCALE_PROBE: &[&str] = &["localectl"];
const XDG_PROBE: &[&str] = &["xdg-mime", "xdg-settings"];

/// Every matrix row: 8 capabilities × 4 supported families.
pub const MATRIX: &[BackendSpec] = &[
    // ---------------------------------------------------------------- updates
    row(
        Capability::Updates,
        FEDORA,
        "dnf5",
        "dnf5 check-update --json; dnf5 upgrade --refresh",
        DNF_PROBE,
        &["dnf5", "PackageKit"],
        "Native dnf5 first; PackageKit is the fallback. Offline updates use dnf5 offline.",
    ),
    row(
        Capability::Updates,
        UBUNTU,
        "apt-get",
        "apt-get update; apt-get --simulate upgrade; apt-get install",
        APT_PROBE,
        &["apt", "PackageKit"],
        "Native apt first; PackageKit is the fallback. unattended-upgrades owns the automatic policy.",
    ),
    row(
        Capability::Updates,
        DEBIAN,
        "apt-get",
        "apt-get update; apt-get --simulate upgrade; apt-get install",
        APT_PROBE,
        &["apt", "PackageKit"],
        "Native apt first; PackageKit is the fallback. unattended-upgrades owns the automatic policy.",
    ),
    row(
        Capability::Updates,
        ARCH,
        "pacman",
        "pacman -Sy; pacman -Qu; pacman -S",
        PACMAN_PROBE,
        &["pacman", "PackageKit"],
        "Native pacman first; PackageKit is the fallback. Rolling release has no offline-update equivalent.",
    ),
    // --------------------------------------------------------------- firewall
    row(
        Capability::Firewall,
        FEDORA,
        "firewall-cmd",
        "org.fedoraproject.FirewallD1 over D-Bus; firewall-cmd --state",
        &["firewall-cmd"],
        &["firewalld"],
        "firewalld D-Bus is the primary path; the CLI is the diagnostic fallback.",
    ),
    row(
        Capability::Firewall,
        UBUNTU,
        "ufw",
        "ufw status; ufw enable; ufw allow <port>/<proto>",
        &["ufw"],
        &["ufw"],
        "ufw CLI is the supported path; there is no stable ufw D-Bus API.",
    ),
    row(
        Capability::Firewall,
        DEBIAN,
        "nft",
        "nft list ruleset; nft add rule ... (ufw when installed)",
        &["nft", "ufw"],
        &["nftables", "ufw"],
        "nftables is the base; ufw is the friendly manager when present.",
    ),
    row(
        Capability::Firewall,
        ARCH,
        "nft",
        "nft list ruleset; nft add rule ... (iptables fallback)",
        &["nft", "iptables"],
        &["nftables", "iptables"],
        "No default manager; nftables/iptables only, and no fake toggle when neither is present.",
    ),
    // ----------------------------------------------------------- users/groups
    row(
        Capability::UsersGroups,
        FEDORA,
        "useradd",
        "useradd/usermod/userdel; groupadd/groupmod/groupdel; passwd/chpasswd; gpasswd; admin group wheel",
        USER_PROBE,
        &["shadow-utils", "passwd"],
        "AccountsService owns the user read; this provider owns the group and password writes. Admin group: wheel.",
    ),
    row(
        Capability::UsersGroups,
        UBUNTU,
        "useradd",
        "useradd/usermod/userdel; groupadd/groupmod/groupdel; passwd/chpasswd; gpasswd; admin groups sudo, admin",
        USER_PROBE_DEB,
        &["passwd", "adduser"],
        "AccountsService owns the user read; this provider owns the group and password writes. Detect sudo and admin.",
    ),
    row(
        Capability::UsersGroups,
        DEBIAN,
        "useradd",
        "useradd/usermod/userdel; groupadd/groupmod/groupdel; passwd/chpasswd; gpasswd; admin groups sudo, admin",
        USER_PROBE_DEB,
        &["passwd", "adduser"],
        "AccountsService owns the user read; this provider owns the group and password writes. Detect sudo and admin.",
    ),
    row(
        Capability::UsersGroups,
        ARCH,
        "useradd",
        "useradd/usermod/userdel; groupadd/groupmod/groupdel; passwd/chpasswd; gpasswd; admin group wheel",
        USER_PROBE,
        &["shadow"],
        "AccountsService owns the user read; this provider owns the group and password writes. Admin group: wheel.",
    ),
    // ------------------------------------------------------------------- time
    row(
        Capability::Time,
        FEDORA,
        "timedatectl",
        "timedatectl set-timezone; timedatectl set-ntp; chronyc tracking",
        &["timedatectl", "chronyc"],
        &["systemd", "chrony"],
        "timedatectl first; chrony is the default NTP client.",
    ),
    row(
        Capability::Time,
        UBUNTU,
        "timedatectl",
        "timedatectl set-timezone; timedatectl set-ntp; timedatectl timesync-status",
        TIME_PROBE,
        &["systemd-timesyncd"],
        "timedatectl first; systemd-timesyncd is the default NTP client.",
    ),
    row(
        Capability::Time,
        DEBIAN,
        "timedatectl",
        "timedatectl set-timezone; timedatectl set-ntp; timedatectl timesync-status",
        TIME_PROBE,
        &["systemd-timesyncd"],
        "timedatectl first; systemd-timesyncd is the default NTP client.",
    ),
    row(
        Capability::Time,
        ARCH,
        "timedatectl",
        "timedatectl set-timezone; timedatectl set-ntp; timedatectl timesync-status",
        TIME_PROBE,
        &["systemd-timesyncd"],
        "timedatectl first; systemd-timesyncd is the default NTP client.",
    ),
    // ----------------------------------------------------------------- locale
    row(
        Capability::Locale,
        FEDORA,
        "localectl",
        "localectl set-locale LANG=...; localectl list-locales",
        LOCALE_PROBE,
        &["systemd"],
        "localectl is the primary path.",
    ),
    row(
        Capability::Locale,
        UBUNTU,
        "localectl",
        "localectl set-locale; locale-gen; update-locale",
        &["localectl", "locale-gen", "update-locale"],
        &["locales"],
        "localectl first; locale-gen/update-locale are the Debian fallback.",
    ),
    row(
        Capability::Locale,
        DEBIAN,
        "localectl",
        "localectl set-locale; locale-gen; update-locale",
        &["localectl", "locale-gen", "update-locale"],
        &["locales"],
        "localectl first; locale-gen/update-locale are the Debian fallback.",
    ),
    row(
        Capability::Locale,
        ARCH,
        "localectl",
        "localectl set-locale; locale-gen",
        &["localectl", "locale-gen"],
        &["glibc"],
        "localectl first; locale-gen is the Arch fallback.",
    ),
    // -------------------------------------------------------------- autologin
    row(
        Capability::Autologin,
        FEDORA,
        "gdm",
        "GDM /etc/gdm/custom.conf [daemon] AutomaticLoginEnable/AutomaticLogin; SDDM /etc/sddm.conf [Autologin]",
        &["gdm", "sddm"],
        &["gdm", "sddm"],
        "Detect the running display manager and write its own file; no shared config format.",
    ),
    row(
        Capability::Autologin,
        UBUNTU,
        "gdm3",
        "GDM /etc/gdm3/custom.conf; LightDM /etc/lightdm/lightdm.conf; SDDM /etc/sddm.conf",
        &["gdm3", "lightdm", "sddm"],
        &["gdm3", "lightdm", "sddm"],
        "Detect GDM, LightDM, or SDDM and write its own file.",
    ),
    row(
        Capability::Autologin,
        DEBIAN,
        "lightdm",
        "LightDM /etc/lightdm/lightdm.conf; GDM /etc/gdm3/custom.conf; SDDM /etc/sddm.conf",
        &["lightdm", "gdm3", "sddm"],
        &["lightdm", "gdm3", "sddm"],
        "Minimal installs often use LightDM; detect before writing.",
    ),
    row(
        Capability::Autologin,
        ARCH,
        "sddm",
        "SDDM /etc/sddm.conf [Autologin]; greetd /etc/greetd/config.toml initial_session",
        &["sddm", "greetd", "gdm"],
        &["sddm", "greetd", "gdm"],
        "Detect SDDM, greetd, or GDM and write its own file.",
    ),
    // ----------------------------------------------------------- default apps
    row(
        Capability::DefaultApps,
        FEDORA,
        "xdg-mime",
        "xdg-mime query default <mime>; xdg-mime default <desktop> <mime>; xdg-settings; org.freedesktop.portal.Settings",
        XDG_PROBE,
        &["xdg-utils"],
        "The Settings portal is preferred; xdg-mime/mimeapps.list are the fallback.",
    ),
    row(
        Capability::DefaultApps,
        UBUNTU,
        "xdg-mime",
        "xdg-mime query default <mime>; xdg-mime default <desktop> <mime>; xdg-settings; org.freedesktop.portal.Settings",
        XDG_PROBE,
        &["xdg-utils"],
        "The Settings portal is preferred; xdg-mime/mimeapps.list are the fallback.",
    ),
    row(
        Capability::DefaultApps,
        DEBIAN,
        "xdg-mime",
        "xdg-mime query default <mime>; xdg-mime default <desktop> <mime>; xdg-settings; org.freedesktop.portal.Settings",
        XDG_PROBE,
        &["xdg-utils"],
        "The Settings portal is preferred; xdg-mime/mimeapps.list are the fallback.",
    ),
    row(
        Capability::DefaultApps,
        ARCH,
        "xdg-mime",
        "xdg-mime query default <mime>; xdg-mime default <desktop> <mime>; xdg-settings; org.freedesktop.portal.Settings",
        XDG_PROBE,
        &["xdg-utils"],
        "The Settings portal is preferred; xdg-mime/mimeapps.list are the fallback.",
    ),
    // ---------------------------------------------------------- app discovery
    row(
        Capability::AppDiscovery,
        FEDORA,
        "app-index",
        "XDG_DATA_HOME/XDG_DATA_DIRS + Flatpak/Snap/AppImage/Nix paths; no distro command",
        &[],
        &["flatpak", "snapd"],
        "app-index owns discovery; this adapter only declares distro extra paths and install sources.",
    ),
    row(
        Capability::AppDiscovery,
        UBUNTU,
        "app-index",
        "XDG_DATA_HOME/XDG_DATA_DIRS + Flatpak/Snap/AppImage/Nix paths; no distro command",
        &[],
        &["flatpak", "snapd"],
        "app-index owns discovery; this adapter only declares distro extra paths and install sources.",
    ),
    row(
        Capability::AppDiscovery,
        DEBIAN,
        "app-index",
        "XDG_DATA_HOME/XDG_DATA_DIRS + Flatpak/Snap/AppImage/Nix paths; no distro command",
        &[],
        &["flatpak", "snapd"],
        "app-index owns discovery; this adapter only declares distro extra paths and install sources.",
    ),
    row(
        Capability::AppDiscovery,
        ARCH,
        "app-index",
        "XDG_DATA_HOME/XDG_DATA_DIRS + Flatpak/Snap/AppImage/Nix paths; no distro command",
        &[],
        &["flatpak", "snapd"],
        "app-index owns discovery; this adapter only declares distro extra paths and install sources.",
    ),
];

/// The matrix row for `capability` on `family`, if the family is supported.
pub fn spec(capability: Capability, family: DistroFamily) -> Option<&'static BackendSpec> {
    MATRIX
        .iter()
        .find(|row| row.capability == capability && row.family == family)
}

/// Every matrix row for a family, in capability order.
pub fn specs_for(family: DistroFamily) -> Vec<&'static BackendSpec> {
    Capability::ALL
        .into_iter()
        .filter_map(|capability| spec(capability, family))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_family_has_every_capability() {
        for family in DistroFamily::SUPPORTED {
            let rows = specs_for(family);
            assert_eq!(rows.len(), Capability::ALL.len(), "{family:?}");
            for capability in Capability::ALL {
                let row = spec(capability, family).expect("matrix row");
                assert_eq!(row.backend, BackendId::from(family));
                assert!(!row.command.is_empty());
                assert!(!row.api.is_empty());
                assert!(!row.note.is_empty());
                assert!(
                    !row.probe.is_empty() || capability == Capability::AppDiscovery,
                    "{family:?} {capability:?} has no probe"
                );
            }
        }
    }

    #[test]
    fn unknown_has_no_matrix_row() {
        for capability in Capability::ALL {
            assert!(spec(capability, DistroFamily::Unknown).is_none());
        }
    }

    #[test]
    fn matrix_rows_are_unique() {
        for family in DistroFamily::SUPPORTED {
            for capability in Capability::ALL {
                let count = MATRIX
                    .iter()
                    .filter(|row| row.family == family && row.capability == capability)
                    .count();
                assert_eq!(count, 1, "{family:?} {capability:?}");
            }
        }
    }

    #[test]
    fn the_commands_are_the_documented_distro_tools() {
        assert_eq!(
            spec(Capability::Updates, DistroFamily::Fedora)
                .unwrap()
                .command,
            "dnf5"
        );
        assert_eq!(
            spec(Capability::Updates, DistroFamily::Ubuntu)
                .unwrap()
                .command,
            "apt-get"
        );
        assert_eq!(
            spec(Capability::Firewall, DistroFamily::Fedora)
                .unwrap()
                .command,
            "firewall-cmd"
        );
        assert_eq!(
            spec(Capability::Firewall, DistroFamily::Ubuntu)
                .unwrap()
                .command,
            "ufw"
        );
        assert_eq!(
            spec(Capability::Firewall, DistroFamily::Arch)
                .unwrap()
                .command,
            "nft"
        );
        assert_eq!(
            spec(Capability::Locale, DistroFamily::Debian)
                .unwrap()
                .command,
            "localectl"
        );
        assert!(spec(Capability::AppDiscovery, DistroFamily::Arch)
            .unwrap()
            .is_builtin());
    }
}
