// SPDX-License-Identifier: MIT
//! The live transport: AccountsService over its D-Bus API, plus the
//! distribution group-provider seam.
//!
//! The **users** half is AccountsService (`org.freedesktop.Accounts`) over the
//! **system bus** ([07-system-integration.md]). One read is
//! `ListCachedUsers` at `/org/freedesktop/Accounts` followed by
//! `org.freedesktop.DBus.Properties.GetAll("org.freedesktop.Accounts.User")`
//! on each returned user object, so one pass enumerates the whole account
//! list. The writes are single method calls, never a loop:
//!
//! * create — `CreateUser(name, real_name, account_type)` on the manager;
//! * delete — `DeleteUser(uid)` on the manager;
//! * account type — `SetAccountType(type)` on the user object;
//! * lock — `SetLocked(locked)` on the user object;
//! * automatic login — `SetAutomaticLogin(automatic_login)` on the user.
//!
//! The **groups** half is the distribution's own provider behind the
//! [`GroupProvider`] seam, because AccountsService has no group API. The
//! concrete provider is distro-specific and belongs with the platform packaging
//! work, so `HostAccounts::new` runs with no provider and reports groups as
//! absent until one is attached — exactly as `HostSystem` runs with no update
//! provider. Its group writes delegate to the provider or answer `Absent`.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all, or a bus where `org.freedesktop.Accounts` does not own
//! its name, is treated as absence; a name owned but unreadable is reported to
//! the adapter, which shows the item visible and inert. A polkit refusal on a
//! write is reported separately so the pane can surface it without treating the
//! daemon as broken.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use crate::source::{
    AccountData, AccountOutcome, AccountSource, AccountType, AccountsData, GroupData, PasswordMode,
};

/// The well-known name `accounts-daemon` owns.
pub const ACCOUNTS_SERVICE: &str = "org.freedesktop.Accounts";
/// The manager object AccountsService serves.
pub const ACCOUNTS_ROOT: &str = "/org/freedesktop/Accounts";
/// The manager interface.
pub const ACCOUNTS_INTERFACE: &str = "org.freedesktop.Accounts";
/// The user-object interface.
pub const USER_INTERFACE: &str = "org.freedesktop.Accounts.User";
/// The standard properties interface.
pub const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";

/// The distribution's own group provider.
///
/// This is the "distro provider" half of the Users & Groups routing
/// ([08-settings.md]), narrowed to groups because AccountsService already owns
/// the user list. A provider is owned by the session host and reports its own
/// state; the adapter never edits a group itself.
///
/// The provider is owned by a session host and may be driven from the bridge
/// host's D-Bus worker threads, so it must be `Send`.
///
/// [08-settings.md]: ../../../docs/design/08-settings.md
pub trait GroupProvider: Send {
    /// The provider's current group list. `Ok(None)` means the provider has
    /// gone away; `Err` means it is present but could not be read.
    fn status(&mut self) -> Result<Option<Vec<GroupData>>, AdapterError>;

    /// Create a group. One explicit request.
    fn create_group(&mut self, _name: &str) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Delete a group. One explicit request.
    fn delete_group(&mut self, _name: &str) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Replace a group's membership. One explicit request.
    fn set_members(&mut self, _name: &str, _members: &[String]) -> AccountOutcome {
        AccountOutcome::Absent
    }
}

/// The live host-stack source.
pub struct HostAccounts {
    provider: Option<Box<dyn GroupProvider>>,
}

impl Default for HostAccounts {
    fn default() -> Self {
        HostAccounts::new()
    }
}

impl HostAccounts {
    /// A source reading AccountsService with no group provider attached.
    pub fn new() -> Self {
        HostAccounts { provider: None }
    }

    /// Attach the distribution group provider.
    pub fn with_provider(mut self, provider: Box<dyn GroupProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Whether a group provider is attached.
    pub fn has_group_provider(&self) -> bool {
        self.provider.is_some()
    }

    fn groups(&mut self) -> Result<Option<Vec<GroupData>>, AdapterError> {
        match self.provider.as_mut() {
            Some(provider) => provider.status(),
            None => Ok(None),
        }
    }

    fn group_write(
        &mut self,
        call: impl FnOnce(&mut dyn GroupProvider) -> AccountOutcome,
    ) -> AccountOutcome {
        match self.provider.as_mut() {
            Some(provider) => call(provider.as_mut()),
            None => AccountOutcome::Absent,
        }
    }
}

impl AccountSource for HostAccounts {
    fn read(&mut self) -> Result<Option<AccountsData>, AdapterError> {
        let users = read_users()?;
        let groups = self.groups()?;
        if users.is_none() && groups.is_none() {
            return Ok(None);
        }
        Ok(Some(AccountsData {
            users: users.unwrap_or_default(),
            groups,
        }))
    }

    fn create_user(
        &mut self,
        user_name: &str,
        real_name: &str,
        account_type: AccountType,
    ) -> AccountOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        match connection.call_method(
            Some(ACCOUNTS_SERVICE),
            ACCOUNTS_ROOT,
            Some(ACCOUNTS_INTERFACE),
            "CreateUser",
            &(user_name, real_name, account_type.code()),
        ) {
            Ok(_) => AccountOutcome::Applied,
            Err(error) => classify(error),
        }
    }

    fn delete_user(&mut self, uid: u64) -> AccountOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        match connection.call_method(
            Some(ACCOUNTS_SERVICE),
            ACCOUNTS_ROOT,
            Some(ACCOUNTS_INTERFACE),
            "DeleteUser",
            &(uid as i64),
        ) {
            Ok(_) => AccountOutcome::Applied,
            Err(error) => classify(error),
        }
    }

    fn set_account_type(&mut self, uid: u64, account_type: AccountType) -> AccountOutcome {
        call_user_method(uid, "SetAccountType", &(account_type.code(),))
    }

    fn set_locked(&mut self, uid: u64, locked: bool) -> AccountOutcome {
        call_user_method(uid, "SetLocked", &(locked,))
    }

    fn set_automatic_login(&mut self, uid: u64, automatic_login: bool) -> AccountOutcome {
        call_user_method(uid, "SetAutomaticLogin", &(automatic_login,))
    }

    fn create_group(&mut self, name: &str) -> AccountOutcome {
        self.group_write(|provider| provider.create_group(name))
    }

    fn delete_group(&mut self, name: &str) -> AccountOutcome {
        self.group_write(|provider| provider.delete_group(name))
    }

    fn set_group_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        self.group_write(|provider| provider.set_members(name, members))
    }
}

/// The AccountsService user path for `uid`.
pub fn user_path(uid: u64) -> String {
    format!("{ACCOUNTS_ROOT}/User{uid}")
}

/// Read every cached user AccountsService reports. `Ok(None)` means the
/// daemon is absent.
fn read_users() -> Result<Option<Vec<AccountData>>, AdapterError> {
    // No system bus means no AccountsService can be reached: absence, not an
    // error, and never a startup blocker.
    let connection = match Connection::system() {
        Ok(connection) => connection,
        Err(_) => return Ok(None),
    };
    if !name_has_owner(&connection, ACCOUNTS_SERVICE).map_err(failed)? {
        return Ok(None);
    }

    let mut users = Vec::new();
    for path in list_cached_users(&connection).map_err(failed)? {
        let props = user_properties(&connection, path.as_str()).map_err(failed)?;
        users.push(account_from_props(&props));
    }
    // Deterministic order regardless of the daemon's iteration order.
    users.sort_by(|left, right| {
        left.uid
            .cmp(&right.uid)
            .then_with(|| left.user_name.cmp(&right.user_name))
    });
    Ok(Some(users))
}

/// The user object paths AccountsService caches.
fn list_cached_users(connection: &Connection) -> zbus::Result<Vec<OwnedObjectPath>> {
    let reply = connection.call_method(
        Some(ACCOUNTS_SERVICE),
        ACCOUNTS_ROOT,
        Some(ACCOUNTS_INTERFACE),
        "ListCachedUsers",
        &(),
    )?;
    reply.body().deserialize::<Vec<OwnedObjectPath>>()
}

/// One user's `org.freedesktop.Accounts.User` properties.
fn user_properties(
    connection: &Connection,
    path: &str,
) -> zbus::Result<HashMap<String, OwnedValue>> {
    let reply = connection.call_method(
        Some(ACCOUNTS_SERVICE),
        path,
        Some(PROPERTIES_INTERFACE),
        "GetAll",
        &(USER_INTERFACE,),
    )?;
    reply.body().deserialize::<HashMap<String, OwnedValue>>()
}

/// Build one account from its property map. Pure, so a fixture map drives it
/// in tests.
pub fn account_from_props(props: &HashMap<String, OwnedValue>) -> AccountData {
    AccountData {
        uid: u64_prop(props, "Uid").unwrap_or_default(),
        user_name: string_prop(props, "UserName"),
        real_name: string_prop(props, "RealName"),
        account_type: AccountType::from_code(i32_prop(props, "AccountType").unwrap_or_default()),
        password_mode: PasswordMode::from_code(i32_prop(props, "PasswordMode").unwrap_or_default()),
        home_directory: string_prop(props, "HomeDirectory"),
        shell: string_prop(props, "Shell"),
        email: string_prop(props, "Email"),
        language: string_prop(props, "Language"),
        icon_file: string_prop(props, "IconFile"),
        locked: bool_prop(props, "Locked"),
        system_account: bool_prop(props, "SystemAccount"),
        automatic_login: bool_prop(props, "AutomaticLogin"),
        login_time: i64_prop(props, "LoginTime").unwrap_or_default(),
        x_session: string_prop(props, "XSession"),
    }
}

/// Call one method on the user object for `uid`.
fn call_user_method<B>(uid: u64, method: &str, body: &B) -> AccountOutcome
where
    B: serde::Serialize + zbus::zvariant::DynamicType,
{
    let connection = match daemon_connection() {
        Ok(connection) => connection,
        Err(outcome) => return outcome,
    };
    let path = user_path(uid);
    match connection.call_method(
        Some(ACCOUNTS_SERVICE),
        path.as_str(),
        Some(USER_INTERFACE),
        method,
        body,
    ) {
        Ok(_) => AccountOutcome::Applied,
        Err(error) => classify(error),
    }
}

/// A connection to the system bus, or the outcome that stands in for it.
///
/// No bus, or a bus with no `org.freedesktop.Accounts` owner, is absence —
/// never an error.
fn daemon_connection() -> Result<Connection, AccountOutcome> {
    let connection = Connection::system().map_err(|_| AccountOutcome::Absent)?;
    match name_has_owner(&connection, ACCOUNTS_SERVICE) {
        Ok(true) => Ok(connection),
        Ok(false) | Err(_) => Err(AccountOutcome::Absent),
    }
}

/// Whether `service` currently owns its name.
fn name_has_owner(connection: &Connection, service: &str) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(service,),
    )?;
    reply.body().deserialize::<bool>()
}

/// Map a D-Bus error from a write to the adapter outcome.
///
/// A polkit refusal is a denial the pane surfaces; everything else is a plain
/// failure.
fn classify(error: zbus::Error) -> AccountOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        AccountOutcome::Denied(format!("AccountsService: {error}"))
    } else {
        AccountOutcome::Failed(AdapterError::new(format!("AccountsService: {error}")))
    }
}

/// Whether an error name or message reports an authorization refusal.
fn is_permission_denied(name: &str, message: &str) -> bool {
    const NAMES: [&str; 4] = [
        "org.freedesktop.Accounts.Error.PermissionDenied",
        "org.freedesktop.Accounts.Error.NotAuthorized",
        "org.freedesktop.DBus.Error.AccessDenied",
        "org.freedesktop.PolicyKit1.Error.NotAuthorized",
    ];
    if NAMES.contains(&name) {
        return true;
    }
    let lower = message.to_lowercase();
    lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("polkit")
}

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("AccountsService: {error}"))
}

/// A string property, `""` when AccountsService did not report it.
fn string_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<String>().ok())
        .unwrap_or_default()
}

/// A boolean property, `false` when AccountsService did not report it.
fn bool_prop(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or_default()
}

/// A `u64` property, `None` when AccountsService did not report it. Tolerates
/// a `u32` read for daemons that report `Uid` narrower than the spec's `t`.
fn u64_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Option<u64> {
    let value = props.get(key)?;
    value
        .downcast_ref::<u64>()
        .ok()
        .or_else(|| value.downcast_ref::<u32>().ok().map(u64::from))
}

/// An `i64` property, `None` when AccountsService did not report it.
fn i64_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    let value = props.get(key)?;
    value
        .downcast_ref::<i64>()
        .ok()
        .or_else(|| value.downcast_ref::<i32>().ok().map(i64::from))
}

/// An `i32` property, `None` when AccountsService did not report it.
fn i32_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i32> {
    let value = props.get(key)?;
    value
        .downcast_ref::<i32>()
        .ok()
        .or_else(|| value.downcast_ref::<u32>().ok().map(|value| value as i32))
}

/// A fixture-backed distribution group provider.
///
/// The mock is the CI path when a `HostAccounts` test attaches a provider: it
/// serves a [`GroupData`] list with no distro tooling on the machine, and
/// `kill`/`restart` exercise the provider going away and coming back.
#[derive(Debug, Clone, PartialEq)]
pub struct MockGroupProvider {
    present: bool,
    groups: Vec<GroupData>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    statuses: u32,
    creates: u32,
    deletes: u32,
    member_sets: u32,
}

impl MockGroupProvider {
    /// A provider reporting `groups`.
    pub fn present(groups: Vec<GroupData>) -> Self {
        MockGroupProvider {
            present: true,
            groups,
            failure: None,
            write_failure: None,
            statuses: 0,
            creates: 0,
            deletes: 0,
            member_sets: 0,
        }
    }

    /// A provider that is present but fails every status read.
    pub fn failing(message: impl Into<String>) -> Self {
        MockGroupProvider {
            present: true,
            groups: vec![],
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            statuses: 0,
            creates: 0,
            deletes: 0,
            member_sets: 0,
        }
    }

    /// A provider whose writes fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// Publish fresh group state.
    pub fn push(&mut self, groups: Vec<GroupData>) {
        self.present = true;
        self.failure = None;
        self.groups = groups;
    }

    /// The provider goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The provider comes back.
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// The provider's current groups.
    pub fn groups(&self) -> &[GroupData] {
        &self.groups
    }

    /// How many status reads the provider served.
    pub fn statuses(&self) -> u32 {
        self.statuses
    }

    /// How many groups the provider created.
    pub fn creates(&self) -> u32 {
        self.creates
    }

    /// How many groups the provider deleted.
    pub fn deletes(&self) -> u32 {
        self.deletes
    }

    /// How many membership writes the provider accepted.
    pub fn member_sets(&self) -> u32 {
        self.member_sets
    }

    fn outcome(&self) -> AccountOutcome {
        if !self.present {
            return AccountOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return AccountOutcome::Failed(error.clone());
        }
        AccountOutcome::Applied
    }
}

impl GroupProvider for MockGroupProvider {
    fn status(&mut self) -> Result<Option<Vec<GroupData>>, AdapterError> {
        self.statuses += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.groups.clone()))
    }

    fn create_group(&mut self, name: &str) -> AccountOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.creates += 1;
            self.groups.push(GroupData {
                name: name.to_owned(),
                ..GroupData::default()
            });
        }
        outcome
    }

    fn delete_group(&mut self, name: &str) -> AccountOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.deletes += 1;
            self.groups.retain(|group| group.name != name);
        }
        outcome
    }

    fn set_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        let outcome = self.outcome();
        if outcome.is_applied() {
            self.member_sets += 1;
            if let Some(group) = self.groups.iter_mut().find(|group| group.name == name) {
                group.members = members.to_vec();
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Value;

    fn string(value: &str) -> OwnedValue {
        OwnedValue::try_from(Value::from(value)).unwrap()
    }

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let source = HostAccounts::new();
        assert!(!source.has_group_provider());
    }

    #[test]
    fn the_service_names_are_the_accountsservice_ones() {
        assert_eq!(ACCOUNTS_SERVICE, "org.freedesktop.Accounts");
        assert_eq!(ACCOUNTS_ROOT, "/org/freedesktop/Accounts");
        assert_eq!(ACCOUNTS_INTERFACE, "org.freedesktop.Accounts");
        assert_eq!(USER_INTERFACE, "org.freedesktop.Accounts.User");
        assert_eq!(user_path(1000), "/org/freedesktop/Accounts/User1000");
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("AccountsService: "));
    }

    #[test]
    fn polkit_names_and_messages_classify_as_denied() {
        assert!(is_permission_denied(
            "org.freedesktop.Accounts.Error.PermissionDenied",
            "whatever"
        ));
        assert!(is_permission_denied(
            "org.freedesktop.DBus.Error.AccessDenied",
            "whatever"
        ));
        assert!(is_permission_denied("", "polkit refused the request"));
        assert!(!is_permission_denied(
            "org.freedesktop.Accounts.Error.UserExists",
            "the user already exists"
        ));
    }

    #[test]
    fn a_polkit_error_becomes_a_denial_outcome() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        match classify(error) {
            AccountOutcome::Denied(note) => assert!(note.starts_with("AccountsService: ")),
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_non_authorization_error_is_a_failure() {
        let error = zbus::Error::Failure("the user already exists".to_owned());
        assert_eq!(
            classify(error),
            AccountOutcome::Failed(AdapterError::new(
                "AccountsService: the user already exists"
            ))
        );
    }

    #[test]
    fn a_property_map_decodes_to_an_account() {
        let mut props: HashMap<String, OwnedValue> = HashMap::new();
        props.insert("Uid".to_owned(), OwnedValue::from(1000u64));
        props.insert("UserName".to_owned(), string("dan"));
        props.insert("RealName".to_owned(), string("Dan Doe"));
        props.insert("AccountType".to_owned(), OwnedValue::from(1i32));
        props.insert("PasswordMode".to_owned(), OwnedValue::from(0i32));
        props.insert("HomeDirectory".to_owned(), string("/home/dan"));
        props.insert("Shell".to_owned(), string("/bin/bash"));
        props.insert("Email".to_owned(), string("dan@example.com"));
        props.insert(
            "IconFile".to_owned(),
            string("/usr/share/pixmaps/faces/dan.png"),
        );
        props.insert("Locked".to_owned(), OwnedValue::from(false));
        props.insert("SystemAccount".to_owned(), OwnedValue::from(false));
        props.insert("AutomaticLogin".to_owned(), OwnedValue::from(true));
        props.insert("LoginTime".to_owned(), OwnedValue::from(1_700_000_000i64));
        props.insert("XSession".to_owned(), string("dragonfruit"));

        let account = account_from_props(&props);
        assert_eq!(account.uid, 1000);
        assert_eq!(account.user_name, "dan");
        assert_eq!(account.real_name, "Dan Doe");
        assert_eq!(account.account_type, AccountType::Administrator);
        assert_eq!(account.password_mode, PasswordMode::Regular);
        assert_eq!(account.home_directory, "/home/dan");
        assert_eq!(account.shell, "/bin/bash");
        assert_eq!(account.email, "dan@example.com");
        assert_eq!(account.icon_file, "/usr/share/pixmaps/faces/dan.png");
        assert!(account.automatic_login);
        assert!(!account.system_account);
        assert_eq!(account.login_time, 1_700_000_000);
        assert_eq!(account.x_session, "dragonfruit");
    }

    #[test]
    fn a_sparse_property_map_decodes_to_defaults() {
        let account = account_from_props(&HashMap::new());
        assert_eq!(account.uid, 0);
        assert_eq!(account.account_type, AccountType::Standard);
        assert_eq!(account.password_mode, PasswordMode::Regular);
        assert!(account.user_name.is_empty());
        assert!(!account.locked);
    }

    #[test]
    fn the_group_provider_serves_and_writes() {
        let mut provider = MockGroupProvider::present(vec![GroupData {
            name: "wheel".to_owned(),
            gid: 10,
            members: vec!["dan".to_owned()],
            system: false,
        }]);
        assert_eq!(provider.status().unwrap().unwrap().len(), 1);
        assert_eq!(provider.statuses(), 1);

        assert!(provider.create_group("devs").is_applied());
        assert_eq!(provider.creates(), 1);
        assert_eq!(provider.groups().len(), 2);

        assert!(provider
            .set_members("devs", &["dan".to_owned(), "kim".to_owned()])
            .is_applied());
        assert_eq!(provider.member_sets(), 1);
        assert_eq!(provider.groups()[1].members.len(), 2);

        assert!(provider.delete_group("devs").is_applied());
        assert_eq!(provider.deletes(), 1);
        assert_eq!(provider.groups().len(), 1);
    }

    #[test]
    fn a_killed_group_provider_is_absent() {
        let mut provider = MockGroupProvider::present(vec![]);
        provider.kill();
        assert_eq!(provider.status().unwrap(), None);
        assert_eq!(provider.create_group("devs"), AccountOutcome::Absent);
    }

    #[test]
    fn a_failing_group_provider_errors() {
        let mut provider = MockGroupProvider::failing("groups: timeout");
        assert_eq!(provider.status().unwrap_err().message(), "groups: timeout");
    }

    #[test]
    fn a_group_provider_read_drives_host_accounts() {
        // A provider makes the read answer even if AccountsService is absent
        // on the machine (layered absence): the group list stays live.
        let mut host =
            HostAccounts::new().with_provider(Box::new(MockGroupProvider::present(vec![
                GroupData {
                    name: "wheel".to_owned(),
                    ..GroupData::default()
                },
            ])));
        let data = host.read().unwrap().unwrap();
        assert!(data.groups_available());
        assert_eq!(data.groups.as_ref().unwrap()[0].name, "wheel");
    }
}
