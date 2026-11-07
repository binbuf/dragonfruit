// SPDX-License-Identifier: MIT
//! The in-memory mock backend used by the contract tests and CI.
//!
//! One [`MockBackend`] holds every capability's status and data behind a
//! shared lock, so a test can drive several capability adapters from the same
//! simulated host. `available()`, `absent()`, and `unsupported()` are the
//! three contract states; writes mutate the simulated host and are counted so
//! a test can prove one explicit call, never a loop.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::capability::{
    Absence, BackendId, Capability, CapabilityAdapter, CapabilityInfo, CapabilityStatus,
    DistroError, Unsupported,
};
use crate::traits::{
    AppDiscoveryProvider, AppSource, AppSourceKind, AutologinProvider, AutologinStatus, DefaultApp,
    DefaultAppsProvider, DistributionInfo, FirewallBackend, FirewallProvider, FirewallStatus,
    GroupRecord, InstallSource, InstallSourceKind, LocaleProvider, LocaleStatus, NewUser, Port,
    Protocol, Secret, TimeProvider, TimeStatus, Update, UpdateId, UpdatePhase, UpdateProgress,
    UpdateProvider, UpdateSeverity, UserGroupProvider,
};

#[derive(Debug, Clone)]
struct MockState {
    statuses: BTreeMap<Capability, CapabilityStatus>,
    updates: Vec<Update>,
    firewall: FirewallStatus,
    users: Vec<NewUser>,
    admin_groups: Vec<String>,
    groups: Vec<GroupRecord>,
    time: TimeStatus,
    locale: LocaleStatus,
    locales: Vec<String>,
    autologin: AutologinStatus,
    default_apps: BTreeMap<String, String>,
    mime_handlers: BTreeMap<String, String>,
    discovery: Vec<AppSource>,
    install_sources: Vec<InstallSource>,
    reboot_required: bool,
    writes: u32,
    write_failure: Option<String>,
}

impl MockState {
    fn available() -> Self {
        let mut statuses = BTreeMap::new();
        for capability in Capability::ALL {
            statuses.insert(
                capability,
                CapabilityStatus::Available(CapabilityInfo {
                    capability,
                    backend: BackendId::MOCK,
                    command: "mock",
                    api: "in-memory mock backend",
                }),
            );
        }
        let mut default_apps = BTreeMap::new();
        default_apps.insert(
            "browser".to_owned(),
            "org.mozilla.firefox.desktop".to_owned(),
        );
        default_apps.insert(
            "mail".to_owned(),
            "org.mozilla.thunderbird.desktop".to_owned(),
        );
        default_apps.insert(
            "file-manager".to_owned(),
            "org.dragonfruit.Files.desktop".to_owned(),
        );
        let mut mime_handlers = BTreeMap::new();
        mime_handlers.insert(
            "text/html".to_owned(),
            "org.mozilla.firefox.desktop".to_owned(),
        );
        mime_handlers.insert(
            "inode/directory".to_owned(),
            "org.dragonfruit.Files.desktop".to_owned(),
        );
        MockState {
            statuses,
            updates: vec![Update {
                id: "mock-kernel".to_owned(),
                name: "kernel".to_owned(),
                summary: "The Linux kernel".to_owned(),
                current_version: "6.11".to_owned(),
                available_version: "6.12".to_owned(),
                severity: UpdateSeverity::Security,
            }],
            firewall: FirewallStatus {
                backend: FirewallBackend::Firewalld,
                enabled: true,
                default_zone: Some("public".to_owned()),
                open_ports: vec![Port::new(443, Protocol::Tcp)],
            },
            users: Vec::new(),
            admin_groups: vec!["wheel".to_owned()],
            groups: vec![GroupRecord {
                name: "wheel".to_owned(),
                gid: 10,
                members: Vec::new(),
                system: true,
            }],
            time: TimeStatus {
                timezone: "UTC".to_owned(),
                ntp_enabled: true,
                synchronized: true,
                source: Some("mock.pool.ntp.org".to_owned()),
            },
            locale: LocaleStatus {
                locale: "en_US.UTF-8".to_owned(),
                language: "en".to_owned(),
                formats_locale: Some("en_US.UTF-8".to_owned()),
                keymap: Some("us".to_owned()),
            },
            locales: vec!["en_US.UTF-8".to_owned(), "de_DE.UTF-8".to_owned()],
            autologin: AutologinStatus {
                supported: true,
                user: None,
                session: Some("dragonfruit".to_owned()),
                display_manager: Some("gdm".to_owned()),
            },
            default_apps,
            mime_handlers,
            discovery: vec![
                AppSource {
                    kind: AppSourceKind::User,
                    path: "/home/dan/.local/share/applications".to_owned(),
                },
                AppSource {
                    kind: AppSourceKind::System,
                    path: "/usr/share/applications".to_owned(),
                },
                AppSource {
                    kind: AppSourceKind::Flatpak,
                    path: "/var/lib/flatpak/exports/share/applications".to_owned(),
                },
            ],
            install_sources: vec![
                InstallSource {
                    kind: InstallSourceKind::DistroRepository,
                    available: true,
                    label: InstallSourceKind::DistroRepository.label().to_owned(),
                },
                InstallSource {
                    kind: InstallSourceKind::Flatpak,
                    available: false,
                    label: InstallSourceKind::Flatpak.label().to_owned(),
                },
            ],
            reboot_required: false,
            writes: 0,
            write_failure: None,
        }
    }

    fn with_state(status: CapabilityStatus) -> Self {
        let mut state = MockState::available();
        state.statuses.insert(status.capability(), status);
        state
    }
}

/// The in-memory mock backend.
#[derive(Debug, Clone)]
pub struct MockBackend {
    state: Arc<Mutex<MockState>>,
}

impl Default for MockBackend {
    fn default() -> Self {
        MockBackend::available()
    }
}

impl MockBackend {
    /// Every capability is available with fixture data.
    pub fn available() -> Self {
        MockBackend {
            state: Arc::new(Mutex::new(MockState::available())),
        }
    }

    /// Every capability is absent (the host tool is not installed).
    pub fn absent() -> Self {
        let mut state = MockState::available();
        for capability in Capability::ALL {
            state.statuses.insert(
                capability,
                CapabilityStatus::Absent(Absence {
                    capability,
                    missing: vec!["mock"],
                    reason: "mock backend absent".to_owned(),
                }),
            );
        }
        MockBackend {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Every capability is unsupported (the stub/unknown-distro state).
    pub fn unsupported() -> Self {
        let mut state = MockState::available();
        for capability in Capability::ALL {
            state.statuses.insert(
                capability,
                CapabilityStatus::Unsupported(Unsupported {
                    capability,
                    backend: BackendId::MOCK,
                    command: "mock",
                    reason: "mock backend unsupported".to_owned(),
                }),
            );
        }
        MockBackend {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// A mock with every capability available except `status`'s capability.
    pub fn with_status(status: CapabilityStatus) -> Self {
        MockBackend {
            state: Arc::new(Mutex::new(MockState::with_state(status))),
        }
    }

    /// Replace one capability's status.
    pub fn set_status(&self, status: CapabilityStatus) {
        self.lock().statuses.insert(status.capability(), status);
    }

    /// One capability's status.
    pub fn status(&self, capability: Capability) -> CapabilityStatus {
        self.lock()
            .statuses
            .get(&capability)
            .cloned()
            .unwrap_or_else(|| {
                CapabilityStatus::Unsupported(Unsupported {
                    capability,
                    backend: BackendId::MOCK,
                    command: "mock",
                    reason: "mock has no status for this capability".to_owned(),
                })
            })
    }

    /// Whether one capability is available.
    pub fn is_available(&self, capability: Capability) -> bool {
        self.status(capability).is_available()
    }

    /// Make every write fail with `message` (not a denial).
    pub fn fail_writes(&self, message: impl Into<String>) {
        self.lock().write_failure = Some(message.into());
    }

    /// How many write requests the mock has accepted.
    pub fn writes(&self) -> u32 {
        self.lock().writes
    }

    /// The Updates adapter.
    pub fn updates(&self) -> MockUpdates {
        MockUpdates { mock: self.clone() }
    }

    /// The Firewall adapter.
    pub fn firewall(&self) -> MockFirewall {
        MockFirewall { mock: self.clone() }
    }

    /// The Users & Groups adapter.
    pub fn users(&self) -> MockUsers {
        MockUsers { mock: self.clone() }
    }

    /// The Time adapter.
    pub fn time(&self) -> MockTime {
        MockTime { mock: self.clone() }
    }

    /// The Locale adapter.
    pub fn locale(&self) -> MockLocale {
        MockLocale { mock: self.clone() }
    }

    /// The Autologin adapter.
    pub fn autologin(&self) -> MockAutologin {
        MockAutologin { mock: self.clone() }
    }

    /// The Default Apps adapter.
    pub fn default_apps(&self) -> MockDefaultApps {
        MockDefaultApps { mock: self.clone() }
    }

    /// The App Discovery adapter.
    pub fn app_discovery(&self) -> MockAppDiscovery {
        MockAppDiscovery { mock: self.clone() }
    }

    fn lock(&self) -> MutexGuard<'_, MockState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn guard(&self, capability: Capability) -> Result<(), DistroError> {
        match self.status(capability) {
            CapabilityStatus::Available(_) => Ok(()),
            CapabilityStatus::Absent(absence) => {
                Err(DistroError::absent(capability, absence.reason))
            }
            CapabilityStatus::Unsupported(unsupported) => Err(DistroError::not_supported(
                capability,
                unsupported.backend,
                unsupported.command,
            )),
        }
    }

    fn begin_write(&self, capability: Capability) -> Result<(), DistroError> {
        self.guard(capability)?;
        let mut state = self.lock();
        if let Some(message) = &state.write_failure {
            return Err(DistroError::failed(capability, message.clone()));
        }
        state.writes += 1;
        Ok(())
    }
}

macro_rules! mock_adapter {
    ($name:ident, $capability:expr) => {
        impl CapabilityAdapter for $name {
            fn capability(&self) -> Capability {
                $capability
            }

            fn backend(&self) -> BackendId {
                BackendId::MOCK
            }

            fn status(&self) -> CapabilityStatus {
                self.mock.status($capability)
            }
        }
    };
}

/// The Updates mock.
#[derive(Debug, Clone)]
pub struct MockUpdates {
    mock: MockBackend,
}
mock_adapter!(MockUpdates, Capability::Updates);

impl UpdateProvider for MockUpdates {
    fn distribution_info(&self) -> Result<DistributionInfo, DistroError> {
        self.mock.guard(Capability::Updates)?;
        Ok(DistributionInfo {
            id: "mock".to_owned(),
            name: "Mock Linux".to_owned(),
            version: "1".to_owned(),
            backend: BackendId::MOCK,
        })
    }

    fn check_updates(&self) -> Result<Vec<Update>, DistroError> {
        self.mock.guard(Capability::Updates)?;
        Ok(self.mock.lock().updates.clone())
    }

    fn install_updates(&self, ids: &[UpdateId]) -> Result<UpdateProgress, DistroError> {
        self.mock.begin_write(Capability::Updates)?;
        let mut state = self.mock.lock();
        let total = ids.len() as u32;
        state
            .updates
            .retain(|update| !ids.iter().any(|id| id.as_str() == update.id));
        state.reboot_required = true;
        Ok(UpdateProgress {
            phase: UpdatePhase::RebootRequired,
            completed: total,
            total,
            message: None,
        })
    }

    fn reboot_required(&self) -> Result<bool, DistroError> {
        self.mock.guard(Capability::Updates)?;
        Ok(self.mock.lock().reboot_required)
    }
}

/// The Firewall mock.
#[derive(Debug, Clone)]
pub struct MockFirewall {
    mock: MockBackend,
}
mock_adapter!(MockFirewall, Capability::Firewall);

impl FirewallProvider for MockFirewall {
    fn firewall_status(&self) -> Result<FirewallStatus, DistroError> {
        self.mock.guard(Capability::Firewall)?;
        Ok(self.mock.lock().firewall.clone())
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Firewall)?;
        self.mock.lock().firewall.enabled = enabled;
        Ok(())
    }

    fn open_port(&self, port: Port) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Firewall)?;
        let mut state = self.mock.lock();
        if !state.firewall.open_ports.contains(&port) {
            state.firewall.open_ports.push(port);
        }
        Ok(())
    }

    fn close_port(&self, port: Port) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Firewall)?;
        self.mock
            .lock()
            .firewall
            .open_ports
            .retain(|open| *open != port);
        Ok(())
    }
}

/// The Users & Groups mock.
#[derive(Debug, Clone)]
pub struct MockUsers {
    mock: MockBackend,
}
mock_adapter!(MockUsers, Capability::UsersGroups);

impl UserGroupProvider for MockUsers {
    fn admin_groups(&self) -> Result<Vec<String>, DistroError> {
        self.mock.guard(Capability::UsersGroups)?;
        Ok(self.mock.lock().admin_groups.clone())
    }

    fn list_groups(&self) -> Result<Vec<GroupRecord>, DistroError> {
        self.mock.guard(Capability::UsersGroups)?;
        Ok(self.mock.lock().groups.clone())
    }

    fn create_user(&self, user: &NewUser) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        self.mock.lock().users.push(user.clone());
        Ok(())
    }

    fn delete_user(&self, user_name: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        self.mock
            .lock()
            .users
            .retain(|user| user.user_name != user_name);
        Ok(())
    }

    fn set_password(&self, _user_name: &str, _password: &Secret) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        Ok(())
    }

    fn create_group(&self, name: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        let mut state = self.mock.lock();
        if !state.groups.iter().any(|group| group.name == name) {
            let gid = state
                .groups
                .iter()
                .map(|group| group.gid)
                .max()
                .unwrap_or(1000)
                + 1;
            state.groups.push(GroupRecord {
                name: name.to_owned(),
                gid,
                members: Vec::new(),
                system: false,
            });
        }
        Ok(())
    }

    fn delete_group(&self, name: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        self.mock.lock().groups.retain(|group| group.name != name);
        Ok(())
    }

    fn set_group_members(&self, name: &str, members: &[String]) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::UsersGroups)?;
        let mut state = self.mock.lock();
        if let Some(group) = state.groups.iter_mut().find(|group| group.name == name) {
            group.members = members.to_vec();
        }
        Ok(())
    }
}

impl MockUsers {
    /// The users created through this mock (test introspection).
    pub fn created_users(&self) -> Vec<NewUser> {
        self.mock.lock().users.clone()
    }
}

/// The Time mock.
#[derive(Debug, Clone)]
pub struct MockTime {
    mock: MockBackend,
}
mock_adapter!(MockTime, Capability::Time);

impl TimeProvider for MockTime {
    fn time_status(&self) -> Result<TimeStatus, DistroError> {
        self.mock.guard(Capability::Time)?;
        Ok(self.mock.lock().time.clone())
    }

    fn set_timezone(&self, timezone: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Time)?;
        self.mock.lock().time.timezone = timezone.to_owned();
        Ok(())
    }

    fn set_ntp(&self, enabled: bool) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Time)?;
        self.mock.lock().time.ntp_enabled = enabled;
        Ok(())
    }
}

/// The Locale mock.
#[derive(Debug, Clone)]
pub struct MockLocale {
    mock: MockBackend,
}
mock_adapter!(MockLocale, Capability::Locale);

impl LocaleProvider for MockLocale {
    fn locale_status(&self) -> Result<LocaleStatus, DistroError> {
        self.mock.guard(Capability::Locale)?;
        Ok(self.mock.lock().locale.clone())
    }

    fn available_locales(&self) -> Result<Vec<String>, DistroError> {
        self.mock.guard(Capability::Locale)?;
        Ok(self.mock.lock().locales.clone())
    }

    fn set_locale(&self, locale: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Locale)?;
        self.mock.lock().locale.locale = locale.to_owned();
        Ok(())
    }
}

/// The Autologin mock.
#[derive(Debug, Clone)]
pub struct MockAutologin {
    mock: MockBackend,
}
mock_adapter!(MockAutologin, Capability::Autologin);

impl AutologinProvider for MockAutologin {
    fn autologin_status(&self) -> Result<AutologinStatus, DistroError> {
        self.mock.guard(Capability::Autologin)?;
        Ok(self.mock.lock().autologin.clone())
    }

    fn set_autologin(&self, user: Option<&str>) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::Autologin)?;
        self.mock.lock().autologin.user = user.map(str::to_owned);
        Ok(())
    }
}

/// The Default Apps mock.
#[derive(Debug, Clone)]
pub struct MockDefaultApps {
    mock: MockBackend,
}
mock_adapter!(MockDefaultApps, Capability::DefaultApps);

impl DefaultAppsProvider for MockDefaultApps {
    fn roles(&self) -> Result<Vec<DefaultApp>, DistroError> {
        self.mock.guard(Capability::DefaultApps)?;
        let state = self.mock.lock();
        Ok(vec![
            DefaultApp {
                role: "browser".to_owned(),
                desktop_id: state.default_apps.get("browser").cloned(),
                mime_types: vec!["text/html".to_owned(), "x-scheme-handler/http".to_owned()],
            },
            DefaultApp {
                role: "mail".to_owned(),
                desktop_id: state.default_apps.get("mail").cloned(),
                mime_types: vec!["x-scheme-handler/mailto".to_owned()],
            },
            DefaultApp {
                role: "file-manager".to_owned(),
                desktop_id: state.default_apps.get("file-manager").cloned(),
                mime_types: vec!["inode/directory".to_owned()],
            },
        ])
    }

    fn default_app(&self, role: &str) -> Result<Option<String>, DistroError> {
        self.mock.guard(Capability::DefaultApps)?;
        Ok(self.mock.lock().default_apps.get(role).cloned())
    }

    fn set_default_app(&self, role: &str, desktop_id: &str) -> Result<(), DistroError> {
        self.mock.begin_write(Capability::DefaultApps)?;
        self.mock
            .lock()
            .default_apps
            .insert(role.to_owned(), desktop_id.to_owned());
        Ok(())
    }

    fn mime_handler(&self, mime: &str) -> Result<Option<String>, DistroError> {
        self.mock.guard(Capability::DefaultApps)?;
        Ok(self.mock.lock().mime_handlers.get(mime).cloned())
    }
}

/// The App Discovery mock.
#[derive(Debug, Clone)]
pub struct MockAppDiscovery {
    mock: MockBackend,
}
mock_adapter!(MockAppDiscovery, Capability::AppDiscovery);

impl AppDiscoveryProvider for MockAppDiscovery {
    fn discovery_paths(&self) -> Result<Vec<AppSource>, DistroError> {
        self.mock.guard(Capability::AppDiscovery)?;
        Ok(self.mock.lock().discovery.clone())
    }

    fn install_sources(&self) -> Result<Vec<InstallSource>, DistroError> {
        self.mock.guard(Capability::AppDiscovery)?;
        Ok(self.mock.lock().install_sources.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_mock_is_available_with_fixture_data() {
        let mock = MockBackend::available();
        for capability in Capability::ALL {
            assert!(mock.is_available(capability), "{capability:?}");
        }
        assert_eq!(mock.updates().check_updates().unwrap().len(), 1);
        assert!(mock.firewall().firewall_status().unwrap().enabled);
        assert_eq!(mock.users().admin_groups().unwrap(), vec!["wheel"]);
        assert_eq!(mock.time().time_status().unwrap().timezone, "UTC");
        assert_eq!(mock.locale().locale_status().unwrap().locale, "en_US.UTF-8");
        assert!(mock.autologin().autologin_status().unwrap().supported);
        assert_eq!(
            mock.default_apps().default_app("browser").unwrap(),
            Some("org.mozilla.firefox.desktop".to_owned())
        );
        assert!(!mock.app_discovery().discovery_paths().unwrap().is_empty());
    }

    #[test]
    fn the_mock_drives_the_three_states() {
        let mock = MockBackend::absent();
        assert!(matches!(
            mock.updates().check_updates(),
            Err(DistroError::Absent { .. })
        ));
        let mock = MockBackend::unsupported();
        assert!(matches!(
            mock.updates().check_updates(),
            Err(DistroError::NotSupported { .. })
        ));
        let mock = MockBackend::with_status(CapabilityStatus::Absent(Absence {
            capability: Capability::Firewall,
            missing: vec!["firewalld"],
            reason: "No firewall detected".to_owned(),
        }));
        assert!(mock.updates().check_updates().is_ok());
        assert!(matches!(
            mock.firewall().firewall_status(),
            Err(DistroError::Absent { .. })
        ));
    }

    #[test]
    fn writes_mutate_the_mock_and_are_counted() {
        let mock = MockBackend::available();
        let updates = mock.updates();
        let progress = updates
            .install_updates(&[UpdateId::from("mock-kernel")])
            .unwrap();
        assert_eq!(progress.phase, UpdatePhase::RebootRequired);
        assert!(updates.check_updates().unwrap().is_empty());
        assert!(updates.reboot_required().unwrap());

        let firewall = mock.firewall();
        firewall.set_enabled(false).unwrap();
        firewall.open_port(Port::new(22, Protocol::Tcp)).unwrap();
        assert!(!firewall.firewall_status().unwrap().enabled);
        assert!(firewall
            .firewall_status()
            .unwrap()
            .open_ports
            .contains(&Port::new(22, Protocol::Tcp)));

        let users = mock.users();
        users
            .create_user(&NewUser {
                user_name: "dan".to_owned(),
                real_name: "Dan Doe".to_owned(),
                administrator: true,
                shell: None,
            })
            .unwrap();
        assert_eq!(users.created_users().len(), 1);
        users.set_password("dan", &Secret::new("hunter2")).unwrap();

        let time = mock.time();
        time.set_timezone("Europe/Berlin").unwrap();
        time.set_ntp(false).unwrap();
        assert_eq!(time.time_status().unwrap().timezone, "Europe/Berlin");

        let locale = mock.locale();
        locale.set_locale("de_DE.UTF-8").unwrap();
        assert_eq!(locale.locale_status().unwrap().locale, "de_DE.UTF-8");

        let autologin = mock.autologin();
        autologin.set_autologin(Some("dan")).unwrap();
        assert_eq!(
            autologin.autologin_status().unwrap().user,
            Some("dan".to_owned())
        );

        let apps = mock.default_apps();
        apps.set_default_app("browser", "org.chromium.Chromium.desktop")
            .unwrap();
        assert_eq!(
            apps.default_app("browser").unwrap(),
            Some("org.chromium.Chromium.desktop".to_owned())
        );

        assert!(mock.writes() >= 10);
    }

    #[test]
    fn a_write_failure_is_a_failed_error_not_a_denial() {
        let mock = MockBackend::available();
        mock.fail_writes("polkit: not authorized");
        let error = mock.firewall().set_enabled(true).unwrap_err();
        assert!(error.is_failed());
        assert!(error.to_string().contains("polkit: not authorized"));
        assert_eq!(mock.writes(), 0);
    }

    #[test]
    fn status_can_be_replaced_per_capability() {
        let mock = MockBackend::available();
        mock.set_status(CapabilityStatus::Unsupported(Unsupported {
            capability: Capability::Locale,
            backend: BackendId::MOCK,
            command: "mock",
            reason: "not here".to_owned(),
        }));
        assert!(mock.updates().available());
        assert!(!mock.locale().available());
        assert!(mock.locale().status().is_unsupported());
    }
}
