// SPDX-License-Identifier: MIT
//! The Notifications and Focus half of the bridge host (T-15.7b).
//!
//! The notification service (`services/notifications`) is reached by the
//! `dragonfruit-notify-adapter` adapter (T-15.7a); the Settings app and the
//! shell never link it. This module owns the one projection from the typed
//! [`NotificationsSnapshot`] to the flat JSON view the Settings pane draws,
//! plus the two explicit Focus writes the pane raises (the mode and the
//! per-app allow list). It mirrors the Bluetooth/storage/battery halves in
//! their own small hosts because the notification service is not one of the
//! three menu-bar adapters (ADR 0130/0131).
//!
//! A write never invents a snapshot: the host calls
//! [`NotificationsAdapter::refresh`] after a successful action (or when the
//! service signals) and the read state stays the single source of truth.
//!
//! The four global presentation preferences (`Show previews`, the sleeping /
//! locked / mirroring toggles) are **not** here: they are settingsd keys
//! (ADR 0131), not notification-service policy, and the pane writes them
//! through the `Settings` singleton.

use dragonfruit_notify_adapter::{
    AppPreference, FocusMode, FocusOutcome, NotificationStyle, NotificationsAdapter,
    NotificationsSnapshot, NotificationsSource,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_NOTIFICATIONS: &str = "notifications";

/// The bridge host for the notification adapter: one state path and the two
/// explicit Focus writes the Settings pane offers.
#[derive(Debug, Clone, PartialEq)]
pub struct NotificationsHost<S> {
    adapter: NotificationsAdapter<S>,
}

impl<S: NotificationsSource> NotificationsHost<S> {
    /// A host over a notification source.
    pub fn new(source: S) -> Self {
        NotificationsHost {
            adapter: NotificationsAdapter::new(source),
        }
    }

    /// Re-read the notification service once. Called on startup, when the
    /// service signals, and after an explicit action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The notifications view the pane and tile render.
    pub fn view(&self) -> Value {
        notifications_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Set the Focus/DND mode by its stable id (`off`/`focus`/`dnd`). An
    /// unknown id is a failure, never a guess. One explicit write.
    pub fn set_focus_mode(&mut self, mode: &str) -> Value {
        match FocusMode::parse(mode) {
            Some(mode) => focus_report(self.adapter.set_focus_mode(mode)),
            None => json!({
                "outcome": "failed",
                "error": format!("unknown focus mode: {mode}"),
            }),
        }
    }

    /// Replace the per-app Focus allow list. One explicit write.
    pub fn set_focus_allow_list(&mut self, apps: Vec<String>) -> Value {
        focus_report(self.adapter.set_focus_allow_list(apps))
    }

    /// Select a named Focus profile (`personal`/`work`/`sleep`/`custom`). One
    /// explicit write; the service rejects an unknown id.
    pub fn set_focus_profile(&mut self, id: &str) -> Value {
        focus_report(self.adapter.set_focus_profile(id))
    }

    /// Replace the allowed-people list. One explicit write.
    pub fn set_focus_allowed_people(&mut self, people: Vec<String>) -> Value {
        focus_report(self.adapter.set_focus_allowed_people(people))
    }

    /// Set one app's durable notification preference (T229a). One explicit
    /// write; the host re-reads the service afterwards.
    pub fn set_app_preference(
        &mut self,
        app: &str,
        style: &str,
        sound: bool,
        badge: bool,
        allowed: bool,
    ) -> Value {
        let Some(style) = NotificationStyle::parse(style) else {
            return json!({
                "outcome": "failed",
                "error": format!("unknown notification style: {style}"),
            });
        };
        let preference = AppPreference {
            allowed,
            style,
            sound,
            badge,
        };
        focus_report(self.adapter.set_app_preference(app, preference))
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &NotificationsAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut NotificationsAdapter<S> {
        &mut self.adapter
    }
}

/// Build the notifications view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the Focus
/// policy and the observed per-app notification list.
pub fn notifications_view<S: NotificationsSource>(adapter: &NotificationsAdapter<S>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_NOTIFICATIONS, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_NOTIFICATIONS,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_NOTIFICATIONS, "state": "unavailable" });
    };
    notifications_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn notifications_snapshot_view(snapshot: &NotificationsSnapshot) -> Value {
    let focus = snapshot.focus();
    let apps: Vec<Value> = snapshot
        .apps()
        .iter()
        .map(|app| {
            json!({
                "name": app.name,
                "allowed": app.allowed,
                "notificationsAllowed": app.notifications_allowed,
                "notifications": app.notifications,
                "status": app.status_label(),
                "style": app.style.name(),
                "styleLabel": app.style.label(),
                "sound": app.sound,
                "badge": app.badge,
            })
        })
        .collect();
    let profiles: Vec<Value> = focus
        .profiles
        .iter()
        .map(|profile| {
            json!({
                "id": profile.id,
                "name": profile.name,
                "mode": profile.mode.name(),
            })
        })
        .collect();
    let people: Vec<Value> = snapshot
        .people()
        .iter()
        .map(|person| {
            json!({
                "name": person.name,
                "allowed": person.allowed,
                "notifications": person.notifications,
            })
        })
        .collect();
    json!({
        "kind": KIND_NOTIFICATIONS,
        "state": "available",
        "glyph": "bell",
        "label": focus.profile_label(),
        "mode": focus.mode.name(),
        "modeLabel": focus.label(),
        "profile": focus.active_profile,
        "profileLabel": focus.profile_label(),
        "profiles": profiles,
        "suppressing": focus.is_suppressing(),
        "dnd": focus.is_dnd(),
        "batched": focus.batched,
        "allowList": focus.allow_list,
        "allowedPeople": focus.allowed_people,
        "activeCount": snapshot.active_count(),
        "historyCount": snapshot.history_count(),
        "appCount": apps.len(),
        "apps": apps,
        "peopleCount": people.len(),
        "people": people,
    })
}

/// The JSON report for a Focus write (`applied`/`absent`/`failed`), shaped
/// like the other hosts' reports so one decode path reads every action.
pub fn focus_report(outcome: FocusOutcome) -> Value {
    match outcome {
        FocusOutcome::Applied => json!({ "outcome": "applied" }),
        FocusOutcome::Absent => json!({ "outcome": "absent" }),
        FocusOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_notify_adapter::{MockNotifications, NotificationRecord, NotificationsData};

    fn record(id: u32, app: &str, suppressed: bool) -> NotificationRecord {
        NotificationRecord {
            id,
            app_name: app.to_owned(),
            summary: "hello".to_owned(),
            sender: String::new(),
            suppressed,
            created_at_ms: 1000 + u64::from(id),
            ..NotificationRecord::default()
        }
    }

    fn data(mode: FocusMode) -> NotificationsData {
        NotificationsData {
            mode,
            allow_list: vec!["Pager".to_owned()],
            allowed_people: vec!["Ada".to_owned()],
            active_profile: Some("personal".to_owned()),
            profiles: dragonfruit_notify_adapter::FocusProfile::catalog(),
            batched_count: 2,
            active: vec![record(1, "Mail", false)],
            history: vec![record(2, "Pager", true), record(3, "chat", false)],
            app_preferences: vec![],
        }
    }

    #[test]
    fn an_absent_service_projects_a_hidden_slot() {
        let mut host = NotificationsHost::new(MockNotifications::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "notifications");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_present_service_projects_the_focus_policy_and_apps() {
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Focus)));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "bell");
        assert_eq!(view["label"], "Personal");
        assert_eq!(view["mode"], "focus");
        assert_eq!(view["modeLabel"], "Focus");
        assert_eq!(view["profile"], "personal");
        assert_eq!(view["profileLabel"], "Personal");
        assert_eq!(view["profiles"][0]["id"], "dnd");
        assert_eq!(view["profiles"][1]["id"], "personal");
        assert_eq!(view["profiles"][3]["mode"], "dnd");
        assert_eq!(view["suppressing"], true);
        assert_eq!(view["dnd"], false);
        assert_eq!(view["batched"], 2);
        assert_eq!(view["allowList"][0], "Pager");
        assert_eq!(view["allowedPeople"][0], "Ada");
        assert_eq!(view["peopleCount"], 0);
        assert_eq!(view["activeCount"], 1);
        assert_eq!(view["historyCount"], 2);
        assert_eq!(view["appCount"], 3);
        // Case-insensitive sort: chat, Mail, Pager.
        assert_eq!(view["apps"][0]["name"], "chat");
        assert_eq!(view["apps"][0]["allowed"], false);
        assert_eq!(view["apps"][1]["name"], "Mail");
        assert_eq!(view["apps"][2]["name"], "Pager");
        assert_eq!(view["apps"][2]["allowed"], true);
        assert_eq!(view["apps"][2]["status"], "Allowed");
        assert_eq!(view["apps"][2]["notifications"], 1);
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = NotificationsHost::new(MockNotifications::failing("notifications: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "notifications: timeout");
    }

    #[test]
    fn the_focus_mode_write_applies_once_and_the_read_state_converges() {
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Off)));
        host.refresh();
        assert_eq!(host.set_focus_mode("dnd")["outcome"], "applied");
        assert_eq!(host.adapter().source().focus_writes(), 1);
        // The write invents no snapshot until the host re-reads.
        assert_eq!(host.view()["mode"], "off");
        host.refresh();
        assert_eq!(host.view()["mode"], "dnd");
        assert_eq!(host.view()["dnd"], true);

        // An unknown id is a failure, not a guess, and never reaches the
        // service.
        let report = host.set_focus_mode("vacation");
        assert_eq!(report["outcome"], "failed");
        assert!(report["error"].as_str().unwrap().contains("vacation"));
        assert_eq!(host.adapter().source().focus_writes(), 1);
    }

    #[test]
    fn the_allow_list_write_replaces_the_list() {
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Dnd)));
        host.refresh();
        assert_eq!(
            host.set_focus_allow_list(vec!["Mail".to_owned(), "chat".to_owned()])["outcome"],
            "applied"
        );
        assert_eq!(host.adapter().source().focus_writes(), 1);
        host.refresh();
        let view = host.view();
        assert_eq!(view["allowList"][0], "Mail");
        assert_eq!(view["allowList"][1], "chat");
        assert_eq!(view["apps"][0]["allowed"], true);
    }

    #[test]
    fn a_write_while_absent_reports_absence_and_keeps_the_item_hidden() {
        let mut host = NotificationsHost::new(MockNotifications::absent());
        host.refresh();
        assert_eq!(host.set_focus_mode("dnd")["outcome"], "absent");
        assert_eq!(
            host.set_focus_allow_list(vec!["Mail".to_owned()])["outcome"],
            "absent"
        );
        assert_eq!(host.adapter().source().focus_writes(), 0);
        assert_eq!(host.view()["state"], "unavailable");
    }

    #[test]
    fn the_app_preference_write_applies_once_and_projects_the_app_fields() {
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Off)));
        host.refresh();
        assert_eq!(
            host.set_app_preference("Mail", "alert", false, false, false)["outcome"],
            "applied"
        );
        assert_eq!(host.adapter().source().focus_writes(), 1);
        // The write invents no snapshot until the host re-reads.
        assert_eq!(host.view()["apps"][1]["name"], "Mail");
        host.refresh();
        let view = host.view();
        let mail = &view["apps"][1];
        assert_eq!(mail["name"], "Mail");
        assert_eq!(mail["status"], "Off");
        assert_eq!(mail["notificationsAllowed"], false);
        assert_eq!(mail["style"], "alert");
        assert_eq!(mail["styleLabel"], "Alerts");
        assert_eq!(mail["sound"], false);
        assert_eq!(mail["badge"], false);
    }

    #[test]
    fn an_unknown_app_style_is_a_failure_and_never_reaches_the_service() {
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Off)));
        host.refresh();
        let report = host.set_app_preference("Mail", "vacation", true, true, true);
        assert_eq!(report["outcome"], "failed");
        assert!(report["error"].as_str().unwrap().contains("vacation"));
        assert_eq!(host.adapter().source().focus_writes(), 0);
    }

    #[test]
    fn a_muted_app_is_suppressed_by_the_service_policy() {
        use dragonfruit_notify_adapter::AppPreference;
        let mut host = NotificationsHost::new(MockNotifications::present(data(FocusMode::Off)));
        host.refresh();
        host.adapter_mut().source_mut().set_app_preference(
            "Mail",
            AppPreference {
                allowed: false,
                ..AppPreference::DEFAULT
            },
        );
        host.refresh();
        assert_eq!(host.view()["apps"][1]["status"], "Off");
    }
}
