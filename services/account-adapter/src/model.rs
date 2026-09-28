// SPDX-License-Identifier: MIT
//! The Users and Groups snapshot the pane renders, decoded from one raw read.
//!
//! The model owns only the projection a consumer should not repeat: it types
//! each user and group, orders the lists (human users before system accounts,
//! groups by name), derives the labels and glyph the pane header draws, and
//! computes the change stream between two reads. It never creates or deletes
//! an account, never edits a group, and never writes a settings key —
//! AccountsService and the distro group provider own the operations and
//! `settingsd` owns the durable preferences; this is the data they publish.
//!
//! The event half is a pure diff: [`AccountsSnapshot::changes`] reports what
//! moved between two reads (a user added, removed, or edited; the automatic
//! login user; the group provider appearing; a group added, removed, or
//! edited) without any polling.

use crate::source::{AccountData, AccountType, AccountsData, GroupData, PasswordMode};

/// One user account the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Account {
    /// The numeric user id.
    pub uid: u64,
    /// The login name.
    pub user_name: String,
    /// The display name.
    pub real_name: String,
    /// Standard or administrator.
    pub account_type: AccountType,
    /// How the account logs in.
    pub password_mode: PasswordMode,
    /// The home directory.
    pub home_directory: String,
    /// The login shell.
    pub shell: String,
    /// The account email, when one is set.
    pub email: String,
    /// The account language, when one is set.
    pub language: String,
    /// The avatar path, when one is set.
    pub icon_file: String,
    /// The account is disabled.
    pub locked: bool,
    /// A system/daemon account, not a human login.
    pub system_account: bool,
    /// The account logs in without a prompt.
    pub automatic_login: bool,
    /// The last login, seconds since the Unix epoch (`0` when never).
    pub login_time: i64,
    /// The preferred session entry.
    pub x_session: String,
}

impl Account {
    fn from_data(data: &AccountData) -> Self {
        Account {
            uid: data.uid,
            user_name: data.user_name.clone(),
            real_name: data.real_name.clone(),
            account_type: data.account_type,
            password_mode: data.password_mode,
            home_directory: data.home_directory.clone(),
            shell: data.shell.clone(),
            email: data.email.clone(),
            language: data.language.clone(),
            icon_file: data.icon_file.clone(),
            locked: data.locked,
            system_account: data.system_account,
            automatic_login: data.automatic_login,
            login_time: data.login_time,
            x_session: data.x_session.clone(),
        }
    }

    /// The name to show: the real name, else the login name.
    pub fn display_name(&self) -> &str {
        if self.real_name.is_empty() {
            &self.user_name
        } else {
            &self.real_name
        }
    }

    /// The uppercase initial the fallback avatar draws, from the display name
    /// (else the login name); `?` when both are empty.
    pub fn initial(&self) -> String {
        let source = if self.real_name.is_empty() {
            &self.user_name
        } else {
            &self.real_name
        };
        source
            .chars()
            .find(|c| c.is_alphanumeric())
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_owned())
    }

    /// Whether the account is an administrator.
    pub fn is_admin(&self) -> bool {
        self.account_type == AccountType::Administrator
    }

    /// Whether the account is disabled.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Whether the account is a system/daemon account rather than a human
    /// login.
    pub fn is_system(&self) -> bool {
        self.system_account
    }

    /// Whether an avatar image is configured.
    pub fn has_avatar(&self) -> bool {
        !self.icon_file.is_empty()
    }
}

/// One group the pane draws.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Group {
    /// The group name.
    pub name: String,
    /// The numeric group id.
    pub gid: u32,
    /// The member login names.
    pub members: Vec<String>,
    /// A system group, not a user-created one.
    pub system: bool,
}

impl Group {
    fn from_data(data: &GroupData) -> Self {
        Group {
            name: data.name.clone(),
            gid: data.gid,
            members: data.members.clone(),
            system: data.system,
        }
    }

    /// The name to show.
    pub fn display_name(&self) -> &str {
        &self.name
    }

    /// How many members the group has.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Whether the group is a system group.
    pub fn is_system(&self) -> bool {
        self.system
    }

    /// Whether `user_name` is a member.
    pub fn contains(&self, user_name: &str) -> bool {
        self.members.iter().any(|member| member == user_name)
    }
}

/// The Users and Groups snapshot a pane renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountsSnapshot {
    /// Every user AccountsService reports, human users first, by uid.
    pub users: Vec<Account>,
    /// The group list; `None` when the distro group provider is absent.
    pub groups: Option<Vec<Group>>,
}

impl AccountsSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &AccountsData) -> Self {
        let mut users: Vec<Account> = data.users.iter().map(Account::from_data).collect();
        users.sort_by(|left, right| {
            left.is_system()
                .cmp(&right.is_system())
                .then_with(|| left.uid.cmp(&right.uid))
                .then_with(|| left.user_name.cmp(&right.user_name))
        });

        let groups = data.groups.as_ref().map(|groups| {
            let mut groups: Vec<Group> = groups.iter().map(Group::from_data).collect();
            groups.sort_by(|left, right| {
                left.is_system()
                    .cmp(&right.is_system())
                    .then_with(|| left.name.cmp(&right.name))
            });
            groups
        });

        AccountsSnapshot { users, groups }
    }

    /// Every user, human and system alike.
    pub fn users(&self) -> &[Account] {
        &self.users
    }

    /// The human users (not system/daemon accounts).
    pub fn human_users(&self) -> Vec<&Account> {
        self.users.iter().filter(|user| !user.is_system()).collect()
    }

    /// The system/daemon accounts.
    pub fn system_users(&self) -> Vec<&Account> {
        self.users.iter().filter(|user| user.is_system()).collect()
    }

    /// The group list; empty when the provider is absent.
    pub fn groups(&self) -> &[Group] {
        self.groups.as_deref().unwrap_or(&[])
    }

    /// Whether the distribution group provider is present.
    pub const fn groups_available(&self) -> bool {
        self.groups.is_some()
    }

    /// The user with `uid`, when the snapshot has one.
    pub fn user_by_uid(&self, uid: u64) -> Option<&Account> {
        self.users.iter().find(|user| user.uid == uid)
    }

    /// The user with `user_name`, when the snapshot has one.
    pub fn user_by_name(&self, user_name: &str) -> Option<&Account> {
        self.users.iter().find(|user| user.user_name == user_name)
    }

    /// The group with `name`, when the snapshot has one.
    pub fn group_by_name(&self, name: &str) -> Option<&Group> {
        self.groups().iter().find(|group| group.name == name)
    }

    /// Whether the read found any user at all. AccountsService being present
    /// but caching no user is still `Available`; this is the second hide rule
    /// beside the adapter-level `Unavailable`.
    pub fn present(&self) -> bool {
        !self.users.is_empty()
    }

    /// How many human users the snapshot has.
    pub fn human_count(&self) -> usize {
        self.users.iter().filter(|user| !user.is_system()).count()
    }

    /// How many human users are administrators.
    pub fn admin_count(&self) -> usize {
        self.users
            .iter()
            .filter(|user| !user.is_system() && user.is_admin())
            .count()
    }

    /// How many human users are locked.
    pub fn locked_count(&self) -> usize {
        self.users
            .iter()
            .filter(|user| !user.is_system() && user.is_locked())
            .count()
    }

    /// The user configured for automatic login, when one is.
    pub fn automatic_login_user(&self) -> Option<&Account> {
        self.users.iter().find(|user| user.automatic_login)
    }

    /// The design-system glyph for the pane header and tile.
    pub fn glyph(&self) -> &'static str {
        "users"
    }

    /// A one-line label for the pane header / tile.
    pub fn label(&self) -> String {
        if !self.present() {
            return "No Users".to_owned();
        }
        match self.human_count() {
            0 => "No Users".to_owned(),
            1 => "1 User".to_owned(),
            count => format!("{count} Users"),
        }
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the users (added, removed, edited), then the automatic login user, then
    /// the group provider and its groups.
    ///
    /// This is the "event" half of the adapter: the shell bridge diffs two
    /// reads to learn what moved without polling each field.
    pub fn changes(&self, previous: &AccountsSnapshot) -> Vec<AccountsChange> {
        let mut changes = Vec::new();

        for user in &self.users {
            match previous.user_by_name(&user.user_name) {
                None => changes.push(AccountsChange::UserAdded {
                    user_name: user.user_name.clone(),
                }),
                Some(previous_user) if previous_user != user => {
                    changes.push(AccountsChange::UserChanged {
                        user_name: user.user_name.clone(),
                    })
                }
                Some(_) => {}
            }
        }
        for user in &previous.users {
            if self.user_by_name(&user.user_name).is_none() {
                changes.push(AccountsChange::UserRemoved {
                    user_name: user.user_name.clone(),
                });
            }
        }

        let was = previous
            .automatic_login_user()
            .map(|user| user.user_name.clone());
        let now = self
            .automatic_login_user()
            .map(|user| user.user_name.clone());
        if was != now {
            changes.push(AccountsChange::AutomaticLoginChanged { from: was, to: now });
        }

        match (&previous.groups, &self.groups) {
            (None, Some(_)) => {
                changes.push(AccountsChange::GroupProviderChanged { available: true })
            }
            (Some(_), None) => {
                changes.push(AccountsChange::GroupProviderChanged { available: false })
            }
            _ => {}
        }
        if let (Some(previous_groups), Some(groups)) = (&previous.groups, &self.groups) {
            for group in groups {
                match previous_groups.iter().find(|g| g.name == group.name) {
                    None => changes.push(AccountsChange::GroupAdded {
                        name: group.name.clone(),
                    }),
                    Some(previous_group) if previous_group != group => {
                        changes.push(AccountsChange::GroupChanged {
                            name: group.name.clone(),
                        })
                    }
                    Some(_) => {}
                }
            }
            for group in previous_groups {
                if !groups.iter().any(|g| g.name == group.name) {
                    changes.push(AccountsChange::GroupRemoved {
                        name: group.name.clone(),
                    });
                }
            }
        }

        changes
    }
}

/// A change between two Users and Groups snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsChange {
    /// A user was added.
    UserAdded {
        /// The new user's login name.
        user_name: String,
    },
    /// A user was deleted.
    UserRemoved {
        /// The removed user's login name.
        user_name: String,
    },
    /// A user's properties moved.
    UserChanged {
        /// The edited user's login name.
        user_name: String,
    },
    /// The automatic login user changed.
    AutomaticLoginChanged {
        /// The previous automatic login user, if any.
        from: Option<String>,
        /// The new automatic login user, if any.
        to: Option<String>,
    },
    /// The distribution group provider appeared or went away.
    GroupProviderChanged {
        /// `true` when the provider appeared, `false` when it vanished.
        available: bool,
    },
    /// A group was added.
    GroupAdded {
        /// The new group's name.
        name: String,
    },
    /// A group was deleted.
    GroupRemoved {
        /// The removed group's name.
        name: String,
    },
    /// A group's properties or membership moved.
    GroupChanged {
        /// The edited group's name.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> AccountsData {
        AccountsData {
            users: vec![
                AccountData {
                    uid: 0,
                    user_name: "root".to_owned(),
                    real_name: "root".to_owned(),
                    account_type: AccountType::Administrator,
                    system_account: true,
                    ..AccountData::default()
                },
                AccountData {
                    uid: 1001,
                    user_name: "sam".to_owned(),
                    real_name: "Sam Smith".to_owned(),
                    account_type: AccountType::Standard,
                    locked: true,
                    ..AccountData::default()
                },
                AccountData {
                    uid: 1000,
                    user_name: "dan".to_owned(),
                    real_name: "Dan Doe".to_owned(),
                    account_type: AccountType::Administrator,
                    automatic_login: true,
                    ..AccountData::default()
                },
            ],
            groups: Some(vec![
                GroupData {
                    name: "users".to_owned(),
                    gid: 100,
                    members: vec!["dan".to_owned(), "sam".to_owned()],
                    system: true,
                },
                GroupData {
                    name: "wheel".to_owned(),
                    gid: 10,
                    members: vec!["dan".to_owned()],
                    system: false,
                },
            ]),
        }
    }

    #[test]
    fn the_snapshot_orders_and_types_the_read() {
        let snapshot = AccountsSnapshot::from_data(&data());
        assert!(snapshot.present());
        assert_eq!(snapshot.users().len(), 3);
        // Human users first, by uid: dan (1000), sam (1001), then root.
        assert_eq!(snapshot.human_users()[0].user_name, "dan");
        assert_eq!(snapshot.human_users()[1].user_name, "sam");
        assert_eq!(snapshot.system_users()[0].user_name, "root");
        assert_eq!(snapshot.human_count(), 2);
        assert_eq!(snapshot.admin_count(), 1);
        assert_eq!(snapshot.locked_count(), 1);
        assert_eq!(snapshot.label(), "2 Users");
        assert_eq!(snapshot.glyph(), "users");
    }

    #[test]
    fn groups_are_ordered_and_addressable() {
        let snapshot = AccountsSnapshot::from_data(&data());
        assert!(snapshot.groups_available());
        assert_eq!(snapshot.groups().len(), 2);
        // User groups first, by name.
        assert_eq!(snapshot.groups()[0].name, "wheel");
        assert!(!snapshot.groups()[0].is_system());
        assert_eq!(snapshot.groups()[0].member_count(), 1);
        assert!(snapshot.groups()[0].contains("dan"));
        assert_eq!(snapshot.groups()[1].name, "users");
        assert!(snapshot.groups()[1].is_system());
        assert_eq!(
            snapshot.group_by_name("wheel").map(Group::display_name),
            Some("wheel")
        );
        assert!(snapshot.group_by_name("missing").is_none());
    }

    #[test]
    fn an_absent_group_provider_is_a_normal_snapshot() {
        let raw = AccountsData {
            groups: None,
            ..data()
        };
        let snapshot = AccountsSnapshot::from_data(&raw);
        assert!(!snapshot.groups_available());
        assert!(snapshot.groups().is_empty());
        // The users half stays live.
        assert_eq!(snapshot.human_count(), 2);
    }

    #[test]
    fn user_helpers_are_honest() {
        let snapshot = AccountsSnapshot::from_data(&data());
        let dan = snapshot.user_by_name("dan").unwrap();
        assert_eq!(dan.display_name(), "Dan Doe");
        assert_eq!(dan.initial(), "D");
        assert!(dan.is_admin());
        assert!(!dan.is_locked());
        assert!(snapshot.user_by_uid(1000).is_some());
        assert_eq!(
            snapshot
                .automatic_login_user()
                .map(|user| user.user_name.as_str()),
            Some("dan")
        );

        let fallback = Account {
            user_name: "guest".to_owned(),
            ..Account::default()
        };
        assert_eq!(fallback.display_name(), "guest");
        assert_eq!(fallback.initial(), "G");
        assert!(!fallback.has_avatar());
    }

    #[test]
    fn an_empty_read_is_not_present_but_is_a_snapshot() {
        let snapshot = AccountsSnapshot::from_data(&AccountsData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.human_count(), 0);
        assert_eq!(snapshot.label(), "No Users");
        assert!(!snapshot.groups_available());
    }

    #[test]
    fn each_user_move_is_a_change() {
        let previous = AccountsSnapshot::from_data(&data());

        let mut raw = data();
        raw.users[1].locked = false; // sam edited
        raw.users.push(AccountData {
            uid: 1002,
            user_name: "kim".to_owned(),
            ..AccountData::default()
        });
        let next = AccountsSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&AccountsChange::UserChanged {
            user_name: "sam".to_owned()
        }));
        assert!(changes.contains(&AccountsChange::UserAdded {
            user_name: "kim".to_owned()
        }));
        assert!(!changes.contains(&AccountsChange::AutomaticLoginChanged {
            from: Some("dan".to_owned()),
            to: Some("dan".to_owned())
        }));
    }

    #[test]
    fn removing_a_user_and_clearing_auto_login_is_a_change() {
        let previous = AccountsSnapshot::from_data(&data());
        let mut raw = data();
        raw.users.retain(|user| user.uid != 1000);
        let next = AccountsSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&AccountsChange::UserRemoved {
            user_name: "dan".to_owned()
        }));
        assert!(changes.contains(&AccountsChange::AutomaticLoginChanged {
            from: Some("dan".to_owned()),
            to: None,
        }));
    }

    #[test]
    fn the_group_provider_appearing_and_going_away_is_a_change() {
        let with = AccountsSnapshot::from_data(&data());
        let without = AccountsSnapshot::from_data(&AccountsData {
            groups: None,
            ..data()
        });

        assert!(without
            .changes(&with)
            .contains(&AccountsChange::GroupProviderChanged { available: false }));
        assert!(with
            .changes(&without)
            .contains(&AccountsChange::GroupProviderChanged { available: true }));
    }

    #[test]
    fn group_add_remove_and_edit_are_changes() {
        let previous = AccountsSnapshot::from_data(&data());
        let mut raw = data();
        raw.groups.as_mut().unwrap().push(GroupData {
            name: "devs".to_owned(),
            gid: 200,
            ..GroupData::default()
        });
        raw.groups.as_mut().unwrap().retain(|g| g.name != "users");
        raw.groups.as_mut().unwrap()[0]
            .members
            .push("sam".to_owned());
        let next = AccountsSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&AccountsChange::GroupAdded {
            name: "devs".to_owned()
        }));
        assert!(changes.contains(&AccountsChange::GroupRemoved {
            name: "users".to_owned()
        }));
        assert!(changes.contains(&AccountsChange::GroupChanged {
            name: "wheel".to_owned()
        }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = AccountsSnapshot::from_data(&data());
        assert!(snapshot.changes(&snapshot).is_empty());
        assert_eq!(
            AccountsSnapshot::default(),
            AccountsSnapshot::from_data(&AccountsData::default())
        );
    }
}
