// SPDX-License-Identifier: MIT
//! Bridge tests for the Privacy and Security half of the host (T-15.13b): the
//! view the Settings pane and Control Center tile decode, plus the two explicit
//! permission writes applied once through the adapter.

use dragonfruit_privacy_adapter::{
    AppPermissionData, MockPrivacy, PrivacyData, ResourceData, TableData,
};
use dragonfruit_system_status::{privacy_report, PrivacyHost};

fn data() -> PrivacyData {
    PrivacyData {
        tables: vec![
            TableData {
                table: "devices".to_owned(),
                resources: vec![ResourceData {
                    id: "camera".to_owned(),
                    apps: vec![
                        AppPermissionData {
                            app: "org.example.Snapshot".to_owned(),
                            permissions: vec!["no".to_owned()],
                        },
                        AppPermissionData {
                            app: "org.mozilla.firefox".to_owned(),
                            permissions: vec!["yes".to_owned()],
                        },
                    ],
                }],
            },
            TableData {
                table: "location".to_owned(),
                resources: vec![ResourceData {
                    id: "location".to_owned(),
                    apps: vec![AppPermissionData {
                        app: "org.example.Maps".to_owned(),
                        permissions: vec!["exact".to_owned(), "now".to_owned()],
                    }],
                }],
            },
            TableData {
                table: "notifications".to_owned(),
                resources: vec![ResourceData {
                    id: "notification".to_owned(),
                    apps: vec![AppPermissionData {
                        app: "org.example.Calendar".to_owned(),
                        permissions: vec!["ask".to_owned()],
                    }],
                }],
            },
        ],
    }
}

#[test]
fn the_privacy_view_exposes_the_categories_and_counts() {
    let mut host = PrivacyHost::new(MockPrivacy::present(data()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "privacy");
    assert_eq!(view["state"], "available");
    assert_eq!(view["glyph"], "privacy");
    assert_eq!(view["label"], "4 Apps");
    assert_eq!(view["present"], true);
    assert_eq!(view["appCount"], 4);
    assert_eq!(view["categoryCount"], 14);
    // The curated order: Camera before Location before Notifications.
    assert_eq!(view["categories"][0]["id"], "devices");
    assert_eq!(
        view["categories"][0]["resources"][0]["apps"][1]["state"],
        "allowed"
    );
    assert_eq!(view["categories"][1]["id"], "location");
    // A non-tristate record is not guessed.
    assert_eq!(
        view["categories"][1]["resources"][0]["apps"][0]["state"],
        "unset"
    );
    assert_eq!(view["categories"][2]["summary"], "1 app");
    // Empty known categories are still rows.
    assert_eq!(view["categories"][3]["id"], "screencast");
    assert_eq!(view["categories"][3]["summary"], "None");
}

#[test]
fn an_empty_store_is_available_but_not_present() {
    let mut host = PrivacyHost::new(MockPrivacy::present(PrivacyData { tables: vec![] }));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], false);
    assert_eq!(view["label"], "No App Permissions");
    assert_eq!(view["categories"].as_array().unwrap().len(), 14);
}

#[test]
fn the_permission_writes_apply_once_and_report_denials() {
    let mut host = PrivacyHost::new(MockPrivacy::present(data()));
    host.refresh();
    assert_eq!(
        host.set_permission("devices", "camera", "org.example.Snapshot", "allowed")["outcome"],
        "applied"
    );
    assert_eq!(
        host.delete_permission("notifications", "notification", "org.example.Calendar")["outcome"],
        "applied"
    );
    assert_eq!(host.adapter().source().permission_sets(), 1);
    assert_eq!(host.adapter().source().permission_deletes(), 1);

    let mut denied =
        PrivacyHost::new(MockPrivacy::present(data()).deny_writes("portal: not authorized"));
    denied.refresh();
    assert_eq!(
        denied.set_permission("devices", "camera", "org.example.Snapshot", "denied")["outcome"],
        "denied"
    );
    // The read state stays live: a refusal is per action.
    assert_eq!(denied.view()["state"], "available");
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = PrivacyHost::new(MockPrivacy::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(
        host.set_permission("devices", "camera", "org.example.Snapshot", "allowed")["outcome"],
        "absent"
    );
    assert_eq!(
        host.delete_permission("devices", "camera", "org.example.Snapshot")["outcome"],
        "absent"
    );
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = PrivacyHost::new(MockPrivacy::failing("PermissionStore: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "PermissionStore: timeout");
}

#[test]
fn the_report_shapes_match_the_other_hosts() {
    use dragonfruit_privacy_adapter::PrivacyOutcome;
    use dragonfruit_system_adapters::AdapterError;
    assert_eq!(
        privacy_report(PrivacyOutcome::Applied)["outcome"],
        "applied"
    );
    assert_eq!(privacy_report(PrivacyOutcome::Absent)["outcome"], "absent");
    assert_eq!(
        privacy_report(PrivacyOutcome::Denied("no".to_owned()))["note"],
        "no"
    );
    assert_eq!(
        privacy_report(PrivacyOutcome::Failed(AdapterError::new("boom")))["error"],
        "boom"
    );
}
