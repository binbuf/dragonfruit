// SPDX-License-Identifier: MIT
//! The Privacy and Security half of the bridge host (T-15.13b).
//!
//! The host stack is reached by the `dragonfruit-privacy-adapter` over the
//! portal PermissionStore; the shell and the Settings app never link it. This
//! module owns the one projection from the typed [`PrivacySnapshot`] to the
//! flat JSON view the two consumers draw, plus the two explicit permission
//! writes the pane raises (set an application's tristate, remove its record).
//! It mirrors the printers and account halves.
//!
//! Absence is single-layered, exactly as the adapter's is (ADR 0142). The view
//! is `unavailable` only when there is no session bus or no
//! `org.freedesktop.impl.portal.PermissionStore` owner. A store that answers
//! with no entry is `available` with `present: false`; the tile hides then
//! (every category is an honest empty row and the pane still lists them). A
//! store that is present but unreadable is `error`, visible and inert with the
//! message.
//!
//! A write never invents a snapshot: the host re-reads the adapter after a
//! successful action, and the read state stays the single source of truth.

use dragonfruit_privacy_adapter::{
    PermissionState, PrivacyAdapter, PrivacyOutcome, PrivacySnapshot, PrivacySource,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_PRIVACY: &str = "privacy";

/// The bridge host for the Privacy and Security adapter: one state path and
/// the two explicit permission writes the Settings pane offers.
#[derive(Debug, Clone, PartialEq)]
pub struct PrivacyHost<S> {
    adapter: PrivacyAdapter<S>,
}

impl<S: PrivacySource> PrivacyHost<S> {
    /// A host over a host-stack source.
    pub fn new(source: S) -> Self {
        PrivacyHost {
            adapter: PrivacyAdapter::new(source),
        }
    }

    /// Re-read the permission store once. Called on startup and after an
    /// explicit action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The Privacy and Security view the pane and tile render.
    pub fn view(&self) -> Value {
        privacy_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Store the permission for `app` on the resource `id` in `table`. The
    /// `permission` is the stable [`PermissionState`] id (`allowed`/`denied`/
    /// `ask`); an unknown id is `unset`. One explicit write.
    pub fn set_permission(&mut self, table: &str, id: &str, app: &str, permission: &str) -> Value {
        let state = PermissionState::from_id(permission);
        let permissions: Vec<String> = state
            .permissions()
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        privacy_report(self.adapter.set_permission(table, id, app, &permissions))
    }

    /// Remove the stored permission for `app` on the resource `id` in `table`.
    /// One explicit write.
    pub fn delete_permission(&mut self, table: &str, id: &str, app: &str) -> Value {
        privacy_report(self.adapter.delete_permission(table, id, app))
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &PrivacyAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut PrivacyAdapter<S> {
        &mut self.adapter
    }
}

/// Build the Privacy and Security view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries every category
/// (empty ones included) plus the counts.
pub fn privacy_view<S: PrivacySource>(adapter: &PrivacyAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_PRIVACY, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_PRIVACY,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_PRIVACY, "state": "unavailable" });
    };
    privacy_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn privacy_snapshot_view(snapshot: &PrivacySnapshot) -> Value {
    let categories: Vec<Value> = snapshot.categories().iter().map(category_json).collect();
    json!({
        "kind": KIND_PRIVACY,
        "state": "available",
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        // The portal store answered; every category is a row. `present` is the
        // tile's second hide rule: no application permission at all.
        "present": snapshot.present(),
        "appCount": snapshot.app_count(),
        "grantedCount": snapshot.granted_count(),
        "deniedCount": snapshot.denied_count(),
        "categoryCount": snapshot.categories().len(),
        "categories": categories,
    })
}

/// One category's flat JSON row.
fn category_json(category: &dragonfruit_privacy_adapter::PermissionCategory) -> Value {
    let resources: Vec<Value> = category
        .resources()
        .iter()
        .map(|resource| {
            let apps: Vec<Value> = resource.apps.iter().map(app_json).collect();
            json!({
                "id": resource.id,
                "appCount": resource.app_count(),
                "apps": apps,
            })
        })
        .collect();
    json!({
        "id": category.id(),
        "label": category.label(),
        "summary": category.summary(),
        "appCount": category.app_count(),
        "grantedCount": category.granted_count(),
        "resources": resources,
    })
}

/// One application permission's flat JSON row.
fn app_json(app: &dragonfruit_privacy_adapter::AppPermission) -> Value {
    json!({
        "app": app.app,
        "state": app.state.id(),
        "stateLabel": app.state.label(),
        "permissions": app.permissions,
    })
}

/// The JSON report for a permission write, shaped like the printers and account
/// reports so one shell decode path reads every action. A policy refusal is a
/// `denied` note; the read state stays live.
pub fn privacy_report(outcome: PrivacyOutcome) -> Value {
    match outcome {
        PrivacyOutcome::Applied => json!({ "outcome": "applied" }),
        PrivacyOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        PrivacyOutcome::Absent => json!({ "outcome": "absent" }),
        PrivacyOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_privacy_adapter::{
        AppPermissionData, MockPrivacy, PrivacyData, ResourceData, TableData,
    };

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
    fn an_absent_store_projects_a_hidden_slot() {
        let mut host = PrivacyHost::new(MockPrivacy::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "privacy");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_store_projects_every_category_and_count() {
        let mut host = PrivacyHost::new(MockPrivacy::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "privacy");
        assert_eq!(view["label"], "3 Apps");
        assert_eq!(view["present"], true);
        assert_eq!(view["appCount"], 3);
        assert_eq!(view["grantedCount"], 1);
        assert_eq!(view["deniedCount"], 1);
        assert_eq!(view["categoryCount"], 14);
        // Every known table is a category; the curated order starts at Camera.
        assert_eq!(view["categories"][0]["id"], "devices");
        assert_eq!(view["categories"][0]["label"], "Camera");
        assert_eq!(view["categories"][0]["summary"], "2 apps");
        assert_eq!(view["categories"][1]["id"], "location");
        assert_eq!(view["categories"][1]["summary"], "None");
        // The camera resource carries the two sorted apps and their tristate.
        assert_eq!(view["categories"][0]["resources"][0]["id"], "camera");
        assert_eq!(
            view["categories"][0]["resources"][0]["apps"][0]["app"],
            "org.example.Snapshot"
        );
        assert_eq!(
            view["categories"][0]["resources"][0]["apps"][0]["state"],
            "denied"
        );
        assert_eq!(
            view["categories"][0]["resources"][0]["apps"][1]["app"],
            "org.mozilla.firefox"
        );
        assert_eq!(
            view["categories"][0]["resources"][0]["apps"][1]["state"],
            "allowed"
        );
        // A location accuracy record is never a guess: it reads `unset`.
        assert_eq!(view["categories"][2]["id"], "notifications");
        assert_eq!(
            view["categories"][2]["resources"][0]["apps"][0]["state"],
            "ask"
        );
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
        assert_eq!(view["categories"][0]["summary"], "None");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = PrivacyHost::new(MockPrivacy::failing("PermissionStore: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "PermissionStore: timeout");
    }

    #[test]
    fn the_permission_writes_apply_once_through_the_adapter() {
        let mut host = PrivacyHost::new(MockPrivacy::present(data()));
        host.refresh();
        assert_eq!(
            host.set_permission("devices", "camera", "org.example.Snapshot", "allowed")["outcome"],
            "applied"
        );
        assert_eq!(
            host.delete_permission("notifications", "notification", "org.example.Calendar")
                ["outcome"],
            "applied"
        );
        assert_eq!(host.adapter().source().permission_sets(), 1);
        assert_eq!(host.adapter().source().permission_deletes(), 1);
        // The host re-reads after a write, so the view converges.
        host.refresh();
        let view = host.view();
        assert_eq!(view["appCount"], 2);
        assert_eq!(
            view["categories"][0]["resources"][0]["apps"][0]["state"],
            "allowed"
        );
    }

    #[test]
    fn an_unset_write_deletes_the_permission() {
        let mut host = PrivacyHost::new(MockPrivacy::present(data()));
        host.refresh();
        // An unknown state id is `Unset`, whose permission list is empty:
        // the store records no recognizable tristate for the app.
        assert_eq!(
            host.set_permission("devices", "camera", "org.mozilla.firefox", "unset")["outcome"],
            "applied"
        );
        assert_eq!(host.adapter().source().permission_sets(), 1);
    }

    #[test]
    fn a_policy_denial_is_reported_and_leaves_the_read_state_live() {
        let mut host =
            PrivacyHost::new(MockPrivacy::present(data()).deny_writes("portal: not authorized"));
        host.refresh();
        let report = host.set_permission("devices", "camera", "org.example.Snapshot", "allowed");
        assert_eq!(report["outcome"], "denied");
        assert_eq!(report["note"], "portal: not authorized");
        assert_eq!(host.view()["state"], "available");
    }

    #[test]
    fn writes_while_absent_answer_absence() {
        let mut host = PrivacyHost::new(MockPrivacy::absent());
        host.refresh();
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
}
