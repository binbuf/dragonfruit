// SPDX-License-Identifier: MIT
//! The Focus / Do Not Disturb policy (T-11.2a).
//!
//! The service owns one policy state ([04-shell.md](../../docs/design/04-shell.md),
//! "Focus/DND state lives with the notification service so Control Center and
//! the menu bar share one source of truth"). This module is that state and the
//! admission rule; the queue asks it whether a `Notify` becomes a banner. The
//! semantics are frozen here so the menu bar (T-11.2b), the Control Center
//! (T-11.3b), and the Settings Notifications pane all bind to one rule.
//!
//! See [adr/0058](../../docs/design/adr/0058-focus-dnd-policy-semantics.md).
//!
//! # The documented rules
//!
//! | Mode    | Banners                                                    |
//! |---------|------------------------------------------------------------|
//! | `off`   | every notification                                          |
//! | `focus` | allow-listed apps *and* `critical` urgency; the rest batched|
//! | `dnd`   | allow-listed apps only; the rest (including `critical`) batched |
//!
//! A suppressed notification is never dropped: the queue records it in the
//! history with `suppressed = true` and counts it in the current batch. The
//! batch grows while the mode is suppressing and is cleared when the mode
//! returns to `off`, so the shell can summarize "N notifications while Focus
//! was on" without the service ever emitting a synthetic banner.

use crate::model::Urgency;

/// The Focus/DND mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusMode {
    /// Notifications banner normally.
    #[default]
    Off,
    /// Focus: allow-listed apps and critical alerts banner; the rest batched.
    Focus,
    /// Do Not Disturb: only allow-listed apps banner; the rest batched.
    Dnd,
}

impl FocusMode {
    /// The lowercase name on the wire and in the JSON view.
    pub fn name(self) -> &'static str {
        match self {
            FocusMode::Off => "off",
            FocusMode::Focus => "focus",
            FocusMode::Dnd => "dnd",
        }
    }

    /// Parse the wire name. Unknown names are rejected (the caller keeps the
    /// previous mode).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Some(FocusMode::Off),
            "focus" => Some(FocusMode::Focus),
            "dnd" | "do-not-disturb" => Some(FocusMode::Dnd),
            _ => None,
        }
    }

    /// Whether this mode suppresses any banners at all.
    pub fn is_suppressing(self) -> bool {
        self != FocusMode::Off
    }
}

/// The policy state: the mode, the per-app allow list, and the batch of
/// notification ids suppressed since the mode last returned to `off`.
#[derive(Debug, Clone, Default)]
pub struct FocusPolicy {
    mode: FocusMode,
    allow_list: Vec<String>,
    batched: Vec<u32>,
}

impl FocusPolicy {
    /// A new `off` policy with no allow list.
    pub fn new() -> Self {
        FocusPolicy::default()
    }

    /// The current mode.
    pub fn mode(&self) -> FocusMode {
        self.mode
    }

    /// Set the mode. Returning to `off` clears the batch; changing between
    /// suppressing modes keeps it, because the suppressed set has not been
    /// shown yet.
    pub fn set_mode(&mut self, mode: FocusMode) {
        if mode == FocusMode::Off {
            self.batched.clear();
        }
        self.mode = mode;
    }

    /// The per-app allow list, in insertion order.
    pub fn allow_list(&self) -> &[String] {
        &self.allow_list
    }

    /// Replace the allow list. Entries are trimmed, empties dropped, and
    /// duplicates removed case-insensitively (first occurrence wins).
    pub fn set_allow_list(&mut self, apps: impl IntoIterator<Item = String>) {
        let mut normalized: Vec<String> = Vec::new();
        for app in apps {
            let trimmed = app.trim();
            if trimmed.is_empty() {
                continue;
            }
            if normalized
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(trimmed))
            {
                continue;
            }
            normalized.push(trimmed.to_owned());
        }
        self.allow_list = normalized;
    }

    /// Whether `app_name` is on the allow list (case-insensitive, trimmed).
    pub fn is_allowed(&self, app_name: &str) -> bool {
        let app_name = app_name.trim();
        !app_name.is_empty()
            && self
                .allow_list
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(app_name))
    }

    /// The admission rule: whether a notification from `app_name` with
    /// `urgency` becomes a banner under the current mode.
    pub fn admits(&self, app_name: &str, urgency: Urgency) -> bool {
        match self.mode {
            FocusMode::Off => true,
            FocusMode::Focus => urgency == Urgency::Critical || self.is_allowed(app_name),
            FocusMode::Dnd => self.is_allowed(app_name),
        }
    }

    /// The ids suppressed in the current batch, oldest first.
    pub fn batched(&self) -> &[u32] {
        &self.batched
    }

    /// How many notifications the current suppression has batched.
    pub fn batched_count(&self) -> usize {
        self.batched.len()
    }

    /// Record a suppressed notification id in the batch. Ids are unique, so a
    /// repeated call for the same id is a no-op.
    pub fn note_batched(&mut self, id: u32) {
        if !self.batched.contains(&id) {
            self.batched.push(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_admits_every_urgency_and_app() {
        let policy = FocusPolicy::new();
        assert_eq!(policy.mode(), FocusMode::Off);
        assert!(!policy.mode().is_suppressing());
        assert!(policy.admits("Mail", Urgency::Low));
        assert!(policy.admits("Mail", Urgency::Normal));
        assert!(policy.admits("Mail", Urgency::Critical));
    }

    #[test]
    fn focus_admits_critical_and_allowed_apps_only() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Focus);
        policy.set_allow_list(vec!["Chat".to_owned()]);

        assert!(policy.admits("Mail", Urgency::Critical));
        assert!(policy.admits("Chat", Urgency::Normal));
        assert!(!policy.admits("Mail", Urgency::Normal));
        assert!(!policy.admits("Mail", Urgency::Low));
    }

    #[test]
    fn dnd_admits_only_allowed_apps_and_silences_critical() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Dnd);
        policy.set_allow_list(vec!["Pager".to_owned()]);

        assert!(policy.admits("Pager", Urgency::Critical));
        assert!(!policy.admits("Mail", Urgency::Critical));
        assert!(!policy.admits("Mail", Urgency::Normal));
    }

    #[test]
    fn the_allow_list_is_normalized_and_case_insensitive() {
        let mut policy = FocusPolicy::new();
        policy.set_allow_list(vec![
            "  Chat ".to_owned(),
            "chat".to_owned(),
            String::new(),
            "Mail".to_owned(),
        ]);
        assert_eq!(policy.allow_list(), &["Chat".to_owned(), "Mail".to_owned()]);
        assert!(policy.is_allowed("cHAT"));
        assert!(policy.is_allowed(" Mail "));
        assert!(!policy.is_allowed("Files"));
        assert!(!policy.is_allowed("   "));
    }

    #[test]
    fn mode_names_round_trip_and_unknown_names_are_rejected() {
        for mode in [FocusMode::Off, FocusMode::Focus, FocusMode::Dnd] {
            assert_eq!(FocusMode::parse(mode.name()), Some(mode));
        }
        assert_eq!(FocusMode::parse(" DND "), Some(FocusMode::Dnd));
        assert_eq!(FocusMode::parse("do-not-disturb"), Some(FocusMode::Dnd));
        assert_eq!(FocusMode::parse("vacation"), None);
    }

    #[test]
    fn the_batch_accumulates_while_suppressing_and_clears_on_off() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Focus);
        policy.note_batched(7);
        policy.note_batched(7);
        policy.note_batched(8);
        assert_eq!(policy.batched(), &[7, 8]);
        assert_eq!(policy.batched_count(), 2);

        // Focus -> DND keeps the batch (nothing was shown yet).
        policy.set_mode(FocusMode::Dnd);
        assert_eq!(policy.batched_count(), 2);

        // Back to off: the batch is delivered/cleared.
        policy.set_mode(FocusMode::Off);
        assert_eq!(policy.batched_count(), 0);
        assert!(policy.batched().is_empty());
    }
}
