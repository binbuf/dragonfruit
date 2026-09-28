// SPDX-License-Identifier: MIT
//! The Notifications and Focus snapshot the pane and tile render, decoded
//! from one raw read.
//!
//! The model owns only the projection a consumer should not repeat: it folds
//! the Focus policy, the active banners, and the recorded history into one
//! typed value, derives the observed per-app list, and computes the change
//! stream between two reads. It never re-derives the Focus admission rule —
//! `dragonfruit-notifications` owns that ([ADR 0058]); this model reads the
//! resulting state.
//!
//! [ADR 0058]: ../../../docs/design/adr/0058-focus-dnd-policy-semantics.md

use dragonfruit_notifications::FocusMode;

use crate::source::{NotificationRecord, NotificationsData};

/// The Focus/DND policy as a consumer reads it.
///
/// The three modes and their admission rule are owned by the notification
/// service; this is the projection of `FocusPolicy()` — the mode, the allow
/// list, and the suppressed batch count.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FocusSnapshot {
    /// The current mode (`off` / `focus` / `dnd`).
    pub mode: FocusMode,
    /// The per-app allow list, in the service's order.
    pub allow_list: Vec<String>,
    /// Notifications suppressed since the mode last returned to `off`.
    pub batched: u32,
}

impl FocusSnapshot {
    /// Whether the mode suppresses any banners at all.
    pub fn is_suppressing(&self) -> bool {
        self.mode.is_suppressing()
    }

    /// Whether Do Not Disturb (`dnd`) is the current mode.
    pub fn is_dnd(&self) -> bool {
        self.mode == FocusMode::Dnd
    }

    /// Whether the mode is plain `off`.
    pub fn is_off(&self) -> bool {
        self.mode == FocusMode::Off
    }

    /// The human label the tile and menu bar show.
    pub fn label(&self) -> &'static str {
        match self.mode {
            FocusMode::Off => "Off",
            FocusMode::Focus => "Focus",
            FocusMode::Dnd => "Do Not Disturb",
        }
    }

    /// The design-system glyph name for the mode.
    pub fn glyph(&self) -> &'static str {
        "focus"
    }

    /// Whether `app` is on the allow list (case-insensitive, trimmed), the
    /// per-app override the service honors in both suppressing modes.
    pub fn allows(&self, app: &str) -> bool {
        let app = app.trim();
        !app.is_empty()
            && self
                .allow_list
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(app))
    }
}

/// One observed application and its notification state, the row the
/// Notifications pane lists.
///
/// The app set is derived from the notifications the service has recorded
/// (active plus history); `allowed` reads the Focus allow list. An app that
/// has never notified does not appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppNotifications {
    /// The app name as the `Notify` call spelled it.
    pub name: String,
    /// Whether the app is on the Focus allow list.
    pub allowed: bool,
    /// How many recorded notifications name this app.
    pub notifications: usize,
}

impl AppNotifications {
    /// The pane's state subtext.
    pub fn status_label(&self) -> &'static str {
        if self.allowed {
            "Allowed"
        } else {
            "Default"
        }
    }
}

/// The Notifications and Focus snapshot a pane or tile renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationsSnapshot {
    /// The Focus/DND policy.
    pub focus: FocusSnapshot,
    /// The active banners, oldest first.
    pub active: Vec<NotificationRecord>,
    /// The recorded history, most recent first.
    pub history: Vec<NotificationRecord>,
}

impl NotificationsSnapshot {
    /// Build the snapshot from one raw read.
    pub fn from_data(data: &NotificationsData) -> Self {
        NotificationsSnapshot {
            focus: FocusSnapshot {
                mode: data.mode,
                allow_list: data.allow_list.clone(),
                batched: data.batched_count,
            },
            active: data.active.clone(),
            history: data.history.clone(),
        }
    }

    /// How many banners are on screen.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// How many notifications the history records.
    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    /// The newest active banner, when any.
    pub fn latest(&self) -> Option<&NotificationRecord> {
        self.active.last()
    }

    /// The Focus/DND policy.
    pub fn focus(&self) -> &FocusSnapshot {
        &self.focus
    }

    /// Whether any suppression is active.
    pub fn suppressing(&self) -> bool {
        self.focus.is_suppressing()
    }

    /// The observed applications, sorted case-insensitively by name.
    ///
    /// An app appears once if any active banner or history entry names it;
    /// `notifications` counts the history entries (the durable record). The
    /// allow-list state comes from the Focus policy, so one read fills both
    /// the policy control and the per-app list.
    pub fn apps(&self) -> Vec<AppNotifications> {
        let mut names: Vec<&str> = self
            .active
            .iter()
            .chain(self.history.iter())
            .map(|entry| entry.app_name.as_str())
            .filter(|name| !name.trim().is_empty())
            .collect();
        names.sort_by_key(|name| name.to_ascii_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

        names
            .into_iter()
            .map(|name| AppNotifications {
                name: name.to_owned(),
                allowed: self.focus.allows(name),
                notifications: self
                    .history
                    .iter()
                    .filter(|entry| entry.app_name.eq_ignore_ascii_case(name))
                    .count(),
            })
            .collect()
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the Focus mode, the allow list, the suppressed batch, then the
    /// notification counts.
    ///
    /// This is the "event" half of the adapter: the host diffs two reads to
    /// learn what moved without polling each field. A notification body or an
    /// urgency change is not diffed — the pane re-reads the lists from the
    /// snapshot itself.
    pub fn changes(&self, previous: &NotificationsSnapshot) -> Vec<NotificationsChange> {
        let mut changes = Vec::new();
        if self.focus.mode != previous.focus.mode {
            changes.push(NotificationsChange::FocusModeChanged {
                from: previous.focus.mode,
                to: self.focus.mode,
            });
        }
        if self.focus.allow_list != previous.focus.allow_list {
            changes.push(NotificationsChange::AllowListChanged {
                from: previous.focus.allow_list.clone(),
                to: self.focus.allow_list.clone(),
            });
        }
        if self.focus.batched != previous.focus.batched {
            changes.push(NotificationsChange::BatchedChanged {
                from: previous.focus.batched,
                to: self.focus.batched,
            });
        }
        if self.active_count() != previous.active_count() {
            changes.push(NotificationsChange::ActiveChanged {
                from: previous.active_count(),
                to: self.active_count(),
            });
        }
        if self.history_count() != previous.history_count() {
            changes.push(NotificationsChange::HistoryChanged {
                from: previous.history_count(),
                to: self.history_count(),
            });
        }
        changes
    }
}

/// A change between two notification snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsChange {
    /// The Focus/DND mode moved.
    FocusModeChanged {
        /// The previous mode.
        from: FocusMode,
        /// The new mode.
        to: FocusMode,
    },
    /// The per-app allow list moved.
    AllowListChanged {
        /// The previous allow list.
        from: Vec<String>,
        /// The new allow list.
        to: Vec<String>,
    },
    /// The suppressed batch count moved.
    BatchedChanged {
        /// The previous count.
        from: u32,
        /// The new count.
        to: u32,
    },
    /// The number of active banners moved.
    ActiveChanged {
        /// The previous count.
        from: usize,
        /// The new count.
        to: usize,
    },
    /// The number of recorded history entries moved (including roll-off at
    /// the history capacity).
    HistoryChanged {
        /// The previous count.
        from: usize,
        /// The new count.
        to: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_notifications::Urgency;

    fn record(id: u32, app: &str, summary: &str, suppressed: bool) -> NotificationRecord {
        NotificationRecord {
            id,
            app_name: app.to_owned(),
            summary: summary.to_owned(),
            urgency: Urgency::Normal,
            suppressed,
            created_at_ms: 1000 + u64::from(id),
        }
    }

    fn data(mode: FocusMode) -> NotificationsData {
        NotificationsData {
            mode,
            allow_list: vec!["Pager".to_owned()],
            batched_count: 0,
            active: vec![record(1, "Mail", "New message", false)],
            history: vec![record(2, "Pager", "on-call", true)],
        }
    }

    #[test]
    fn the_snapshot_projects_policy_and_notifications() {
        let snapshot = NotificationsSnapshot::from_data(&data(FocusMode::Focus));
        assert_eq!(snapshot.focus.mode, FocusMode::Focus);
        assert_eq!(snapshot.focus.label(), "Focus");
        assert!(snapshot.focus.is_suppressing());
        assert!(!snapshot.focus.is_dnd());
        assert_eq!(snapshot.focus.glyph(), "focus");
        assert_eq!(snapshot.active_count(), 1);
        assert_eq!(snapshot.history_count(), 1);
        assert_eq!(snapshot.latest().unwrap().summary, "New message");
    }

    #[test]
    fn off_focus_has_its_own_label_and_does_not_suppress() {
        let snapshot = NotificationsSnapshot::from_data(&NotificationsData::default());
        assert!(snapshot.focus.is_off());
        assert!(!snapshot.suppressing());
        assert_eq!(snapshot.focus.label(), "Off");
        assert!(snapshot.active.is_empty());
        assert!(snapshot.latest().is_none());
    }

    #[test]
    fn the_allow_list_is_case_insensitive() {
        let focus = FocusSnapshot {
            mode: FocusMode::Dnd,
            allow_list: vec!["Pager".to_owned()],
            batched: 0,
        };
        assert!(focus.allows("pager"));
        assert!(focus.allows(" Pager "));
        assert!(!focus.allows("Mail"));
        assert!(!focus.allows("   "));
    }

    #[test]
    fn apps_are_derived_from_active_and_history_and_sorted() {
        let mut raw = data(FocusMode::Focus);
        raw.history.push(record(3, "chat", "hello", false));
        raw.history.push(record(4, "Pager", "again", false));
        let snapshot = NotificationsSnapshot::from_data(&raw);
        let apps = snapshot.apps();
        // Case-insensitive sort; distinct case-insensitive names dedupe.
        assert_eq!(
            apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
            vec!["chat", "Mail", "Pager"]
        );
        let pager = apps.iter().find(|app| app.name == "Pager").unwrap();
        assert!(pager.allowed);
        assert_eq!(pager.status_label(), "Allowed");
        assert_eq!(pager.notifications, 2);
        assert_eq!(pager.notifications, 2);

        let mail = apps.iter().find(|app| app.name == "Mail").unwrap();
        assert!(!mail.allowed);
        assert_eq!(mail.status_label(), "Default");
        // Mail has one active banner but no history entry.
        assert_eq!(mail.notifications, 0);
    }

    #[test]
    fn focus_moves_are_changes() {
        let previous = NotificationsSnapshot::from_data(&data(FocusMode::Off));
        let next = NotificationsSnapshot::from_data(&data(FocusMode::Dnd));
        assert_eq!(
            next.changes(&previous),
            vec![NotificationsChange::FocusModeChanged {
                from: FocusMode::Off,
                to: FocusMode::Dnd,
            }]
        );
    }

    #[test]
    fn allow_list_batch_and_count_moves_are_changes() {
        let previous = NotificationsSnapshot::from_data(&data(FocusMode::Focus));

        let mut raw = data(FocusMode::Focus);
        raw.allow_list = vec!["Pager".to_owned(), "Chat".to_owned()];
        raw.batched_count = 3;
        raw.active.clear();
        raw.history.push(record(9, "Files", "copied", false));
        let next = NotificationsSnapshot::from_data(&raw);

        let changes = next.changes(&previous);
        assert!(changes.contains(&NotificationsChange::AllowListChanged {
            from: vec!["Pager".to_owned()],
            to: vec!["Pager".to_owned(), "Chat".to_owned()],
        }));
        assert!(changes.contains(&NotificationsChange::BatchedChanged { from: 0, to: 3 }));
        assert!(changes.contains(&NotificationsChange::ActiveChanged { from: 1, to: 0 }));
        assert!(changes.contains(&NotificationsChange::HistoryChanged { from: 1, to: 2 }));
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = NotificationsSnapshot::from_data(&data(FocusMode::Focus));
        assert!(snapshot.changes(&snapshot).is_empty());
    }
}
