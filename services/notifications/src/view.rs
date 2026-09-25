// SPDX-License-Identifier: MIT
//! The shell-facing JSON views (T-11.1a).
//!
//! The Qt shell never links the Rust model ([01-architecture.md],
//! "D-Bus is the services seam"), so the shell-facing interface ships flat
//! JSON the C++ side decodes into the banner list and the notification-center
//! history. Keeping the encoder here — next to the model it describes — means
//! one shape for the wire, the tests, and the docs.

use serde_json::{json, Value};

use crate::model::{Notification, Queue};
use crate::policy::FocusPolicy;

/// The banner list, oldest first.
pub fn banners_json(queue: &Queue) -> String {
    let banners: Vec<Value> = queue.banners().iter().map(banner_value).collect();
    serde_json::to_string(&Value::Array(banners)).unwrap_or_else(|_| "[]".to_owned())
}

/// The notification-center history, most recent first.
pub fn history_json(queue: &Queue) -> String {
    let history: Vec<Value> = queue
        .history()
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "appName": entry.app_name,
                "appIcon": entry.app_icon,
                "summary": entry.summary,
                "body": entry.body,
                "urgency": entry.urgency.name(),
                "createdAt": entry.created_at_ms,
                "closedAt": entry.closed_at_ms,
                "reason": entry.reason.map(|reason| reason.name()),
                "suppressed": entry.suppressed,
                "actions": actions_value(&entry.actions),
            })
        })
        .collect();
    serde_json::to_string(&Value::Array(history)).unwrap_or_else(|_| "[]".to_owned())
}

/// The Focus/DND policy state, frozen for the menu bar, the Control Center,
/// and Settings (T-11.2a).
pub fn focus_policy_json(policy: &FocusPolicy) -> String {
    json!({
        "mode": policy.mode().name(),
        "allowList": policy.allow_list(),
        "batchedCount": policy.batched_count(),
    })
    .to_string()
}

fn banner_value(notification: &Notification) -> Value {
    // `deadline` is resolved against the service default so the shell does not
    // re-derive the `< 0 == default` rule; `null` means "never expires".
    json!({
        "id": notification.id,
        "appName": notification.app_name,
        "appIcon": notification.app_icon,
        "summary": notification.summary,
        "body": notification.body,
        "urgency": notification.urgency.name(),
        "createdAt": notification.created_at_ms,
        "deadline": notification.deadline_ms(crate::model::DEFAULT_TIMEOUT_MS),
        "actions": actions_value(&notification.actions),
    })
}

fn actions_value(actions: &[crate::model::Action]) -> Value {
    Value::Array(
        actions
            .iter()
            .map(|action| json!({ "key": action.key, "label": action.label }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NotifyRequest, Urgency};

    #[test]
    fn a_banner_encodes_the_shell_contract() {
        let mut queue = Queue::new();
        queue.notify(
            NotifyRequest {
                app_name: "Mail".to_owned(),
                summary: "New message".to_owned(),
                body: "From Ada".to_owned(),
                urgency: Urgency::Critical,
                expire_timeout_ms: 2500,
                ..NotifyRequest::default()
            },
            1000,
        );
        let value: Value = serde_json::from_str(&banners_json(&queue)).unwrap();
        assert_eq!(value[0]["id"], 1);
        assert_eq!(value[0]["appName"], "Mail");
        assert_eq!(value[0]["summary"], "New message");
        assert_eq!(value[0]["body"], "From Ada");
        assert_eq!(value[0]["urgency"], "critical");
        assert_eq!(value[0]["deadline"], 3500);
        assert!(value[0]["actions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn a_never_expiring_banner_encodes_a_null_deadline() {
        let mut queue = Queue::new();
        queue.notify(
            NotifyRequest {
                summary: "sticky".to_owned(),
                expire_timeout_ms: 0,
                ..NotifyRequest::default()
            },
            1000,
        );
        let value: Value = serde_json::from_str(&banners_json(&queue)).unwrap();
        assert!(value[0]["deadline"].is_null());
    }

    #[test]
    fn the_focus_policy_encodes_the_shell_contract() {
        let mut policy = FocusPolicy::new();
        assert_eq!(
            serde_json::from_str::<Value>(&focus_policy_json(&policy)).unwrap(),
            serde_json::json!({ "mode": "off", "allowList": [], "batchedCount": 0 })
        );

        policy.set_mode(crate::policy::FocusMode::Focus);
        policy.set_allow_list(vec!["Chat".to_owned()]);
        policy.note_batched(4);
        let value: Value = serde_json::from_str(&focus_policy_json(&policy)).unwrap();
        assert_eq!(value["mode"], "focus");
        assert_eq!(value["allowList"][0], "Chat");
        assert_eq!(value["batchedCount"], 1);
    }

    #[test]
    fn a_suppressed_history_entry_encodes_the_flag() {
        let mut queue = Queue::new();
        queue.set_focus_mode(crate::policy::FocusMode::Dnd);
        queue.notify(
            NotifyRequest {
                summary: "quiet".to_owned(),
                ..NotifyRequest::default()
            },
            1000,
        );
        let value: Value = serde_json::from_str(&history_json(&queue)).unwrap();
        assert_eq!(value[0]["suppressed"], true);
    }

    #[test]
    fn a_closed_notification_encodes_its_reason_in_history() {
        use crate::model::CloseReason;
        let mut queue = Queue::new();
        let id = queue.notify(
            NotifyRequest {
                summary: "gone".to_owned(),
                ..NotifyRequest::default()
            },
            1000,
        );
        queue.close(id, CloseReason::Dismissed, 1200);
        let value: Value = serde_json::from_str(&history_json(&queue)).unwrap();
        assert_eq!(value[0]["id"], 1);
        assert_eq!(value[0]["reason"], "dismissed");
        assert_eq!(value[0]["closedAt"], 1200);
    }
}
