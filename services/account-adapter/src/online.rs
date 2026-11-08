// SPDX-License-Identifier: MIT
//! The Online Accounts half of the account adapter (T228).
//!
//! Online Accounts is **optional**. When `org.gnome.OnlineAccounts` is
//! not on the session bus the whole view is absent and the Settings pane is
//! hidden, not shown as a dead entry (ADR 0229). When it is present:
//!
//! * the account list comes from the manager's
//!   `org.freedesktop.DBus.ObjectManager.GetManagedObjects`;
//! * the provider list comes from the installed
//!   `/usr/share/gnome-online-accounts/providers/*.provider` descriptors; // df-allow-desktop-name: GOA package path
//! * sync toggles set the account's `*Disabled` properties;
//! * add/remove call the manager's `AddAccount` and the account's `Remove`.
//!
//! The adapter never implements OAuth and never stores a credential. The
//! provider owns the account; this is a projection of its D-Bus state.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::biometrics::service_available;
use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// The GOA session-bus name.
pub const GOA_SERVICE: &str = "org.gnome.OnlineAccounts";
/// The GOA manager object path.
pub const GOA_MANAGER_PATH: &str = "/org/gnome/OnlineAccounts"; // df-allow-desktop-name: GOA object path
/// The GOA manager interface.
pub const GOA_MANAGER_INTERFACE: &str = "org.gnome.OnlineAccounts.Manager";
/// The GOA account interface.
pub const GOA_ACCOUNT_INTERFACE: &str = "org.gnome.OnlineAccounts.Account";
/// The standard properties interface.
pub const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";

/// One capability a GOA account can expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnlineCapability {
    /// Stable id (`mail`, `calendar`, …).
    pub id: &'static str,
    /// The label the pane draws.
    pub label: &'static str,
    /// The account property that disables it.
    pub disabled_property: &'static str,
    /// The interface that appears when it is enabled.
    pub interface: &'static str,
}

/// The capability vocabulary, in the pane's order.
pub const ONLINE_CAPABILITIES: [OnlineCapability; 12] = [
    OnlineCapability {
        id: "mail",
        label: "Mail",
        disabled_property: "MailDisabled",
        interface: "org.gnome.OnlineAccounts.Mail",
    },
    OnlineCapability {
        id: "calendar",
        label: "Calendar",
        disabled_property: "CalendarDisabled",
        interface: "org.gnome.OnlineAccounts.Calendar",
    },
    OnlineCapability {
        id: "contacts",
        label: "Contacts",
        disabled_property: "ContactsDisabled",
        interface: "org.gnome.OnlineAccounts.Contacts",
    },
    OnlineCapability {
        id: "chat",
        label: "Chat",
        disabled_property: "ChatDisabled",
        interface: "org.gnome.OnlineAccounts.Chat",
    },
    OnlineCapability {
        id: "documents",
        label: "Documents",
        disabled_property: "DocumentsDisabled",
        interface: "org.gnome.OnlineAccounts.Documents",
    },
    OnlineCapability {
        id: "photos",
        label: "Photos",
        disabled_property: "PhotosDisabled",
        interface: "org.gnome.OnlineAccounts.Photos",
    },
    OnlineCapability {
        id: "files",
        label: "Files",
        disabled_property: "FilesDisabled",
        interface: "org.gnome.OnlineAccounts.Files",
    },
    OnlineCapability {
        id: "ticketing",
        label: "Ticketing",
        disabled_property: "TicketingDisabled",
        interface: "org.gnome.OnlineAccounts.Ticketing",
    },
    OnlineCapability {
        id: "todo",
        label: "To Do",
        disabled_property: "TodoDisabled",
        interface: "org.gnome.OnlineAccounts.Todo",
    },
    OnlineCapability {
        id: "readlater",
        label: "Read Later",
        disabled_property: "ReadLaterDisabled",
        interface: "org.gnome.OnlineAccounts.ReadLater",
    },
    OnlineCapability {
        id: "printers",
        label: "Printers",
        disabled_property: "PrintersDisabled",
        interface: "org.gnome.OnlineAccounts.Printers",
    },
    OnlineCapability {
        id: "music",
        label: "Music",
        disabled_property: "MusicDisabled",
        interface: "org.gnome.OnlineAccounts.Music",
    },
];

/// One provider descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineProviderData {
    /// The provider type (`google`, `nextcloud`, …).
    pub id: String,
    /// The display name.
    pub name: String,
    /// The icon name/string, when the descriptor carries one.
    pub icon: String,
    /// Whether the daemon reports it as supported.
    pub supported: bool,
}

/// One sync capability on an account.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineCapabilityData {
    /// Stable id (`mail`, `calendar`, …).
    pub id: String,
    /// The label.
    pub label: String,
    /// Whether the capability is enabled.
    pub enabled: bool,
}

/// One online account.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineAccountData {
    /// The stable account id (`Id` property).
    pub id: String,
    /// The account object path.
    pub path: String,
    /// The provider type.
    pub provider: String,
    /// The provider display name.
    pub provider_name: String,
    /// The identity.
    pub identity: String,
    /// The presentation identity.
    pub presentation_identity: String,
    /// Whether the account is locked by policy.
    pub locked: bool,
    /// Whether the account needs attention (expired credentials).
    pub attention_needed: bool,
    /// The sync capabilities the account exposes.
    pub capabilities: Vec<OnlineCapabilityData>,
}

/// The raw Online Accounts read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineAccountsData {
    /// The installed providers.
    pub providers: Vec<OnlineProviderData>,
    /// The configured accounts.
    pub accounts: Vec<OnlineAccountData>,
}

/// The result of one online-accounts write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnlineAccountsOutcome {
    /// The daemon accepted the request.
    Applied,
    /// Policy refused the request.
    Denied(String),
    /// The daemon is absent.
    Absent,
    /// The request failed.
    Failed(AdapterError),
}

impl OnlineAccountsOutcome {
    /// Whether the daemon accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, OnlineAccountsOutcome::Applied)
    }

    /// The refusal or failure text.
    pub fn message(&self) -> Option<&str> {
        match self {
            OnlineAccountsOutcome::Denied(message) => Some(message),
            OnlineAccountsOutcome::Failed(error) => Some(error.message()),
            OnlineAccountsOutcome::Applied | OnlineAccountsOutcome::Absent => None,
        }
    }
}

/// Reads the Online Accounts host stack and drives its writes.
pub trait OnlineAccountsSource {
    /// One read of the GOA manager and its accounts.
    fn read_online_accounts(&mut self) -> Result<Option<OnlineAccountsData>, AdapterError>;

    /// Add an account for `provider` and `identity`. One explicit write.
    fn add_online_account(&mut self, _provider: &str, _identity: &str) -> OnlineAccountsOutcome {
        OnlineAccountsOutcome::Absent
    }

    /// Remove the account at `id`. One explicit write.
    fn remove_online_account(&mut self, _id: &str) -> OnlineAccountsOutcome {
        OnlineAccountsOutcome::Absent
    }

    /// Enable or disable one sync capability. One explicit write.
    fn set_online_account_sync(
        &mut self,
        _id: &str,
        _capability: &str,
        _enabled: bool,
    ) -> OnlineAccountsOutcome {
        OnlineAccountsOutcome::Absent
    }
}

/// The typed Online Accounts snapshot the pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnlineAccountsSnapshot {
    data: OnlineAccountsData,
}

impl OnlineAccountsSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &OnlineAccountsData) -> Self {
        OnlineAccountsSnapshot { data: data.clone() }
    }

    /// The installed providers.
    pub fn providers(&self) -> &[OnlineProviderData] {
        &self.data.providers
    }

    /// The configured accounts.
    pub fn accounts(&self) -> &[OnlineAccountData] {
        &self.data.accounts
    }

    /// The account with `id`.
    pub fn account(&self, id: &str) -> Option<&OnlineAccountData> {
        self.data.accounts.iter().find(|account| account.id == id)
    }

    /// Whether any account is configured.
    pub fn present(&self) -> bool {
        !self.data.accounts.is_empty()
    }

    /// The pane header label.
    pub fn label(&self) -> String {
        match self.data.accounts.len() {
            0 => "No Accounts".to_owned(),
            1 => "1 Account".to_owned(),
            count => format!("{count} Accounts"),
        }
    }

    /// The design-system glyph.
    pub fn glyph(&self) -> &'static str {
        "cloud"
    }
}

/// The Online Accounts adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct OnlineAccountsAdapter<S> {
    source: S,
    state: AdapterState<OnlineAccountsSnapshot>,
    subscription: Subscription,
}

impl<S: OnlineAccountsSource> OnlineAccountsAdapter<S> {
    /// A fresh adapter over `source`.
    pub fn new(source: S) -> Self {
        OnlineAccountsAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
        }
    }

    /// Read the GOA manager once.
    pub fn refresh(&mut self) {
        match self.source.read_online_accounts() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(OnlineAccountsSnapshot::from_data(&data));
                self.subscription.changed();
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

    /// Add an account.
    pub fn add_account(&mut self, provider: &str, identity: &str) -> OnlineAccountsOutcome {
        self.source.add_online_account(provider, identity)
    }

    /// Remove an account.
    pub fn remove_account(&mut self, id: &str) -> OnlineAccountsOutcome {
        self.source.remove_online_account(id)
    }

    /// Set one sync capability.
    pub fn set_sync(&mut self, id: &str, capability: &str, enabled: bool) -> OnlineAccountsOutcome {
        self.source.set_online_account_sync(id, capability, enabled)
    }

    /// The live snapshot.
    pub fn snapshot(&self) -> Option<&OnlineAccountsSnapshot> {
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
}

impl<S: OnlineAccountsSource> Adapter for OnlineAccountsAdapter<S> {
    type Snapshot = OnlineAccountsSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::ONLINE_ACCOUNTS
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

/// A deterministic online-accounts fixture for CI and pane tests.
pub fn online_accounts_fixture() -> OnlineAccountsData {
    OnlineAccountsData {
        providers: vec![
            OnlineProviderData {
                id: "google".to_owned(),
                name: "Google".to_owned(),
                icon: "google".to_owned(),
                supported: true,
            },
            OnlineProviderData {
                id: "nextcloud".to_owned(),
                name: "Nextcloud".to_owned(),
                icon: "nextcloud".to_owned(),
                supported: true,
            },
        ],
        accounts: vec![OnlineAccountData {
            id: "account-1".to_owned(),
            path: "/org/gnome/OnlineAccounts/Accounts/account_1".to_owned(), // df-allow-desktop-name: GOA object path
            provider: "google".to_owned(),
            provider_name: "Google".to_owned(),
            identity: "dan@example.com".to_owned(),
            presentation_identity: "Dan Doe".to_owned(),
            locked: false,
            attention_needed: false,
            capabilities: vec![
                OnlineCapabilityData {
                    id: "mail".to_owned(),
                    label: "Mail".to_owned(),
                    enabled: true,
                },
                OnlineCapabilityData {
                    id: "calendar".to_owned(),
                    label: "Calendar".to_owned(),
                    enabled: true,
                },
                OnlineCapabilityData {
                    id: "contacts".to_owned(),
                    label: "Contacts".to_owned(),
                    enabled: false,
                },
            ],
        }],
    }
}

/// A deterministic online-accounts source for CI and pane tests.
#[derive(Debug, Clone, PartialEq)]
pub struct MockOnlineAccounts {
    present: bool,
    data: Option<OnlineAccountsData>,
    write_failure: Option<AdapterError>,
    deny_writes: Option<String>,
    writes: u32,
    last_add: Option<(String, String)>,
    last_remove: Option<String>,
    last_sync: Option<(String, String, bool)>,
}

impl MockOnlineAccounts {
    /// GOA is not installed.
    pub fn absent() -> Self {
        MockOnlineAccounts {
            present: false,
            data: None,
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_add: None,
            last_remove: None,
            last_sync: None,
        }
    }

    /// GOA answers with the deterministic fixture.
    pub fn present() -> Self {
        MockOnlineAccounts {
            present: true,
            data: Some(online_accounts_fixture()),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_add: None,
            last_remove: None,
            last_sync: None,
        }
    }

    /// GOA answers with `data`.
    pub fn with_data(data: OnlineAccountsData) -> Self {
        MockOnlineAccounts {
            present: true,
            data: Some(data),
            write_failure: None,
            deny_writes: None,
            writes: 0,
            last_add: None,
            last_remove: None,
            last_sync: None,
        }
    }

    /// Make every write fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// Make every write a denial.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.deny_writes = Some(note.into());
        self
    }

    /// Publish fresh state.
    pub fn push(&mut self, data: OnlineAccountsData) {
        self.present = true;
        self.data = Some(data);
    }

    /// GOA goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// GOA comes back.
    pub fn restart(&mut self) {
        self.present = true;
    }

    /// The write count.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// The last add request.
    pub fn last_add(&self) -> Option<&(String, String)> {
        self.last_add.as_ref()
    }

    /// The last remove request.
    pub fn last_remove(&self) -> Option<&str> {
        self.last_remove.as_deref()
    }

    /// The last sync write.
    pub fn last_sync(&self) -> Option<&(String, String, bool)> {
        self.last_sync.as_ref()
    }

    fn outcome(&self) -> OnlineAccountsOutcome {
        if !self.present {
            return OnlineAccountsOutcome::Absent;
        }
        if let Some(note) = &self.deny_writes {
            return OnlineAccountsOutcome::Denied(note.clone());
        }
        if let Some(error) = &self.write_failure {
            return OnlineAccountsOutcome::Failed(error.clone());
        }
        OnlineAccountsOutcome::Applied
    }
}

impl OnlineAccountsSource for MockOnlineAccounts {
    fn read_online_accounts(&mut self) -> Result<Option<OnlineAccountsData>, AdapterError> {
        if !self.present {
            return Ok(None);
        }
        Ok(self.data.clone())
    }

    fn add_online_account(&mut self, provider: &str, identity: &str) -> OnlineAccountsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_add = Some((provider.to_owned(), identity.to_owned()));
            if let Some(data) = self.data.as_mut() {
                let id = format!("account-{}", data.accounts.len() + 1);
                let name = data
                    .providers
                    .iter()
                    .find(|candidate| candidate.id == provider)
                    .map(|candidate| candidate.name.clone())
                    .unwrap_or_else(|| provider.to_owned());
                data.accounts.push(OnlineAccountData {
                    id,
                    path: format!(
                        "/org/gnome/OnlineAccounts/Accounts/account_{}", // df-allow-desktop-name: GOA object path
                        data.accounts.len() + 1
                    ),
                    provider: provider.to_owned(),
                    provider_name: name,
                    identity: identity.to_owned(),
                    presentation_identity: identity.to_owned(),
                    locked: false,
                    attention_needed: false,
                    capabilities: vec![OnlineCapabilityData {
                        id: "mail".to_owned(),
                        label: "Mail".to_owned(),
                        enabled: true,
                    }],
                });
            }
        }
        outcome
    }

    fn remove_online_account(&mut self, id: &str) -> OnlineAccountsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_remove = Some(id.to_owned());
            if let Some(data) = self.data.as_mut() {
                data.accounts.retain(|account| account.id != id);
            }
        }
        outcome
    }

    fn set_online_account_sync(
        &mut self,
        id: &str,
        capability: &str,
        enabled: bool,
    ) -> OnlineAccountsOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.writes += 1;
            self.last_sync = Some((id.to_owned(), capability.to_owned(), enabled));
            if let Some(data) = self.data.as_mut() {
                if let Some(account) = data.accounts.iter_mut().find(|account| account.id == id) {
                    if let Some(capability) = account
                        .capabilities
                        .iter_mut()
                        .find(|candidate| candidate.id == capability)
                    {
                        capability.enabled = enabled;
                    }
                }
            }
        }
        outcome
    }
}

/// Read the live GOA manager.
pub fn read_host_online_accounts() -> Result<Option<OnlineAccountsData>, AdapterError> {
    let connection = match Connection::session() {
        Ok(connection) => connection,
        Err(_) => return Ok(None),
    };
    if !service_available(&connection, GOA_SERVICE) {
        return Ok(None);
    }
    let reply = connection
        .call_method(
            Some(GOA_SERVICE),
            GOA_MANAGER_PATH,
            Some("org.freedesktop.DBus.ObjectManager"),
            "GetManagedObjects",
            &(),
        )
        .map_err(failed)?;
    let objects: HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>> =
        reply.body().deserialize().map_err(failed)?;
    let providers = read_providers();
    let mut accounts = Vec::new();
    for (path, interfaces) in objects {
        let Some(props) = interfaces.get(GOA_ACCOUNT_INTERFACE) else {
            continue;
        };
        let id = string_prop(props, "Id");
        let provider = string_prop(props, "ProviderType");
        let provider_name = string_prop(props, "ProviderName");
        let identity = string_prop(props, "Identity");
        let presentation_identity = string_prop(props, "PresentationIdentity");
        let locked = bool_prop(props, "IsLocked");
        let attention_needed = bool_prop(props, "AttentionNeeded");
        let mut capabilities = Vec::new();
        for capability in ONLINE_CAPABILITIES {
            let disabled = bool_prop(props, capability.disabled_property);
            let interface_present = interfaces.contains_key(capability.interface);
            if interface_present || disabled {
                capabilities.push(OnlineCapabilityData {
                    id: capability.id.to_owned(),
                    label: capability.label.to_owned(),
                    enabled: !disabled,
                });
            }
        }
        accounts.push(OnlineAccountData {
            id,
            path: path.to_string(),
            provider,
            provider_name,
            identity,
            presentation_identity,
            locked,
            attention_needed,
            capabilities,
        });
    }
    accounts.sort_by(|left, right| left.presentation_identity.cmp(&right.presentation_identity));
    Ok(Some(OnlineAccountsData {
        providers,
        accounts,
    }))
}

/// Add an account on the live GOA manager.
pub fn add_host_online_account(provider: &str, identity: &str) -> OnlineAccountsOutcome {
    if provider.is_empty() || identity.is_empty() {
        return OnlineAccountsOutcome::Denied("a provider and identity are required".to_owned());
    }
    let connection = match goa_connection() {
        Some(connection) => connection,
        None => return OnlineAccountsOutcome::Absent,
    };
    let credentials: HashMap<String, OwnedValue> = HashMap::new();
    let details: HashMap<String, String> = HashMap::new();
    match connection.call_method(
        Some(GOA_SERVICE),
        GOA_MANAGER_PATH,
        Some(GOA_MANAGER_INTERFACE),
        "AddAccount",
        &(provider, identity, identity, credentials, details),
    ) {
        Ok(_) => OnlineAccountsOutcome::Applied,
        Err(error) => classify(error),
    }
}

/// Remove an account on the live GOA manager, by its stable `Id`.
pub fn remove_host_online_account(id: &str) -> OnlineAccountsOutcome {
    if id.is_empty() {
        return OnlineAccountsOutcome::Absent;
    }
    let data = match read_host_online_accounts() {
        Ok(Some(data)) => data,
        Ok(None) => return OnlineAccountsOutcome::Absent,
        Err(error) => return OnlineAccountsOutcome::Failed(error),
    };
    let Some(account) = data.accounts.into_iter().find(|account| account.id == id) else {
        return OnlineAccountsOutcome::Absent;
    };
    let connection = match goa_connection() {
        Some(connection) => connection,
        None => return OnlineAccountsOutcome::Absent,
    };
    match connection.call_method(
        Some(GOA_SERVICE),
        account.path.as_str(),
        Some(GOA_ACCOUNT_INTERFACE),
        "Remove",
        &(),
    ) {
        Ok(_) => OnlineAccountsOutcome::Applied,
        Err(error) => classify(error),
    }
}

/// Set one `*Disabled` property on the live account.
pub fn set_host_online_account_sync(
    id: &str,
    capability: &str,
    enabled: bool,
) -> OnlineAccountsOutcome {
    let Some(capability) = ONLINE_CAPABILITIES
        .iter()
        .find(|candidate| candidate.id == capability)
    else {
        return OnlineAccountsOutcome::Denied(format!("unknown capability: {capability}"));
    };
    let data = match read_host_online_accounts() {
        Ok(Some(data)) => data,
        Ok(None) => return OnlineAccountsOutcome::Absent,
        Err(error) => return OnlineAccountsOutcome::Failed(error),
    };
    let Some(account) = data.accounts.into_iter().find(|account| account.id == id) else {
        return OnlineAccountsOutcome::Absent;
    };
    let connection = match goa_connection() {
        Some(connection) => connection,
        None => return OnlineAccountsOutcome::Absent,
    };
    let value = zbus::zvariant::Value::from(!enabled);
    match connection.call_method(
        Some(GOA_SERVICE),
        account.path.as_str(),
        Some(PROPERTIES_INTERFACE),
        "Set",
        &(GOA_ACCOUNT_INTERFACE, capability.disabled_property, value),
    ) {
        Ok(_) => OnlineAccountsOutcome::Applied,
        Err(error) => classify(error),
    }
}

fn goa_connection() -> Option<Connection> {
    let connection = Connection::session().ok()?;
    service_available(&connection, GOA_SERVICE).then_some(connection)
}

fn read_providers() -> Vec<OnlineProviderData> {
    let provider_dir = "gnome-online-accounts/providers"; // df-allow-desktop-name: GOA package path
    let mut dirs = vec![PathBuf::from("/usr/share").join(provider_dir)];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join(provider_dir));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share").join(provider_dir));
    }
    let mut providers = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("provider") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_owned();
            let mut name = id.clone();
            let mut icon = String::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(value) = line.strip_prefix("ProviderName=") {
                    name = value.trim().to_owned();
                } else if let Some(value) = line.strip_prefix("ProviderIcon=") {
                    icon = value.trim().to_owned();
                }
            }
            if providers
                .iter()
                .any(|provider: &OnlineProviderData| provider.id == id)
            {
                continue;
            }
            providers.push(OnlineProviderData {
                id,
                name,
                icon,
                supported: true,
            });
        }
    }
    providers.sort_by(|left, right| left.name.cmp(&right.name));
    providers
}

fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("OnlineAccounts: {error}"))
}

fn string_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<String>().ok())
        .unwrap_or_default()
}

fn bool_prop(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or_default()
}

fn classify(error: zbus::Error) -> OnlineAccountsOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    let lower = error.to_string().to_lowercase();
    if name.contains("AccessDenied")
        || name.contains("NotAuthorized")
        || lower.contains("not authorized")
        || lower.contains("polkit")
    {
        OnlineAccountsOutcome::Denied(format!("OnlineAccounts: {error}"))
    } else {
        OnlineAccountsOutcome::Failed(AdapterError::new(format!("OnlineAccounts: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_is_deterministic() {
        assert_eq!(online_accounts_fixture(), online_accounts_fixture());
    }

    #[test]
    fn an_absent_goa_is_unavailable() {
        let mut adapter = OnlineAccountsAdapter::new(MockOnlineAccounts::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_present_goa_reports_providers_and_accounts() {
        let mut adapter = OnlineAccountsAdapter::new(MockOnlineAccounts::present());
        adapter.refresh();
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.providers().len(), 2);
        assert_eq!(snapshot.accounts().len(), 1);
        assert_eq!(snapshot.label(), "1 Account");
        let account = snapshot.account("account-1").unwrap();
        assert_eq!(account.provider_name, "Google");
        assert_eq!(account.capabilities.len(), 3);
        assert!(!account.capabilities[2].enabled);
    }

    #[test]
    fn add_and_remove_mutate_the_mock() {
        let mut adapter = OnlineAccountsAdapter::new(MockOnlineAccounts::present());
        adapter.refresh();
        assert!(adapter
            .add_account("nextcloud", "dan@cloud.example")
            .is_applied());
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().accounts().len(), 2);
        assert!(adapter.remove_account("account-2").is_applied());
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().accounts().len(), 1);
        assert_eq!(adapter.source().writes(), 2);
    }

    #[test]
    fn a_sync_toggle_mutates_the_mock() {
        let mut adapter = OnlineAccountsAdapter::new(MockOnlineAccounts::present());
        adapter.refresh();
        assert!(adapter.set_sync("account-1", "contacts", true).is_applied());
        adapter.refresh();
        let account = adapter.snapshot().unwrap().account("account-1").unwrap();
        assert!(account.capabilities[2].enabled);
        assert_eq!(
            adapter.source().last_sync(),
            Some(&("account-1".to_owned(), "contacts".to_owned(), true))
        );
    }

    #[test]
    fn a_denied_write_keeps_the_state() {
        let mut adapter = OnlineAccountsAdapter::new(
            MockOnlineAccounts::present().deny_writes("account is locked"),
        );
        adapter.refresh();
        assert_eq!(
            adapter.remove_account("account-1"),
            OnlineAccountsOutcome::Denied("account is locked".to_owned())
        );
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().accounts().len(), 1);
    }
}
