// SPDX-License-Identifier: MIT
//! The Fedora/Ubuntu/Debian/Arch stub backends.
//!
//! T221 lands the contract, the matrix, and these stubs. Every trait method
//! returns the typed [`DistroError::NotSupported`] answer and every status is
//! `Unsupported`, but the stub carries the exact command/API the eventual
//! backend will use (T254–T259), so the contract tests, the probe, and the
//! Settings diagnostics can already name it. No stub shells out.

use std::path::Path;

use crate::capability::{
    BackendId, Capability, CapabilityAdapter, CapabilityStatus, DistroError, Unsupported,
};
use crate::distro::{Distro, DistroFamily};
use crate::matrix::{self, BackendSpec};
use crate::traits::{
    AppDiscoveryProvider, AppSource, AutologinProvider, AutologinStatus, DefaultApp,
    DefaultAppsProvider, FirewallProvider, FirewallStatus, GroupRecord, InstallSource,
    LocaleProvider, LocaleStatus, NewUser, Port, Secret, TimeProvider, TimeStatus, Update,
    UpdateId, UpdateProgress, UpdateProvider, UserGroupProvider,
};

/// The reason every stub reports today.
pub const STUB_REASON: &str = "T221 stub: the concrete backend lands in T254-T259";

/// The shared stub core: one distribution and its matrix rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubBackend {
    distro: Distro,
}

impl StubBackend {
    /// A stub for `distro`.
    pub fn new(distro: Distro) -> Self {
        StubBackend { distro }
    }

    /// The stub selected for the current host.
    pub fn detect() -> Self {
        StubBackend::new(Distro::detect())
    }

    /// The stub selected from `root/etc/os-release`.
    pub fn detect_from_root(root: &Path) -> Self {
        StubBackend::new(Distro::detect_from_root(root))
    }

    /// A stub for a family with a synthetic identity.
    pub fn for_family(family: DistroFamily) -> Self {
        StubBackend::new(Distro::for_family(family))
    }

    /// The Fedora stub.
    pub fn fedora() -> Self {
        StubBackend::for_family(DistroFamily::Fedora)
    }

    /// The Ubuntu stub.
    pub fn ubuntu() -> Self {
        StubBackend::for_family(DistroFamily::Ubuntu)
    }

    /// The Debian stub.
    pub fn debian() -> Self {
        StubBackend::for_family(DistroFamily::Debian)
    }

    /// The Arch stub.
    pub fn arch() -> Self {
        StubBackend::for_family(DistroFamily::Arch)
    }

    /// The detected distribution.
    pub fn distro(&self) -> &Distro {
        &self.distro
    }

    /// The mapped family.
    pub fn family(&self) -> DistroFamily {
        self.distro.family
    }

    /// The selected backend id.
    pub fn backend(&self) -> BackendId {
        self.distro.family.into()
    }

    /// The matrix row for `capability` on this family.
    pub fn spec(&self, capability: Capability) -> Option<&'static BackendSpec> {
        matrix::spec(capability, self.family())
    }

    /// The unsupported status for `capability`, carrying the matrix command.
    pub fn unsupported_status(&self, capability: Capability) -> CapabilityStatus {
        let (command, reason) = match self.spec(capability) {
            Some(spec) => (spec.api, STUB_REASON.to_owned()),
            None => (
                "no backend",
                format!("unsupported distribution: {}", self.distro.display_name()),
            ),
        };
        CapabilityStatus::Unsupported(Unsupported {
            capability,
            backend: self.backend(),
            command,
            reason,
        })
    }

    /// The `NotSupported` error for `capability`.
    pub fn not_supported<T>(&self, capability: Capability) -> Result<T, DistroError> {
        let command = self
            .spec(capability)
            .map(|spec| spec.api)
            .unwrap_or("no backend");
        Err(DistroError::NotSupported {
            capability,
            backend: self.backend(),
            command,
        })
    }

    /// The Updates stub.
    pub fn updates(self) -> StubUpdates {
        StubUpdates(self)
    }

    /// The Firewall stub.
    pub fn firewall(self) -> StubFirewall {
        StubFirewall(self)
    }

    /// The Users & Groups stub.
    pub fn users(self) -> StubUsers {
        StubUsers(self)
    }

    /// The Time stub.
    pub fn time(self) -> StubTime {
        StubTime(self)
    }

    /// The Locale stub.
    pub fn locale(self) -> StubLocale {
        StubLocale(self)
    }

    /// The Autologin stub.
    pub fn autologin(self) -> StubAutologin {
        StubAutologin(self)
    }

    /// The Default Apps stub.
    pub fn default_apps(self) -> StubDefaultApps {
        StubDefaultApps(self)
    }

    /// The App Discovery stub.
    pub fn app_discovery(self) -> StubAppDiscovery {
        StubAppDiscovery(self)
    }
}

macro_rules! stub_adapter {
    ($name:ident, $capability:expr) => {
        impl CapabilityAdapter for $name {
            fn capability(&self) -> Capability {
                $capability
            }

            fn backend(&self) -> BackendId {
                self.0.backend()
            }

            fn status(&self) -> CapabilityStatus {
                self.0.unsupported_status($capability)
            }
        }
    };
}

/// The Updates stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubUpdates(StubBackend);
stub_adapter!(StubUpdates, Capability::Updates);

impl UpdateProvider for StubUpdates {
    fn distribution_info(&self) -> Result<crate::traits::DistributionInfo, DistroError> {
        self.0.not_supported(Capability::Updates)
    }

    fn check_updates(&self) -> Result<Vec<Update>, DistroError> {
        self.0.not_supported(Capability::Updates)
    }

    fn install_updates(&self, _ids: &[UpdateId]) -> Result<UpdateProgress, DistroError> {
        self.0.not_supported(Capability::Updates)
    }

    fn reboot_required(&self) -> Result<bool, DistroError> {
        self.0.not_supported(Capability::Updates)
    }
}

/// The Firewall stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubFirewall(StubBackend);
stub_adapter!(StubFirewall, Capability::Firewall);

impl FirewallProvider for StubFirewall {
    fn firewall_status(&self) -> Result<FirewallStatus, DistroError> {
        self.0.not_supported(Capability::Firewall)
    }

    fn set_enabled(&self, _enabled: bool) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Firewall)
    }

    fn open_port(&self, _port: Port) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Firewall)
    }

    fn close_port(&self, _port: Port) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Firewall)
    }
}

/// The Users & Groups stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubUsers(StubBackend);
stub_adapter!(StubUsers, Capability::UsersGroups);

impl UserGroupProvider for StubUsers {
    fn admin_groups(&self) -> Result<Vec<String>, DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn list_groups(&self) -> Result<Vec<GroupRecord>, DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn create_user(&self, _user: &NewUser) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn delete_user(&self, _user_name: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn set_password(&self, _user_name: &str, _password: &Secret) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn create_group(&self, _name: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn delete_group(&self, _name: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }

    fn set_group_members(&self, _name: &str, _members: &[String]) -> Result<(), DistroError> {
        self.0.not_supported(Capability::UsersGroups)
    }
}

/// The Time stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubTime(StubBackend);
stub_adapter!(StubTime, Capability::Time);

impl TimeProvider for StubTime {
    fn time_status(&self) -> Result<TimeStatus, DistroError> {
        self.0.not_supported(Capability::Time)
    }

    fn set_timezone(&self, _timezone: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Time)
    }

    fn set_ntp(&self, _enabled: bool) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Time)
    }
}

/// The Locale stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubLocale(StubBackend);
stub_adapter!(StubLocale, Capability::Locale);

impl LocaleProvider for StubLocale {
    fn locale_status(&self) -> Result<LocaleStatus, DistroError> {
        self.0.not_supported(Capability::Locale)
    }

    fn available_locales(&self) -> Result<Vec<String>, DistroError> {
        self.0.not_supported(Capability::Locale)
    }

    fn set_locale(&self, _locale: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Locale)
    }
}

/// The Autologin stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubAutologin(StubBackend);
stub_adapter!(StubAutologin, Capability::Autologin);

impl AutologinProvider for StubAutologin {
    fn autologin_status(&self) -> Result<AutologinStatus, DistroError> {
        self.0.not_supported(Capability::Autologin)
    }

    fn set_autologin(&self, _user: Option<&str>) -> Result<(), DistroError> {
        self.0.not_supported(Capability::Autologin)
    }
}

/// The Default Apps stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubDefaultApps(StubBackend);
stub_adapter!(StubDefaultApps, Capability::DefaultApps);

impl DefaultAppsProvider for StubDefaultApps {
    fn roles(&self) -> Result<Vec<DefaultApp>, DistroError> {
        self.0.not_supported(Capability::DefaultApps)
    }

    fn default_app(&self, _role: &str) -> Result<Option<String>, DistroError> {
        self.0.not_supported(Capability::DefaultApps)
    }

    fn set_default_app(&self, _role: &str, _desktop_id: &str) -> Result<(), DistroError> {
        self.0.not_supported(Capability::DefaultApps)
    }

    fn mime_handler(&self, _mime: &str) -> Result<Option<String>, DistroError> {
        self.0.not_supported(Capability::DefaultApps)
    }
}

/// The App Discovery stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubAppDiscovery(StubBackend);
stub_adapter!(StubAppDiscovery, Capability::AppDiscovery);

impl AppDiscoveryProvider for StubAppDiscovery {
    fn discovery_paths(&self) -> Result<Vec<AppSource>, DistroError> {
        self.0.not_supported(Capability::AppDiscovery)
    }

    fn install_sources(&self) -> Result<Vec<InstallSource>, DistroError> {
        self.0.not_supported(Capability::AppDiscovery)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_family_constructors_select_the_right_backend() {
        assert_eq!(StubBackend::fedora().backend(), BackendId::FEDORA);
        assert_eq!(StubBackend::ubuntu().backend(), BackendId::UBUNTU);
        assert_eq!(StubBackend::debian().backend(), BackendId::DEBIAN);
        assert_eq!(StubBackend::arch().backend(), BackendId::ARCH);
    }

    #[test]
    fn detect_selects_the_current_host_family() {
        assert_eq!(
            StubBackend::detect().family(),
            crate::distro::Distro::detect().family
        );
    }

    #[test]
    fn every_stub_status_is_unsupported_and_names_the_matrix_command() {
        for family in DistroFamily::SUPPORTED {
            let backend = StubBackend::for_family(family);
            for capability in Capability::ALL {
                let status = backend.unsupported_status(capability);
                assert!(status.is_unsupported(), "{family:?} {capability:?}");
                assert_eq!(status.capability(), capability);
                assert_eq!(status.backend(), BackendId::from(family));
                let expected = matrix::spec(capability, family).unwrap().api;
                assert_eq!(status.command(), expected);
            }
        }
    }

    #[test]
    fn an_unknown_distro_is_unsupported_without_a_matrix_row() {
        let backend = StubBackend::for_family(DistroFamily::Unknown);
        let status = backend.unsupported_status(Capability::Updates);
        assert!(status.is_unsupported());
        assert_eq!(status.backend(), BackendId::NONE);
        assert!(status.reason().contains("unsupported distribution"));
    }
}
