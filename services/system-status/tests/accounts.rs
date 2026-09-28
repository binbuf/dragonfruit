// SPDX-License-Identifier: MIT
//! Bridge tests for the Users and Groups half of the host (T-15.11b): the view
//! the Settings pane and Control Center tile decode, plus the eight explicit
//! writes applied once through the adapter.

use dragonfruit_account_adapter::{
    AccountData, AccountType, AccountsData, GroupData, MockAccounts,
};
use dragonfruit_system_status::AccountsHost;

fn data(groups: bool) -> AccountsData {
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
        ],
        groups: groups.then(|| {
            vec![GroupData {
                name: "wheel".to_owned(),
                gid: 10,
                members: vec!["dan".to_owned()],
                system: false,
            }]
        }),
    }
}

#[test]
fn the_accounts_view_exposes_the_users_groups_and_auto_login() {
    let mut host = AccountsHost::new(MockAccounts::present(data(true)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "accounts");
    assert_eq!(view["state"], "available");
    assert_eq!(view["glyph"], "users");
    assert_eq!(view["label"], "2 Users");
    assert_eq!(view["humanCount"], 2);
    assert_eq!(view["adminCount"], 1);
    assert_eq!(view["lockedCount"], 1);
    assert_eq!(view["groupsAvailable"], true);
    assert_eq!(view["automaticLoginUser"], "dan");
    assert_eq!(view["users"][0]["displayName"], "Dan Doe");
    assert_eq!(view["users"][0]["accountTypeLabel"], "Admin");
    assert_eq!(view["groups"][0]["name"], "wheel");
}

#[test]
fn a_host_without_a_group_provider_keeps_the_user_list_live() {
    let mut host = AccountsHost::new(MockAccounts::present(data(false)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["groupsAvailable"], false);
    assert_eq!(view["groupCount"], 0);
    assert_eq!(view["humanCount"], 2);
    assert_eq!(host.create_group("devs")["outcome"], "absent");
}

#[test]
fn the_account_writes_apply_once_and_report_polkit_denials() {
    let mut host = AccountsHost::new(MockAccounts::present(data(true)));
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
    assert_eq!(host.adapter().source().writes(), 5);

    let mut denied =
        AccountsHost::new(MockAccounts::present(data(true)).deny_writes("polkit: denied"));
    denied.refresh();
    assert_eq!(denied.delete_user(1000)["outcome"], "denied");
    assert_eq!(denied.view()["state"], "available");
}

#[test]
fn the_group_writes_apply_once_through_the_adapter() {
    let mut host = AccountsHost::new(MockAccounts::present(data(true)));
    host.refresh();
    assert_eq!(host.create_group("devs")["outcome"], "applied");
    assert_eq!(
        host.set_group_members("devs", vec!["dan".to_owned(), "sam".to_owned()])["outcome"],
        "applied"
    );
    assert_eq!(host.delete_group("devs")["outcome"], "applied");
    assert_eq!(host.adapter().source().group_creates(), 1);
    assert_eq!(host.adapter().source().group_member_sets(), 1);
    assert_eq!(host.adapter().source().group_deletes(), 1);
}

#[test]
fn an_unknown_account_type_is_a_failure_not_a_guess() {
    let mut host = AccountsHost::new(MockAccounts::present(data(true)));
    host.refresh();
    let report = host.set_account_type(1000, "superuser");
    assert_eq!(report["outcome"], "failed");
    assert!(report["error"].as_str().unwrap().contains("superuser"));
    assert_eq!(host.adapter().source().type_sets(), 0);
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = AccountsHost::new(MockAccounts::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(
        host.create_user("kim", "Kim", "standard")["outcome"],
        "absent"
    );
    assert_eq!(host.delete_user(1000)["outcome"], "absent");
    assert_eq!(host.create_group("devs")["outcome"], "absent");
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = AccountsHost::new(MockAccounts::failing("AccountsService: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "AccountsService: timeout");
}
