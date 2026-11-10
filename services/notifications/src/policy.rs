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

/// The stable id of the synthetic profile a manual edit produces.
pub const CUSTOM_PROFILE_ID: &str = "custom";

/// One named Focus presentation profile (T234).
///
/// A profile is **not** a second policy: it carries a [`FocusMode`] and the
/// app/person allow lists that the one admission rule already consumes.
/// Selecting a profile installs those values into the policy; editing a list
/// while the profile is active edits that profile, so switching away and back
/// keeps the user's choices. `custom` is the synthetic profile a manual mode
/// change or list edit selects when no named profile matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusProfile {
    /// The stable id on the wire (`dnd`, `personal`, `work`, `sleep`,
    /// `custom`).
    pub id: String,
    /// The user-facing label.
    pub name: String,
    /// The admission mode this profile selects.
    pub mode: FocusMode,
    /// The apps allowed while this profile is active.
    pub apps: Vec<String>,
    /// The people (senders) allowed while this profile is active.
    pub people: Vec<String>,
}

impl FocusProfile {
    /// A profile with no allow lists.
    pub fn new(id: &str, name: &str, mode: FocusMode) -> Self {
        FocusProfile {
            id: id.to_owned(),
            name: name.to_owned(),
            mode,
            apps: Vec::new(),
            people: Vec::new(),
        }
    }

    /// The built-in profile catalog (the service's default presentation).
    pub fn catalog() -> Vec<FocusProfile> {
        default_profiles()
    }
}

/// The built-in profile catalog. The order is the presentation order: the
/// four named profiles then the synthetic custom entry.
pub fn default_profiles() -> Vec<FocusProfile> {
    vec![
        FocusProfile::new("dnd", "Do Not Disturb", FocusMode::Dnd),
        FocusProfile::new("personal", "Personal", FocusMode::Focus),
        FocusProfile::new("work", "Work", FocusMode::Focus),
        FocusProfile::new("sleep", "Sleep", FocusMode::Dnd),
        FocusProfile::new(CUSTOM_PROFILE_ID, "Custom", FocusMode::Focus),
    ]
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

    /// The user-facing mode label (the fallback when no named profile is
    /// active).
    pub fn label(self) -> &'static str {
        match self {
            FocusMode::Off => "Off",
            FocusMode::Focus => "Focus",
            FocusMode::Dnd => "Do Not Disturb",
        }
    }
}

/// The policy state: the mode, the allow lists, the named profiles, and the
/// batch of notification ids suppressed since the mode last returned to `off`.
#[derive(Debug, Clone)]
pub struct FocusPolicy {
    mode: FocusMode,
    allow_list: Vec<String>,
    allowed_people: Vec<String>,
    profiles: Vec<FocusProfile>,
    active_profile: Option<String>,
    app_preferences: BTreeMap<String, AppPreferenceEntry>,
    batched: Vec<u32>,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        FocusPolicy {
            mode: FocusMode::Off,
            allow_list: Vec::new(),
            allowed_people: Vec::new(),
            profiles: default_profiles(),
            active_profile: None,
            app_preferences: BTreeMap::new(),
            batched: Vec::new(),
        }
    }
}

impl FocusPolicy {
    /// A new `off` policy with the built-in profile catalog.
    pub fn new() -> Self {
        FocusPolicy::default()
    }

    /// The current mode.
    pub fn mode(&self) -> FocusMode {
        self.mode
    }

    /// The named profiles, in presentation order (the synthetic `custom`
    /// entry last).
    pub fn profiles(&self) -> &[FocusProfile] {
        &self.profiles
    }

    /// The active profile id, when a named profile is selected. `None` means
    /// `off` (or a fresh policy); `Some("custom")` means a manual suppressing
    /// state.
    pub fn active_profile(&self) -> Option<&str> {
        self.active_profile.as_deref()
    }

    /// The active profile's user-facing label, or `None` when off.
    pub fn active_profile_label(&self) -> Option<&str> {
        let id = self.active_profile.as_deref()?;
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .map(|profile| profile.name.as_str())
    }

    /// The presentation label the menu bar and Control Center show: a named
    /// profile when one is selected, otherwise the mode label. `custom` is a
    /// synthetic marker, not a name a user picked, so it reads as the mode.
    pub fn presentation_label(&self) -> Option<&str> {
        let id = self.active_profile.as_deref()?;
        if id == CUSTOM_PROFILE_ID {
            return Some(self.mode.label());
        }
        self.active_profile_label()
    }

    /// Select a named profile by its stable id. The profile's mode and allow
    /// lists become the policy's; unknown ids are rejected and the previous
    /// state is kept. Selecting `off` is a mode change, not a profile.
    pub fn set_active_profile(&mut self, id: &str) -> bool {
        let id = id.trim();
        let Some(profile) = self
            .profiles
            .iter()
            .find(|profile| profile.id.eq_ignore_ascii_case(id))
            .cloned()
        else {
            return false;
        };
        if profile.mode == FocusMode::Off {
            return false;
        }
        if self.mode == FocusMode::Off {
            self.batched.clear();
        }
        self.mode = profile.mode;
        self.allow_list = profile.apps;
        self.allowed_people = profile.people;
        self.active_profile = Some(profile.id);
        true
    }

    /// Set the mode. Returning to `off` clears the batch and the active
    /// profile; a suppressing mode with no matching profile selects `custom`.
    /// Changing between suppressing modes keeps the batch, because the
    /// suppressed set has not been shown yet.
    pub fn set_mode(&mut self, mode: FocusMode) {
        if mode == FocusMode::Off {
            self.batched.clear();
            self.active_profile = None;
            self.mode = mode;
            return;
        }
        let matches_active = self
            .active_profile
            .as_deref()
            .and_then(|id| self.profiles.iter().find(|profile| profile.id == id))
            .is_some_and(|profile| profile.mode == mode);
        if !matches_active {
            self.active_profile = Some(CUSTOM_PROFILE_ID.to_owned());
            if let Some(custom) = self
                .profiles
                .iter_mut()
                .find(|profile| profile.id == CUSTOM_PROFILE_ID)
            {
                custom.mode = mode;
                custom.apps = self.allow_list.clone();
                custom.people = self.allowed_people.clone();
            }
        }
        self.mode = mode;
    }

    /// The per-app allow list, in insertion order.
    pub fn allow_list(&self) -> &[String] {
        &self.allow_list
    }

    /// Replace the allow list. Entries are trimmed, empties dropped, and
    /// duplicates removed case-insensitively (first occurrence wins). When a
    /// named profile is active its stored list follows the edit.
    pub fn set_allow_list(&mut self, apps: impl IntoIterator<Item = String>) {
        self.allow_list = normalize_names(apps);
        let current = self.allow_list.clone();
        if let Some(profile) = self.active_profile_mut() {
            profile.apps = current;
        }
    }

    /// Whether `app_name` is on the allow list (case-insensitive, trimmed).
    pub fn is_allowed(&self, app_name: &str) -> bool {
        contains_name(&self.allow_list, app_name)
    }

    /// The allowed-people list (the optional notification `sender` hint), in
    /// insertion order.
    pub fn allowed_people(&self) -> &[String] {
        &self.allowed_people
    }

    /// Replace the allowed-people list, normalized like the app list.
    pub fn set_allowed_people(&mut self, people: impl IntoIterator<Item = String>) {
        self.allowed_people = normalize_names(people);
        let current = self.allowed_people.clone();
        if let Some(profile) = self.active_profile_mut() {
            profile.people = current;
        }
    }

    /// Whether `sender` is on the allowed-people list.
    pub fn is_person_allowed(&self, sender: &str) -> bool {
        contains_name(&self.allowed_people, sender)
    }

    fn active_profile_mut(&mut self) -> Option<&mut FocusProfile> {
        let id = self.active_profile.clone()?;
        self.profiles.iter_mut().find(|profile| profile.id == id)
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
    /// `urgency` becomes a banner under the current mode. `sender` is the
    /// optional person identity a notification carried (T234); an
    /// allow-listed person is admitted exactly like an allow-listed app.
    pub fn admits(&self, app_name: &str, urgency: Urgency, sender: Option<&str>) -> bool {
        let preference = self.app_preference(app_name);
        if !preference.allowed || preference.style == NotificationStyle::None {
            return false;
        }
        let person_allowed = sender.is_some_and(|sender| self.is_person_allowed(sender));
        match self.mode {
            FocusMode::Off => true,
            FocusMode::Focus => {
                urgency == Urgency::Critical || self.is_allowed(app_name) || person_allowed
            }
            FocusMode::Dnd => self.is_allowed(app_name) || person_allowed,
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

/// Trim, drop empties, and deduplicate case-insensitively (first spelling
/// wins).
fn normalize_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    for name in names {
        let trimmed = name.trim();
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
    normalized
}

/// Whether `name` is in a normalized list (case-insensitive, trimmed).
fn contains_name(names: &[String], name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && names.iter().any(|entry| entry.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_admits_every_urgency_and_app() {
        let policy = FocusPolicy::new();
        assert_eq!(policy.mode(), FocusMode::Off);
        assert!(!policy.mode().is_suppressing());
        assert!(policy.admits("Mail", Urgency::Low, None));
        assert!(policy.admits("Mail", Urgency::Normal, None));
        assert!(policy.admits("Mail", Urgency::Critical, None));
    }

    #[test]
    fn focus_admits_critical_and_allowed_apps_only() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Focus);
        policy.set_allow_list(vec!["Chat".to_owned()]);

        assert!(policy.admits("Mail", Urgency::Critical, None));
        assert!(policy.admits("Chat", Urgency::Normal, None));
        assert!(!policy.admits("Mail", Urgency::Normal, None));
        assert!(!policy.admits("Mail", Urgency::Low, None));
    }

    #[test]
    fn dnd_admits_only_allowed_apps_and_silences_critical() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Dnd);
        policy.set_allow_list(vec!["Pager".to_owned()]);

        assert!(policy.admits("Pager", Urgency::Critical, None));
        assert!(!policy.admits("Mail", Urgency::Critical, None));
        assert!(!policy.admits("Mail", Urgency::Normal, None));
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
        assert!(!policy.admits("Mail", Urgency::Critical, None));
        assert!(!policy.admits("mail", Urgency::Normal, None));

        policy.set_app_preference(
            "Chat",
            AppPreference {
                style: NotificationStyle::None,
                ..AppPreference::DEFAULT
            },
        );
        assert!(!policy.admits("Chat", Urgency::Critical, None));
        // An untouched app still banners in `off`.
        assert!(policy.admits("Files", Urgency::Normal, None));
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

    #[test]
    fn the_profile_catalog_names_personal_work_sleep_and_custom() {
        let policy = FocusPolicy::new();
        let ids: Vec<&str> = policy
            .profiles()
            .iter()
            .map(|profile| profile.id.as_str())
            .collect();
        assert_eq!(ids, vec!["dnd", "personal", "work", "sleep", "custom"]);
        assert!(policy.active_profile().is_none());
        assert!(policy.active_profile_label().is_none());
        assert_eq!(
            policy
                .profiles()
                .iter()
                .find(|profile| profile.id == "sleep")
                .unwrap()
                .mode,
            FocusMode::Dnd
        );
    }

    #[test]
    fn selecting_a_profile_installs_its_mode_and_lists() {
        let mut policy = FocusPolicy::new();
        // Personal keeps its own allow list across a round trip.
        assert!(policy.set_active_profile("personal"));
        assert_eq!(policy.mode(), FocusMode::Focus);
        assert_eq!(policy.active_profile(), Some("personal"));
        assert_eq!(policy.active_profile_label(), Some("Personal"));
        policy.set_allow_list(vec!["Chat".to_owned()]);
        policy.set_allowed_people(vec!["Ada".to_owned()]);

        assert!(policy.set_active_profile("sleep"));
        assert_eq!(policy.mode(), FocusMode::Dnd);
        assert!(policy.allow_list().is_empty());
        assert!(policy.allowed_people().is_empty());

        assert!(policy.set_active_profile("personal"));
        assert_eq!(policy.allow_list(), &["Chat".to_owned()]);
        assert_eq!(policy.allowed_people(), &["Ada".to_owned()]);

        // Unknown ids are rejected and never change the state.
        assert!(!policy.set_active_profile("vacation"));
        assert_eq!(policy.active_profile(), Some("personal"));
    }

    #[test]
    fn a_manual_mode_change_selects_custom_and_keeps_the_lists() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Dnd);
        assert_eq!(policy.active_profile(), Some(CUSTOM_PROFILE_ID));
        assert_eq!(policy.active_profile_label(), Some("Custom"));
        assert_eq!(policy.mode(), FocusMode::Dnd);

        // Selecting a named profile leaves custom behind.
        assert!(policy.set_active_profile("work"));
        assert_eq!(policy.active_profile(), Some("work"));
        // Turning Focus off clears the profile; the next manual on is custom.
        policy.set_mode(FocusMode::Off);
        assert!(policy.active_profile().is_none());
        policy.set_mode(FocusMode::Focus);
        assert_eq!(policy.active_profile(), Some(CUSTOM_PROFILE_ID));
    }

    #[test]
    fn an_allowed_person_is_admitted_in_both_suppressing_modes() {
        let mut policy = FocusPolicy::new();
        policy.set_mode(FocusMode::Focus);
        policy.set_allowed_people(vec![" Ada ".to_owned(), "ada".to_owned()]);
        assert_eq!(policy.allowed_people(), &["Ada".to_owned()]);
        assert!(policy.is_person_allowed("ada"));

        // A non-critical message from an allowed person banners under Focus.
        assert!(policy.admits("Chat", Urgency::Normal, Some("Ada")));
        assert!(!policy.admits("Chat", Urgency::Normal, Some("Grace")));
        // The app list still works independently.
        assert!(!policy.admits("Mail", Urgency::Normal, None));
        policy.set_allow_list(vec!["Mail".to_owned()]);
        assert!(policy.admits("Mail", Urgency::Normal, None));

        // DND silences critical alerts but honors the person list.
        policy.set_mode(FocusMode::Dnd);
        assert!(policy.admits("Chat", Urgency::Critical, Some("Ada")));
        assert!(!policy.admits("Chat", Urgency::Critical, Some("Grace")));
        // An empty sender never matches.
        assert!(!policy.admits("Chat", Urgency::Critical, Some("   ")));
        assert!(!policy.admits("Chat", Urgency::Critical, None));
    }
}
