// SPDX-License-Identifier: MIT
//! The transport seam: the raw AccountsService read, the group-provider seam,
//! and their mocks.
//!
//! An [`AccountSource`] is the only thing that talks to the host stack. That
//! stack has two halves:
//!
//! * the **users** half — the cached user list AccountsService serves over
//!   `org.freedesktop.Accounts` on the system bus ([`crate::HostAccounts`]);
//!   and
//! * the **groups** half — the distribution provider behind a
//!   [`crate::GroupProvider`] seam, because AccountsService has no group API.
//!
//! This crate reuses both; it never reimplements account or group management.
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the host stack answered; `data` is the live read.
//! * `Ok(None)` — the host stack is absent. A normal state; the item hides.
//! * `Err(error)` — the host stack is present but could not be read; the item
//!   shows visible and inert with the message.
//!
//! Absence is layered, as the battery and update adapters' is. AccountsService
//! answering but reporting no users is a live, empty snapshot; a running host
//! with **no group provider** is carried inside the snapshot
//! (`AccountsData::groups` is `None`) and only disables the group controls.
//! The whole adapter is `Unavailable` only when neither half is reachable — a
//! normal hidden state, never an error.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The role a user account carries.
///
/// AccountsService's own vocabulary (`AccountType` `i`), narrowed to the two
/// roles the pane draws a badge for. An unrecognized code is
/// [`AccountType::Standard`], never a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountType {
    /// A standard, unprivileged account (AccountsService code `0`).
    #[default]
    Standard,
    /// An administrator account (AccountsService code `1`).
    Administrator,
}

impl AccountType {
    /// Every account type.
    pub const ALL: [AccountType; 2] = [AccountType::Standard, AccountType::Administrator];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            AccountType::Standard => "standard",
            AccountType::Administrator => "administrator",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            AccountType::Standard => "Standard",
            AccountType::Administrator => "Admin",
        }
    }

    /// The AccountsService integer code.
    pub const fn code(self) -> i32 {
        match self {
            AccountType::Standard => 0,
            AccountType::Administrator => 1,
        }
    }

    /// The account type for an AccountsService code; an unknown code is
    /// `Standard`.
    pub const fn from_code(code: i32) -> AccountType {
        match code {
            1 => AccountType::Administrator,
            _ => AccountType::Standard,
        }
    }

    /// Parse the stable id back to an account type; an unknown id is
    /// `Standard`.
    pub fn from_id(id: &str) -> AccountType {
        AccountType::ALL
            .into_iter()
            .find(|account_type| account_type.id() == id)
            .unwrap_or(AccountType::Standard)
    }
}

/// How a user logs in.
///
/// AccountsService's `PasswordMode` (`i`). An unrecognized code is
/// [`PasswordMode::Regular`], never a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PasswordMode {
    /// A password is required (AccountsService code `0`).
    #[default]
    Regular,
    /// No password; the account logs in without one (code `1`).
    None,
    /// The password is chosen at first login (code `2`).
    SetAtLogin,
    /// An empty password (code `3`).
    Empty,
}

impl PasswordMode {
    /// Every password mode.
    pub const ALL: [PasswordMode; 4] = [
        PasswordMode::Regular,
        PasswordMode::None,
        PasswordMode::SetAtLogin,
        PasswordMode::Empty,
    ];

    /// The stable id used by the wire and the pane.
    pub const fn id(self) -> &'static str {
        match self {
            PasswordMode::Regular => "regular",
            PasswordMode::None => "none",
            PasswordMode::SetAtLogin => "set-at-login",
            PasswordMode::Empty => "empty",
        }
    }

    /// The short label the pane draws.
    pub const fn label(self) -> &'static str {
        match self {
            PasswordMode::Regular => "Password",
            PasswordMode::None => "No password",
            PasswordMode::SetAtLogin => "Set at login",
            PasswordMode::Empty => "Empty",
        }
    }

    /// The AccountsService integer code.
    pub const fn code(self) -> i32 {
        match self {
            PasswordMode::Regular => 0,
            PasswordMode::None => 1,
            PasswordMode::SetAtLogin => 2,
            PasswordMode::Empty => 3,
        }
    }

    /// The password mode for an AccountsService code; an unknown code is
    /// `Regular`.
    pub const fn from_code(code: i32) -> PasswordMode {
        match code {
            1 => PasswordMode::None,
            2 => PasswordMode::SetAtLogin,
            3 => PasswordMode::Empty,
            _ => PasswordMode::Regular,
        }
    }

    /// Parse the stable id back to a mode; an unknown id is `Regular`.
    pub fn from_id(id: &str) -> PasswordMode {
        PasswordMode::ALL
            .into_iter()
            .find(|mode| mode.id() == id)
            .unwrap_or(PasswordMode::Regular)
    }
}

/// One user account from AccountsService.
///
/// Every field is a plain read of an `org.freedesktop.Accounts.User` property;
/// an unknown string is empty and an unknown flag is `false`, never an
/// invented value. The live reads live in [`crate::HostAccounts`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AccountData {
    /// `Uid`: the numeric user id.
    #[serde(default)]
    pub uid: u64,
    /// `UserName`: the login name.
    #[serde(default)]
    pub user_name: String,
    /// `RealName`: the display name.
    #[serde(default)]
    pub real_name: String,
    /// `AccountType`: standard or administrator.
    #[serde(default)]
    pub account_type: AccountType,
    /// `PasswordMode`: how the account logs in.
    #[serde(default)]
    pub password_mode: PasswordMode,
    /// `HomeDirectory`.
    #[serde(default)]
    pub home_directory: String,
    /// `Shell`.
    #[serde(default)]
    pub shell: String,
    /// `Email`.
    #[serde(default)]
    pub email: String,
    /// `Language`.
    #[serde(default)]
    pub language: String,
    /// `IconFile`: the account avatar path, when one is set.
    #[serde(default)]
    pub icon_file: String,
    /// `Locked`: the account is disabled.
    #[serde(default)]
    pub locked: bool,
    /// `SystemAccount`: a system/daemon account, not a human login.
    #[serde(default)]
    pub system_account: bool,
    /// `AutomaticLogin`: the account logs in without a prompt.
    #[serde(default)]
    pub automatic_login: bool,
    /// `LoginTime`: seconds since the Unix epoch of the last login (`0` when
    /// never).
    #[serde(default)]
    pub login_time: i64,
    /// `XSession`: the preferred session entry.
    #[serde(default)]
    pub x_session: String,
}

/// One group from the distribution's group provider.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct GroupData {
    /// The group name.
    #[serde(default)]
    pub name: String,
    /// The numeric group id.
    #[serde(default)]
    pub gid: u32,
    /// The member login names.
    #[serde(default)]
    pub members: Vec<String>,
    /// A system group, not a user-created one.
    #[serde(default)]
    pub system: bool,
}

/// The raw result of one host-stack read: the AccountsService user list, plus
/// the group provider's list when the provider is present.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AccountsData {
    /// Every user AccountsService caches.
    #[serde(default)]
    pub users: Vec<AccountData>,
    /// The group list; `None` when the distro group provider is absent (only
    /// the group controls disable).
    #[serde(default)]
    pub groups: Option<Vec<GroupData>>,
}

impl AccountsData {
    /// Whether the distribution group provider is present.
    pub const fn groups_available(&self) -> bool {
        self.groups.is_some()
    }
}

/// The result of one account or group write.
///
/// These are explicit user actions, never a poll. A write that lands invents
/// no snapshot: AccountsService (or the group provider) publishes the
/// resulting state and the host re-reads, so the snapshot stays the single
/// source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOutcome {
    /// The daemon or provider accepted the request.
    Applied,
    /// polkit refused the request. `note` records the daemon's message.
    Denied(String),
    /// There is no AccountsService (or no such account) or no group provider.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

impl AccountOutcome {
    /// Whether the daemon or provider accepted the request.
    pub fn is_applied(&self) -> bool {
        matches!(self, AccountOutcome::Applied)
    }

    /// The recorded denial note, when polkit refused.
    pub fn denial_note(&self) -> Option<&str> {
        match self {
            AccountOutcome::Denied(note) => Some(note),
            _ => None,
        }
    }

    /// The failure, when the request failed for a reason other than a denial.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            AccountOutcome::Failed(error) => Some(error),
            _ => None,
        }
    }
}

/// Reads the host stack over some transport and drives its write methods.
pub trait AccountSource {
    /// One read of the host stack.
    fn read(&mut self) -> Result<Option<AccountsData>, AdapterError>;

    /// Create a user account. One explicit write.
    fn create_user(
        &mut self,
        _user_name: &str,
        _real_name: &str,
        _account_type: AccountType,
    ) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Delete the user account with `uid`. One explicit write.
    fn delete_user(&mut self, _uid: u64) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Set the account type of the user with `uid`. One explicit write.
    fn set_account_type(&mut self, _uid: u64, _account_type: AccountType) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Lock or unlock the user with `uid`. One explicit write.
    fn set_locked(&mut self, _uid: u64, _locked: bool) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Set or clear automatic login for the user with `uid`. One explicit
    /// write.
    fn set_automatic_login(&mut self, _uid: u64, _automatic_login: bool) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Create a group. One explicit write (the distro provider's).
    fn create_group(&mut self, _name: &str) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Delete a group. One explicit write (the distro provider's).
    fn delete_group(&mut self, _name: &str) -> AccountOutcome {
        AccountOutcome::Absent
    }

    /// Replace a group's membership. One explicit write (the distro
    /// provider's).
    fn set_group_members(&mut self, _name: &str, _members: &[String]) -> AccountOutcome {
        AccountOutcome::Absent
    }
}

/// How the simulated daemon answers write requests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// polkit denies the request; carries the recorded note.
    Deny(String),
    /// The request fails for another reason.
    Fail(AdapterError),
}

/// The write counters the mock keeps, one per method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct WriteCounts {
    creates: u32,
    deletes: u32,
    type_sets: u32,
    lock_sets: u32,
    auto_login_sets: u32,
    group_creates: u32,
    group_deletes: u32,
    group_member_sets: u32,
}

impl WriteCounts {
    fn total(&self) -> u32 {
        self.creates
            + self.deletes
            + self.type_sets
            + self.lock_sets
            + self.auto_login_sets
            + self.group_creates
            + self.group_deletes
            + self.group_member_sets
    }
}

/// A fixture-backed source with a simulated host-stack lifecycle.
///
/// The mock is the CI path: it serves [`AccountsData`] with no AccountsService
/// on the bus, and `kill`/`restart` exercise absence and re-subscribe the way
/// masking the daemon would. `push` drives the user and group lists so a test
/// observes the change stream. Its writes mutate the simulated stack the way
/// AccountsService and the group provider would, so a subsequent `read` sees
/// the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockAccounts {
    present: bool,
    data: Option<AccountsData>,
    failure: Option<AdapterError>,
    behavior: WriteBehavior,
    reads: u32,
    writes: WriteCounts,
}

impl MockAccounts {
    /// The host stack is not reachable.
    pub fn absent() -> Self {
        MockAccounts {
            present: false,
            data: None,
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A host stack that answers with `data`.
    pub fn present(data: AccountsData) -> Self {
        MockAccounts {
            present: true,
            data: Some(data),
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// A host stack that fails every read (e.g. the daemon went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockAccounts {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            behavior: WriteBehavior::Accept,
            reads: 0,
            writes: WriteCounts::default(),
        }
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
        self
    }

    /// Make every write come back as a polkit denial with `note`.
    pub fn deny_writes(mut self, note: impl Into<String>) -> Self {
        self.behavior = WriteBehavior::Deny(note.into());
        self
    }

    /// The host stack publishes fresh state.
    pub fn push(&mut self, data: AccountsData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is reachable.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many accounts the source has created.
    pub fn creates(&self) -> u32 {
        self.writes.creates
    }

    /// How many accounts the source has deleted.
    pub fn deletes(&self) -> u32 {
        self.writes.deletes
    }

    /// How many account-type writes the source has accepted.
    pub fn type_sets(&self) -> u32 {
        self.writes.type_sets
    }

    /// How many lock writes the source has accepted.
    pub fn lock_sets(&self) -> u32 {
        self.writes.lock_sets
    }

    /// How many automatic-login writes the source has accepted.
    pub fn auto_login_sets(&self) -> u32 {
        self.writes.auto_login_sets
    }

    /// How many groups the source has created.
    pub fn group_creates(&self) -> u32 {
        self.writes.group_creates
    }

    /// How many groups the source has deleted.
    pub fn group_deletes(&self) -> u32 {
        self.writes.group_deletes
    }

    /// How many group-membership writes the source has accepted.
    pub fn group_member_sets(&self) -> u32 {
        self.writes.group_member_sets
    }

    /// How many write requests the source has served in total. Lets a test
    /// prove a write is one explicit call, never a loop.
    pub fn writes(&self) -> u32 {
        self.writes.total()
    }

    fn read_outcome(&self) -> Result<(), AdapterError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        Ok(())
    }

    fn write_outcome(&mut self) -> AccountOutcome {
        if !self.present {
            return AccountOutcome::Absent;
        }
        // A stack that cannot answer a read cannot answer a write either.
        if let Some(error) = &self.failure {
            return AccountOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => AccountOutcome::Applied,
            WriteBehavior::Deny(note) => AccountOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => AccountOutcome::Failed(error.clone()),
        }
    }

    fn with_data_mut(&mut self, edit: impl FnOnce(&mut AccountsData)) {
        if let Some(data) = self.data.as_mut() {
            edit(data);
        }
    }
}

impl AccountSource for MockAccounts {
    fn read(&mut self) -> Result<Option<AccountsData>, AdapterError> {
        self.reads += 1;
        self.read_outcome()?;
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn create_user(
        &mut self,
        user_name: &str,
        real_name: &str,
        account_type: AccountType,
    ) -> AccountOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.creates += 1;
            let next_uid = self
                .data
                .as_ref()
                .map(|data| {
                    data.users
                        .iter()
                        .map(|user| user.uid)
                        .max()
                        .unwrap_or(1000)
                        .saturating_add(1)
                })
                .unwrap_or(1000);
            self.with_data_mut(|data| {
                data.users.push(AccountData {
                    uid: next_uid,
                    user_name: user_name.to_owned(),
                    real_name: real_name.to_owned(),
                    account_type,
                    ..AccountData::default()
                });
            });
        }
        outcome
    }

    fn delete_user(&mut self, uid: u64) -> AccountOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.deletes += 1;
            self.with_data_mut(|data| data.users.retain(|user| user.uid != uid));
        }
        outcome
    }

    fn set_account_type(&mut self, uid: u64, account_type: AccountType) -> AccountOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.type_sets += 1;
            self.with_data_mut(|data| {
                if let Some(user) = data.users.iter_mut().find(|user| user.uid == uid) {
                    user.account_type = account_type;
                }
            });
        }
        outcome
    }

    fn set_locked(&mut self, uid: u64, locked: bool) -> AccountOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.lock_sets += 1;
            self.with_data_mut(|data| {
                if let Some(user) = data.users.iter_mut().find(|user| user.uid == uid) {
                    user.locked = locked;
                }
            });
        }
        outcome
    }

    fn set_automatic_login(&mut self, uid: u64, automatic_login: bool) -> AccountOutcome {
        let outcome = self.write_outcome();
        if outcome.is_applied() {
            self.writes.auto_login_sets += 1;
            self.with_data_mut(|data| {
                for user in &mut data.users {
                    if user.uid == uid {
                        user.automatic_login = automatic_login;
                    } else if automatic_login {
                        // AccountsService keeps one automatic login user.
                        user.automatic_login = false;
                    }
                }
            });
        }
        outcome
    }

    fn create_group(&mut self, name: &str) -> AccountOutcome {
        let outcome = self.group_outcome();
        if outcome.is_applied() {
            self.writes.group_creates += 1;
            self.with_data_mut(|data| {
                if let Some(groups) = data.groups.as_mut() {
                    groups.push(GroupData {
                        name: name.to_owned(),
                        ..GroupData::default()
                    });
                }
            });
        }
        outcome
    }

    fn delete_group(&mut self, name: &str) -> AccountOutcome {
        let outcome = self.group_outcome();
        if outcome.is_applied() {
            self.writes.group_deletes += 1;
            self.with_data_mut(|data| {
                if let Some(groups) = data.groups.as_mut() {
                    groups.retain(|group| group.name != name);
                }
            });
        }
        outcome
    }

    fn set_group_members(&mut self, name: &str, members: &[String]) -> AccountOutcome {
        let outcome = self.group_outcome();
        if outcome.is_applied() {
            self.writes.group_member_sets += 1;
            self.with_data_mut(|data| {
                if let Some(groups) = data.groups.as_mut() {
                    if let Some(group) = groups.iter_mut().find(|group| group.name == name) {
                        group.members = members.to_vec();
                    }
                }
            });
        }
        outcome
    }
}

impl MockAccounts {
    /// The write outcome when a group provider must be present: a group write
    /// against a stack with no group list (`groups: None`) is absence, layered
    /// the way `AccountsData::groups` is.
    fn group_outcome(&mut self) -> AccountOutcome {
        if self.present
            && self.failure.is_none()
            && matches!(self.behavior, WriteBehavior::Accept)
            && !self
                .data
                .as_ref()
                .is_some_and(AccountsData::groups_available)
        {
            return AccountOutcome::Absent;
        }
        self.write_outcome()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> AccountsData {
        AccountsData {
            users: vec![AccountData {
                uid: 1000,
                user_name: "dan".to_owned(),
                real_name: "Dan Doe".to_owned(),
                account_type: AccountType::Administrator,
                ..AccountData::default()
            }],
            groups: Some(vec![GroupData {
                name: "wheel".to_owned(),
                gid: 10,
                members: vec!["dan".to_owned()],
                system: false,
            }]),
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockAccounts::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockAccounts::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockAccounts::failing("AccountsService: timeout");
        assert_eq!(
            mock.read().unwrap_err().message(),
            "AccountsService: timeout"
        );
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockAccounts::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn pushes_replace_the_served_data() {
        let mut mock = MockAccounts::present(data());
        let mut next = data();
        next.users[0].real_name = "Renamed".to_owned();
        mock.push(next);
        assert_eq!(mock.read().unwrap().unwrap().users[0].real_name, "Renamed");
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockAccounts::present(data()).deny_writes("AccountsService: not authorized");
        let outcome = mock.delete_user(1000);
        assert_eq!(
            outcome,
            AccountOutcome::Denied("AccountsService: not authorized".to_owned())
        );
        assert_eq!(
            outcome.denial_note(),
            Some("AccountsService: not authorized")
        );
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockAccounts::present(data()).fail_writes("AccountsService: busy");
        assert_eq!(
            mock.set_locked(1000, true),
            AccountOutcome::Failed(AdapterError::new("AccountsService: busy"))
        );
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut mock = MockAccounts::absent();
        assert_eq!(
            mock.create_user("x", "X", AccountType::Standard),
            AccountOutcome::Absent
        );
        assert_eq!(mock.delete_user(1), AccountOutcome::Absent);
        assert_eq!(mock.create_group("g"), AccountOutcome::Absent);
        assert_eq!(mock.writes(), 0);
    }

    #[test]
    fn a_group_write_without_a_provider_is_absent() {
        let mut mock = MockAccounts::present(AccountsData {
            groups: None,
            ..data()
        });
        assert_eq!(mock.create_group("g"), AccountOutcome::Absent);
        assert_eq!(mock.group_creates(), 0);
        // A user write is unaffected: absence is layered.
        assert_eq!(mock.delete_user(1000), AccountOutcome::Applied);
        assert_eq!(mock.deletes(), 1);
    }

    #[test]
    fn account_type_codes_and_ids_round_trip() {
        for account_type in AccountType::ALL {
            assert_eq!(AccountType::from_id(account_type.id()), account_type);
            assert_eq!(AccountType::from_code(account_type.code()), account_type);
        }
        assert_eq!(AccountType::from_id("bogus"), AccountType::Standard);
        assert_eq!(AccountType::from_code(99), AccountType::Standard);
        assert_eq!(AccountType::Administrator.label(), "Admin");
    }

    #[test]
    fn password_mode_codes_and_ids_round_trip() {
        for mode in PasswordMode::ALL {
            assert_eq!(PasswordMode::from_id(mode.id()), mode);
            assert_eq!(PasswordMode::from_code(mode.code()), mode);
        }
        assert_eq!(PasswordMode::from_id("bogus"), PasswordMode::Regular);
        assert_eq!(PasswordMode::from_code(99), PasswordMode::Regular);
        assert_eq!(PasswordMode::None.label(), "No password");
    }
}
