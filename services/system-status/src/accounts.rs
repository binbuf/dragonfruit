// SPDX-License-Identifier: MIT
//! The Users and Groups half of the bridge host (T-15.11b).
//!
//! The host stack is reached by the `dragonfruit-account-adapter`; the shell
//! and the Settings app never link it. This module owns the one projection from
//! the typed [`AccountsSnapshot`] to the flat JSON view the two consumers draw,
//! plus the eight explicit writes the pane raises (five account writes and
//! three group writes). It mirrors the storage and updates halves.
//!
//! Absence is layered, exactly as the adapter's is (ADR 0138). The whole view
//! is `unavailable` only when neither AccountsService nor the group provider is
//! reachable. A reachable host with **no group provider** is `available` with
//! `groupsAvailable: false`, so only the group controls disable and the user
//! list stays live. A host that is present but unreadable is `error`, visible
//! and inert with the message.
//!
//! A write never invents a snapshot: the host re-reads the adapter after a
//! successful action, and the read state stays the single source of truth.

use dragonfruit_account_adapter::{
    Account, AccountOutcome, AccountSource, AccountType, AccountsAdapter, AccountsSnapshot,
    BiometricsAdapter, BiometricsOutcome, BiometricsSnapshot, BiometricsSource, Group,
    OnlineAccountsAdapter, OnlineAccountsOutcome, OnlineAccountsSnapshot, OnlineAccountsSource,
    PasswordMode,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

use crate::shared::SharedSource;

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_ACCOUNTS: &str = "accounts";
const KIND_BIOMETRICS: &str = "biometrics";
const KIND_ONLINE_ACCOUNTS: &str = "online-accounts";

/// The bridge host for the account services: Users & Groups, Biometrics &
/// Password, and Online Accounts share one host-stack source (T228).
#[derive(Debug, Clone, PartialEq)]
pub struct AccountsHost<S> {
    adapter: AccountsAdapter<SharedSource<S>>,
    biometrics: BiometricsAdapter<SharedSource<S>>,
    online: OnlineAccountsAdapter<SharedSource<S>>,
}

impl<S: AccountSource + BiometricsSource + OnlineAccountsSource + Send> AccountsHost<S> {
    /// A host over a host-stack source.
    pub fn new(source: S) -> Self {
        let shared = SharedSource::new(source);
        AccountsHost {
            adapter: AccountsAdapter::new(shared.clone()),
            biometrics: BiometricsAdapter::new(shared.clone()),
            online: OnlineAccountsAdapter::new(shared),
        }
    }

    /// Re-read the host stack once. Called on startup and after an explicit
    /// action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
        self.biometrics.refresh();
        self.online.refresh();
    }

    /// Re-read only the user/group half.
    pub fn refresh_accounts(&mut self) {
        self.adapter.refresh();
    }

    /// Re-read only the biometrics half.
    pub fn refresh_biometrics(&mut self) {
        self.biometrics.refresh();
    }

    /// Re-read only the online-accounts half.
    pub fn refresh_online_accounts(&mut self) {
        self.online.refresh();
    }

    /// The Users and Groups view the pane and tile render.
    pub fn view(&self) -> Value {
        accounts_view(&self.adapter)
    }

    /// The Biometrics and Password view.
    pub fn biometrics_view(&self) -> Value {
        biometrics_view(&self.biometrics)
    }

    /// The Online Accounts view.
    pub fn online_accounts_view(&self) -> Value {
        online_accounts_view(&self.online)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// The Biometrics view as JSON.
    pub fn biometrics_state(&self) -> String {
        self.biometrics_view().to_string()
    }

    /// The Online Accounts view as JSON.
    pub fn online_accounts_state(&self) -> String {
        self.online_accounts_view().to_string()
    }

    /// Start enrolling `finger`.
    pub fn start_enrollment(&mut self, finger: &str) -> Value {
        biometrics_report(self.biometrics.start_enrollment(finger))
    }

    /// Stop the in-flight enrollment.
    pub fn cancel_enrollment(&mut self) -> Value {
        biometrics_report(self.biometrics.cancel_enrollment())
    }

    /// Delete `finger`.
    pub fn delete_finger(&mut self, finger: &str) -> Value {
        biometrics_report(self.biometrics.delete_finger(finger))
    }

    /// Change the current user's password.
    pub fn change_password(&mut self, password: &str) -> Value {
        biometrics_report(self.biometrics.change_password(password))
    }

    /// Add an online account.
    pub fn add_online_account(&mut self, provider: &str, identity: &str) -> Value {
        online_accounts_report(self.online.add_account(provider, identity))
    }

    /// Remove an online account.
    pub fn remove_online_account(&mut self, id: &str) -> Value {
        online_accounts_report(self.online.remove_account(id))
    }

    /// Set one online-account sync capability.
    pub fn set_online_account_sync(&mut self, id: &str, capability: &str, enabled: bool) -> Value {
        online_accounts_report(self.online.set_sync(id, capability, enabled))
    }

    /// Create a user. One explicit write; an unknown account-type id is a
    /// failure, never a guess.
    pub fn create_user(&mut self, user_name: &str, real_name: &str, account_type: &str) -> Value {
        let account_type = parse_account_type(account_type);
        match account_type {
            Ok(account_type) => {
                account_report(self.adapter.create_user(user_name, real_name, account_type))
            }
            Err(report) => report,
        }
    }

    /// Delete the user with `uid`. One explicit write.
    pub fn delete_user(&mut self, uid: u64) -> Value {
        account_report(self.adapter.delete_user(uid))
    }

    /// Set the account type of the user with `uid`. One explicit write; an
    /// unknown account-type id is a failure, never a guess.
    pub fn set_account_type(&mut self, uid: u64, account_type: &str) -> Value {
        match parse_account_type(account_type) {
            Ok(account_type) => account_report(self.adapter.set_account_type(uid, account_type)),
            Err(report) => report,
        }
    }

    /// Lock or unlock the user with `uid`. One explicit write.
    pub fn set_locked(&mut self, uid: u64, locked: bool) -> Value {
        account_report(self.adapter.set_locked(uid, locked))
    }

    /// Set or clear automatic login for the user with `uid`. One explicit
    /// write. Clearing passes `uid = 0`, which AccountsService treats as no
    /// automatic login user.
    pub fn set_automatic_login(&mut self, uid: u64, automatic_login: bool) -> Value {
        account_report(self.adapter.set_automatic_login(uid, automatic_login))
    }

    /// Create a group. One explicit write (the distro provider's).
    pub fn create_group(&mut self, name: &str) -> Value {
        account_report(self.adapter.create_group(name))
    }

    /// Delete a group. One explicit write (the distro provider's).
    pub fn delete_group(&mut self, name: &str) -> Value {
        account_report(self.adapter.delete_group(name))
    }

    /// Replace a group's membership. One explicit write (the distro
    /// provider's).
    pub fn set_group_members(&mut self, name: &str, members: Vec<String>) -> Value {
        account_report(self.adapter.set_group_members(name, &members))
    }

    /// The Users & Groups adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &AccountsAdapter<SharedSource<S>> {
        &self.adapter
    }

    /// The Users & Groups adapter, mutably (mostly for tests that drive a
    /// mock source).
    pub fn adapter_mut(&mut self) -> &mut AccountsAdapter<SharedSource<S>> {
        &mut self.adapter
    }

    /// The biometrics adapter (read-only).
    pub fn biometrics_adapter(&self) -> &BiometricsAdapter<SharedSource<S>> {
        &self.biometrics
    }

    /// The online-accounts adapter (read-only).
    pub fn online_adapter(&self) -> &OnlineAccountsAdapter<SharedSource<S>> {
        &self.online
    }
}

/// Parse an account-type id; an unknown id is a `failed` report, never a
/// silent `Standard`.
fn parse_account_type(id: &str) -> Result<AccountType, Value> {
    AccountType::ALL
        .into_iter()
        .find(|account_type| account_type.id() == id)
        .ok_or_else(|| {
            json!({
                "outcome": "failed",
                "error": format!("unknown account type: {id}"),
            })
        })
}

/// Build the Users and Groups view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the user and
/// group lists.
pub fn accounts_view<S: AccountSource>(adapter: &AccountsAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_ACCOUNTS, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_ACCOUNTS,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_ACCOUNTS, "state": "unavailable" });
    };
    accounts_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn accounts_snapshot_view(snapshot: &AccountsSnapshot) -> Value {
    let users: Vec<Value> = snapshot.users().iter().map(account_json).collect();
    let groups: Vec<Value> = snapshot.groups().iter().map(group_json).collect();
    let automatic = snapshot.automatic_login_user();
    json!({
        "kind": KIND_ACCOUNTS,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "present": snapshot.present(),
        "userCount": snapshot.users.len(),
        "humanCount": snapshot.human_count(),
        "systemCount": snapshot.system_users().len(),
        "adminCount": snapshot.admin_count(),
        "lockedCount": snapshot.locked_count(),
        // The distribution group provider, when present.
        "groupsAvailable": snapshot.groups_available(),
        "groupCount": groups.len(),
        // The automatic login user, when one is configured.
        "automaticLogin": automatic.map(Account::display_name).unwrap_or(""),
        "automaticLoginUser": automatic.map(|user| user.user_name.clone()).unwrap_or_default(),
        "automaticLoginUid": automatic.map(|user| user.uid).unwrap_or(0),
        "users": users,
        "groups": groups,
    })
}

/// One user's flat JSON row.
fn account_json(account: &Account) -> Value {
    json!({
        "uid": account.uid,
        "userName": account.user_name,
        "realName": account.real_name,
        "displayName": account.display_name(),
        "initial": account.initial(),
        "accountType": account.account_type.id(),
        "accountTypeLabel": account.account_type.label(),
        "passwordMode": account.password_mode.id(),
        "passwordModeLabel": account.password_mode.label(),
        "homeDirectory": account.home_directory,
        "shell": account.shell,
        "email": account.email,
        "language": account.language,
        "hasAvatar": account.has_avatar(),
        "iconFile": account.icon_file,
        "locked": account.is_locked(),
        "system": account.is_system(),
        "automaticLogin": account.automatic_login,
        "loginTime": account.login_time,
        "xSession": account.x_session,
    })
}

/// One group's flat JSON row.
fn group_json(group: &Group) -> Value {
    json!({
        "name": group.name,
        "gid": group.gid,
        "memberCount": group.member_count(),
        "members": group.members,
        "system": group.is_system(),
    })
}

/// The JSON report for an account or group write, shaped like the storage and
/// Wi-Fi reports so one shell decode path reads every action. A polkit refusal
/// is a `denied` note; the read state stays live.
pub fn account_report(outcome: AccountOutcome) -> Value {
    match outcome {
        AccountOutcome::Applied => json!({ "outcome": "applied" }),
        AccountOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        AccountOutcome::Absent => json!({ "outcome": "absent" }),
        AccountOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

/// The account-type label a consumer draws, exposed for parity with the
/// adapter.
pub fn account_type_label(account_type: AccountType) -> &'static str {
    account_type.label()
}

/// The password-mode label a consumer draws, exposed for parity with the
/// adapter.
pub fn password_mode_label(mode: PasswordMode) -> &'static str {
    mode.label()
}

/// Build the Biometrics and Password view from an adapter.
pub fn biometrics_view<S: BiometricsSource>(adapter: &BiometricsAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_BIOMETRICS, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_BIOMETRICS,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_BIOMETRICS, "state": "unavailable" });
    };
    biometrics_snapshot_view(snapshot)
}

/// Build the Biometrics view from an already-decoded snapshot.
pub fn biometrics_snapshot_view(snapshot: &BiometricsSnapshot) -> Value {
    let device = snapshot.device().map(|device| {
        let fingers: Vec<Value> = device
            .enrolled
            .iter()
            .map(|finger| {
                json!({
                    "id": finger,
                    "label": dragonfruit_account_adapter::finger_label(finger),
                })
            })
            .collect();
        json!({
            "path": device.path,
            "name": device.name,
            "scanType": device.scan_type,
            "enrolled": fingers,
            "enrolledCount": device.enrolled.len(),
            "note": device.note,
        })
    });
    let enrolling = snapshot.enrolling().map(|enrollment| {
        json!({
            "finger": enrollment.finger,
            "label": dragonfruit_account_adapter::finger_label(&enrollment.finger),
            "status": enrollment.status,
        })
    });
    json!({
        "kind": KIND_BIOMETRICS,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "user": snapshot.user(),
        "uid": snapshot.uid(),
        "fprintdAvailable": snapshot.fprintd_available(),
        "device": device,
        "pam": {
            "available": snapshot.pam().available,
            "unlock": snapshot.pam().unlock,
            "auth": snapshot.pam().auth,
            "files": snapshot.pam().files,
        },
        "password": {
            "available": snapshot.password().available,
            "method": snapshot.password().method,
            "note": snapshot.password().note,
        },
        "enrolling": enrolling,
    })
}

/// The JSON report for a biometrics write.
pub fn biometrics_report(outcome: BiometricsOutcome) -> Value {
    match outcome {
        BiometricsOutcome::Applied => json!({ "outcome": "applied" }),
        BiometricsOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        BiometricsOutcome::Absent => json!({ "outcome": "absent" }),
        BiometricsOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

/// Build the Online Accounts view from an adapter.
pub fn online_accounts_view<S: OnlineAccountsSource>(adapter: &OnlineAccountsAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_ONLINE_ACCOUNTS, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_ONLINE_ACCOUNTS,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_ONLINE_ACCOUNTS, "state": "unavailable" });
    };
    online_accounts_snapshot_view(snapshot)
}

/// Build the Online Accounts view from an already-decoded snapshot.
pub fn online_accounts_snapshot_view(snapshot: &OnlineAccountsSnapshot) -> Value {
    let providers: Vec<Value> = snapshot
        .providers()
        .iter()
        .map(|provider| {
            json!({
                "id": provider.id,
                "name": provider.name,
                "icon": provider.icon,
                "supported": provider.supported,
            })
        })
        .collect();
    let accounts: Vec<Value> = snapshot
        .accounts()
        .iter()
        .map(|account| {
            let capabilities: Vec<Value> = account
                .capabilities
                .iter()
                .map(|capability| {
                    json!({
                        "id": capability.id,
                        "label": capability.label,
                        "enabled": capability.enabled,
                    })
                })
                .collect();
            json!({
                "id": account.id,
                "path": account.path,
                "provider": account.provider,
                "providerName": account.provider_name,
                "identity": account.identity,
                "presentationIdentity": account.presentation_identity,
                "locked": account.locked,
                "attentionNeeded": account.attention_needed,
                "capabilities": capabilities,
            })
        })
        .collect();
    json!({
        "kind": KIND_ONLINE_ACCOUNTS,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "present": snapshot.present(),
        "providerCount": providers.len(),
        "accountCount": accounts.len(),
        "providers": providers,
        "accounts": accounts,
    })
}

/// The JSON report for an online-accounts write.
pub fn online_accounts_report(outcome: OnlineAccountsOutcome) -> Value {
    match outcome {
        OnlineAccountsOutcome::Applied => json!({ "outcome": "applied" }),
        OnlineAccountsOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        OnlineAccountsOutcome::Absent => json!({ "outcome": "absent" }),
        OnlineAccountsOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_account_adapter::{AccountData, AccountsData, GroupData, MockAccounts};

    fn data() -> AccountsData {
        AccountsData {
            users: vec![
                AccountData {
                    uid: 1000,
                    user_name: "dan".to_owned(),
                    real_name: "Dan Doe".to_owned(),
                    account_type: AccountType::Administrator,
                    automatic_login: true,
                    ..AccountData::default()
                },
                AccountData {
                    uid: 1001,
                    user_name: "sam".to_owned(),
                    real_name: "Sam Smith".to_owned(),
                    locked: true,
                    ..AccountData::default()
                },
                AccountData {
                    uid: 0,
                    user_name: "root".to_owned(),
                    real_name: "root".to_owned(),
                    account_type: AccountType::Administrator,
                    system_account: true,
                    ..AccountData::default()
                },
            ],
            groups: Some(vec![GroupData {
                name: "wheel".to_owned(),
                gid: 10,
                members: vec!["dan".to_owned()],
                system: false,
            }]),
        }
    }

    #[test]
    fn the_biometrics_view_carries_the_reader_pam_and_password() {
        let mut host = AccountsHost::new(MockAccounts::present(data()).with_biometrics());
        host.refresh();
        let view = host.biometrics_view();
        assert_eq!(view["kind"], "biometrics");
        assert_eq!(view["state"], "available");
        assert_eq!(view["fprintdAvailable"], true);
        assert_eq!(view["device"]["enrolledCount"], 1);
        assert_eq!(view["pam"]["available"], true);
        assert_eq!(view["pam"]["unlock"], true);
        assert_eq!(view["pam"]["auth"], true);
        assert_eq!(view["password"]["available"], true);
    }

    #[test]
    fn a_missing_fprintd_keeps_pam_and_password_live() {
        let biometrics = dragonfruit_account_adapter::MockBiometrics::with_data(
            dragonfruit_account_adapter::BiometricsData {
                user: "dan".to_owned(),
                uid: 1000,
                fprintd_available: false,
                device: None,
                pam: dragonfruit_account_adapter::PamData {
                    available: true,
                    unlock: true,
                    auth: true,
                    files: vec!["system-auth".to_owned()],
                },
                password: dragonfruit_account_adapter::PasswordData {
                    available: true,
                    method: "accountsservice".to_owned(),
                    note: String::new(),
                },
                enrolling: None,
            },
        );
        let mut host =
            AccountsHost::new(MockAccounts::present(data()).with_biometrics_source(biometrics));
        host.refresh();
        let view = host.biometrics_view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["fprintdAvailable"], false);
        assert_eq!(view["device"], serde_json::Value::Null);
        assert_eq!(view["pam"]["available"], true);
        assert_eq!(view["password"]["available"], true);
    }

    #[test]
    fn a_missing_biometrics_owner_is_an_unavailable_view() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        assert_eq!(host.biometrics_view()["state"], "unavailable");
    }

    #[test]
    fn the_online_accounts_view_carries_providers_accounts_and_sync() {
        let mut host = AccountsHost::new(MockAccounts::present(data()).with_online_accounts());
        host.refresh();
        let view = host.online_accounts_view();
        assert_eq!(view["kind"], "online-accounts");
        assert_eq!(view["state"], "available");
        assert_eq!(view["providerCount"], 2);
        assert_eq!(view["accountCount"], 1);
        assert_eq!(view["accounts"][0]["providerName"], "Google");
        assert_eq!(
            view["accounts"][0]["capabilities"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            host.set_online_account_sync("account-1", "contacts", true)["outcome"],
            "applied"
        );
        host.refresh_online_accounts();
        assert_eq!(
            host.online_accounts_view()["accounts"][0]["capabilities"][2]["enabled"],
            true
        );
    }

    #[test]
    fn an_absent_goa_is_an_unavailable_view() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        assert_eq!(host.online_accounts_view()["state"], "unavailable");
        assert_eq!(host.remove_online_account("account-1")["outcome"], "absent");
    }

    #[test]
    fn an_absent_host_stack_projects_a_hidden_slot() {
        let mut host = AccountsHost::new(MockAccounts::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "accounts");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_host_projects_the_users_and_groups() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "users");
        assert_eq!(view["label"], "2 Users");
        assert_eq!(view["humanCount"], 2);
        assert_eq!(view["adminCount"], 1);
        assert_eq!(view["lockedCount"], 1);
        assert_eq!(view["groupsAvailable"], true);
        assert_eq!(view["groupCount"], 1);
        assert_eq!(view["automaticLogin"], "Dan Doe");
        assert_eq!(view["automaticLoginUser"], "dan");
        assert_eq!(view["automaticLoginUid"], 1000);
        // Human users first, by uid.
        assert_eq!(view["users"][0]["userName"], "dan");
        assert_eq!(view["users"][0]["initial"], "D");
        assert_eq!(view["users"][0]["accountType"], "administrator");
        assert_eq!(view["users"][0]["accountTypeLabel"], "Admin");
        assert_eq!(view["users"][0]["automaticLogin"], true);
        assert_eq!(view["users"][1]["userName"], "sam");
        assert_eq!(view["users"][1]["locked"], true);
        assert_eq!(view["users"][2]["system"], true);
        assert_eq!(view["groups"][0]["name"], "wheel");
        assert_eq!(view["groups"][0]["memberCount"], 1);
    }

    #[test]
    fn a_host_without_a_group_provider_keeps_the_user_list_live() {
        let mut host = AccountsHost::new(MockAccounts::present(AccountsData {
            groups: None,
            ..data()
        }));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["groupsAvailable"], false);
        assert_eq!(view["groupCount"], 0);
        assert_eq!(view["humanCount"], 2);
        // A group write answers absence; the user half is untouched.
        assert_eq!(host.create_group("devs")["outcome"], "absent");
        assert_eq!(host.view()["state"], "available");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = AccountsHost::new(MockAccounts::failing("AccountsService: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "AccountsService: timeout");
    }

    #[test]
    fn the_account_writes_apply_once_through_the_adapter() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        assert_eq!(
            host.create_user("kim", "Kim", "standard")["outcome"],
            "applied"
        );
        assert_eq!(
            host.set_account_type(1000, "administrator")["outcome"],
            "applied"
        );
        assert_eq!(host.set_locked(1000, true)["outcome"], "applied");
        assert_eq!(host.set_automatic_login(1000, true)["outcome"], "applied");
        assert_eq!(host.delete_user(1001)["outcome"], "applied");
        assert_eq!(host.adapter().source().lock().creates(), 1);
        assert_eq!(host.adapter().source().lock().type_sets(), 1);
        assert_eq!(host.adapter().source().lock().lock_sets(), 1);
        assert_eq!(host.adapter().source().lock().auto_login_sets(), 1);
        assert_eq!(host.adapter().source().lock().deletes(), 1);
    }

    #[test]
    fn the_group_writes_apply_once_through_the_adapter() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        assert_eq!(host.create_group("devs")["outcome"], "applied");
        assert_eq!(
            host.set_group_members("devs", vec!["dan".to_owned()])["outcome"],
            "applied"
        );
        assert_eq!(host.delete_group("devs")["outcome"], "applied");
        assert_eq!(host.adapter().source().lock().group_creates(), 1);
        assert_eq!(host.adapter().source().lock().group_member_sets(), 1);
        assert_eq!(host.adapter().source().lock().group_deletes(), 1);
    }

    #[test]
    fn an_unknown_account_type_is_a_failure_not_a_guess() {
        let mut host = AccountsHost::new(MockAccounts::present(data()));
        host.refresh();
        let report = host.set_account_type(1000, "superuser");
        assert_eq!(report["outcome"], "failed");
        assert!(report["error"].as_str().unwrap().contains("superuser"));
        // The write never reached the daemon.
        assert_eq!(host.adapter().source().lock().type_sets(), 0);
        let report = host.create_user("kim", "Kim", "root-user");
        assert_eq!(report["outcome"], "failed");
        assert_eq!(host.adapter().source().lock().creates(), 0);
    }

    #[test]
    fn a_polkit_denial_is_reported_and_leaves_the_read_state_live() {
        let mut host =
            AccountsHost::new(MockAccounts::present(data()).deny_writes("polkit: denied"));
        host.refresh();
        let report = host.delete_user(1000);
        assert_eq!(report["outcome"], "denied");
        assert_eq!(report["note"], "polkit: denied");
        assert_eq!(host.view()["state"], "available");
    }

    #[test]
    fn writes_while_absent_answer_absence() {
        let mut host = AccountsHost::new(MockAccounts::absent());
        host.refresh();
        assert_eq!(
            host.create_user("kim", "Kim", "standard")["outcome"],
            "absent"
        );
        assert_eq!(host.delete_user(1000)["outcome"], "absent");
        assert_eq!(host.create_group("devs")["outcome"], "absent");
        assert_eq!(host.view()["state"], "unavailable");
    }
}
