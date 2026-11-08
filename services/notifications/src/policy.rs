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

use std::collections::BTreeMap;

use crate::model::Urgency;

/// How a per-app notification is presented (T229a).
///
/// The service owns the preference so the pane and the shell read one source.
/// `Banner` is the default transient banner, `Alert` is a persistent banner
/// (the shell may keep it until dismissed), and `None` suppresses the banner
/// while still recording the notification in the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationStyle {
    /// A transient banner.
    #[default]
    Banner,
    /// A persistent alert that waits for dismissal.
    Alert,
    /// No banner; the notification is recorded in the history only.
    None,
}

impl NotificationStyle {
    /// The lowercase name on the wire and in the JSON view.
    pub fn name(self) -> &'static str {
        match self {
            NotificationStyle::Banner => "banner",
            NotificationStyle::Alert => "alert",
            NotificationStyle::None => "none",
        }
    }

    /// Parse the wire name. Unknown names are rejected.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "banner" => Some(NotificationStyle::Banner),
            "alert" => Some(NotificationStyle::Alert),
            "none" => Some(NotificationStyle::None),
            _ => None,
        }
    }

    /// The human label the Settings pane shows.
    pub fn label(self) -> &'static str {
        match self {
            NotificationStyle::Banner => "Banners",
            NotificationStyle::Alert => "Alerts",
            NotificationStyle::None => "None",
        }
    }
}

/// One app's durable notification preference (T229a).
///
/// `allowed` is the per-app mute ("Allow notifications"); `style`, `sound`
/// and `badge` are the presentation choices the pane edits. The Focus allow
/// list is separate: it only governs whether the app is admitted while Focus
/// or Do Not Disturb is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppPreference {
    /// Whether the app may raise notifications at all.
    pub allowed: bool,
    /// How a notification is presented.
    pub style: NotificationStyle,
    /// Whether the banner plays a sound.
    pub sound: bool,
    /// Whether the app contributes a Dock badge.
    pub badge: bool,
}

impl Default for AppPreference {
    fn default() -> Self {
        AppPreference {
            allowed: true,
            style: NotificationStyle::Banner,
            sound: true,
            badge: true,
        }
    }
}

impl AppPreference {
    /// The preference for an app the user has not customized.
    pub const DEFAULT: AppPreference = AppPreference {
        allowed: true,
        style: NotificationStyle::Banner,
        sound: true,
        badge: true,
    };

    /// Whether this is the default preference (the store drops it).
    pub fn is_default(self) -> bool {
        self == AppPreference::DEFAULT
    }
}

/// One stored per-app preference, with the app name as the user spelled it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPreferenceEntry {
    /// The app name the `Notify` call carried (first spelling wins).
    pub name: String,
    /// The preference.
    pub preference: AppPreference,
}

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
    app_preferences: BTreeMap<String, AppPreferenceEntry>,
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

    /// The durable preference for `app_name`, defaulting to
    /// [`AppPreference::DEFAULT`].
    pub fn app_preference(&self, app_name: &str) -> AppPreference {
        let key = app_name.trim().to_ascii_lowercase();
        self.app_preferences
            .get(&key)
            .map(|entry| entry.preference)
            .unwrap_or(AppPreference::DEFAULT)
    }

    /// Every stored per-app preference, sorted by the normalized app name.
    pub fn app_preferences(&self) -> impl Iterator<Item = &AppPreferenceEntry> {
        self.app_preferences.values()
    }

    /// Store one app's preference. A default preference is dropped from the
    /// store so the JSON view carries only customizations. Empty app names
    /// are ignored; the first spelling of an app name is kept.
    pub fn set_app_preference(&mut self, app_name: &str, preference: AppPreference) {
        let name = app_name.trim();
        if name.is_empty() {
            return;
        }
        let key = name.to_ascii_lowercase();
        if preference.is_default() {
            self.app_preferences.remove(&key);
            return;
        }
        match self.app_preferences.get_mut(&key) {
            Some(entry) => entry.preference = preference,
            None => {
                self.app_preferences.insert(
                    key,
                    AppPreferenceEntry {
                        name: name.to_owned(),
                        preference,
                    },
                );
            }
        }
    }

    /// The admission rule: whether a notification from `app_name` with
    /// `urgency` becomes a banner under the current mode.
    pub fn admits(&self, app_name: &str, urgency: Urgency) -> bool {
        let preference = self.app_preference(app_name);
        if !preference.allowed || preference.style == NotificationStyle::None {
            return false;
        }
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
    fn app_preferences_default_and_round_trip() {
        let mut policy = FocusPolicy::new();
        assert_eq!(policy.app_preference("Mail"), AppPreference::DEFAULT);
        assert_eq!(policy.app_preferences().count(), 0);

        let pref = AppPreference {
            allowed: false,
            style: NotificationStyle::Alert,
            sound: false,
            badge: true,
        };
        policy.set_app_preference("  Mail ", pref);
        assert_eq!(policy.app_preference("mail"), pref);
        assert_eq!(policy.app_preferences().count(), 1);
        let entry = policy.app_preferences().next().unwrap();
        assert_eq!(entry.name, "Mail");

        // The first spelling wins across later updates.
        policy.set_app_preference("MAIL", AppPreference::DEFAULT);
        assert_eq!(policy.app_preferences().count(), 0);
        assert_eq!(policy.app_preference("mail"), AppPreference::DEFAULT);
    }

    #[test]
    fn a_muted_or_hidden_app_never_banners() {
        let mut policy = FocusPolicy::new();
        policy.set_app_preference(
            "Mail",
            AppPreference {
                allowed: false,
                ..AppPreference::DEFAULT
            },
        );
        assert!(!policy.admits("Mail", Urgency::Critical));
        assert!(!policy.admits("mail", Urgency::Normal));

        policy.set_app_preference(
            "Chat",
            AppPreference {
                style: NotificationStyle::None,
                ..AppPreference::DEFAULT
            },
        );
        assert!(!policy.admits("Chat", Urgency::Critical));
        // An untouched app still banners in `off`.
        assert!(policy.admits("Files", Urgency::Normal));
    }

    #[test]
    fn notification_style_names_round_trip() {
        for style in [
            NotificationStyle::Banner,
            NotificationStyle::Alert,
            NotificationStyle::None,
        ] {
            assert_eq!(NotificationStyle::parse(style.name()), Some(style));
            assert!(!style.label().is_empty());
        }
        assert_eq!(NotificationStyle::parse("vacation"), None);
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
