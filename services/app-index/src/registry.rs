// SPDX-License-Identifier: MIT
//! The launch registry and app recency (T-14.1b).
//!
//! The compositor owns windows; the shell sees they appear and disappear and
//! forwards that activity here. The registry is deliberately pure and
//! std-only: it is fed an already-resolved app key (a desktop id when the
//! identity resolves, otherwise the raw `app_id`/`WM_CLASS` string) and it
//! tracks
//!
//! * which apps are running and how many windows each has;
//! * the most-recently-active first recency order; and
//! * an activity event log (`app_running` / `app_exited` / `focused`).
//!
//! Recency is ordered by the registry itself, not by timestamps: two events in
//! the same millisecond still order correctly, and the order is exactly the
//! sequence the consumers rendered. Timestamps are carried for the UI.
//!
//! Nothing here polls: a window map/unmap or a focus change is one call.

use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

/// How many app keys the recency list retains.
pub const RECENT_CAPACITY: usize = 32;
/// How many activity events the log retains.
pub const ACTIVITY_CAPACITY: usize = 256;

/// Milliseconds since the Unix epoch, saturating at 0 for a pre-epoch clock.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

/// One running application in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningApp {
    /// The registry key: a desktop id when resolved, else the raw identity.
    pub key: String,
    /// The display name (the desktop `Name`, or the raw key on a miss).
    pub name: String,
    /// The desktop `Icon` field (empty on a miss).
    pub icon: String,
    /// Open windows for this app.
    pub windows: u32,
    /// When the app first appeared in this run.
    pub since_ms: u64,
    /// When the app was last the active window.
    pub active_ms: u64,
}

impl RunningApp {
    /// True when the registry key is a resolved desktop id.
    pub fn is_resolved(&self) -> bool {
        self.key.ends_with(".desktop")
    }
}

/// What an activity event records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    /// A window for the app appeared (`app_running`).
    Running,
    /// A window for the app closed (`app_exited`).
    Exited,
    /// The app became the focused window (recency only).
    Focused,
}

impl ActivityKind {
    /// The wire spelling served over the bus.
    pub fn as_str(self) -> &'static str {
        match self {
            ActivityKind::Running => "app_running",
            ActivityKind::Exited => "app_exited",
            ActivityKind::Focused => "focused",
        }
    }
}

/// One `app_running`/`app_exited`/`focused` observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEvent {
    pub kind: ActivityKind,
    pub key: String,
    /// Open windows after the event (0 once fully exited).
    pub windows: u32,
    pub at_ms: u64,
}

/// The running set, the recency order, and the activity log.
#[derive(Debug, Default)]
pub struct LaunchRegistry {
    running: BTreeMap<String, RunningApp>,
    /// App keys, most recent first, bounded by [`RECENT_CAPACITY`].
    recent: VecDeque<String>,
    events: VecDeque<ActivityEvent>,
}

impl LaunchRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A window for `key` appeared. `name`/`icon` describe the app (the raw
    /// key when unresolved). Marks the app running, moves it to the front of
    /// the recency order, and returns the recorded event.
    pub fn app_running(&mut self, key: &str, name: &str, icon: &str) -> ActivityEvent {
        let at = now_ms();
        {
            let app = self
                .running
                .entry(key.to_owned())
                .or_insert_with(|| RunningApp {
                    key: key.to_owned(),
                    name: name.to_owned(),
                    icon: icon.to_owned(),
                    windows: 0,
                    since_ms: at,
                    active_ms: at,
                });
            app.name = name.to_owned();
            app.icon = icon.to_owned();
            app.windows = app.windows.saturating_add(1);
            app.active_ms = at;
        }
        let windows = self.running.get(key).map(|app| app.windows).unwrap_or(0);
        self.touch(key);
        let event = ActivityEvent {
            kind: ActivityKind::Running,
            key: key.to_owned(),
            windows,
            at_ms: at,
        };
        self.push_event(event.clone());
        event
    }

    /// A window for `key` closed. Returns the recorded event, or `None` when
    /// the app was not running. The app stays in the recency order (it was
    /// used), but leaves the running set once its last window closes.
    pub fn app_exited(&mut self, key: &str) -> Option<ActivityEvent> {
        let at = now_ms();
        let windows = {
            let app = self.running.get_mut(key)?;
            app.windows = app.windows.saturating_sub(1);
            app.windows
        };
        if windows == 0 {
            self.running.remove(key);
        }
        let event = ActivityEvent {
            kind: ActivityKind::Exited,
            key: key.to_owned(),
            windows,
            at_ms: at,
        };
        self.push_event(event.clone());
        Some(event)
    }

    /// `key` became the focused window. Recency only: a focus change does not
    /// change the running set. Returns the recorded event.
    pub fn note_activity(&mut self, key: &str) -> ActivityEvent {
        let at = now_ms();
        if let Some(app) = self.running.get_mut(key) {
            app.active_ms = at;
        }
        self.touch(key);
        let event = ActivityEvent {
            kind: ActivityKind::Focused,
            key: key.to_owned(),
            windows: self.running.get(key).map(|app| app.windows).unwrap_or(0),
            at_ms: at,
        };
        self.push_event(event.clone());
        event
    }

    /// True when `key` has at least one open window.
    pub fn is_running(&self, key: &str) -> bool {
        self.running.contains_key(key)
    }

    /// The running app for `key`, if any.
    pub fn app(&self, key: &str) -> Option<&RunningApp> {
        self.running.get(key)
    }

    /// The running apps, ordered by key (a stable, deterministic order).
    pub fn running(&self) -> impl Iterator<Item = &RunningApp> {
        self.running.values()
    }

    /// Number of running apps.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// The app keys in recency order, most recent first, at most `limit`.
    pub fn recent(&self, limit: usize) -> Vec<&str> {
        self.recent.iter().take(limit).map(String::as_str).collect()
    }

    /// Whether the app is in the recency list (the launch registry keeps used
    /// apps even after they exit).
    pub fn was_recent(&self, key: &str) -> bool {
        self.recent.iter().any(|recent| recent == key)
    }

    /// The retained activity events, oldest first.
    pub fn events(&self) -> impl Iterator<Item = &ActivityEvent> {
        self.events.iter()
    }

    /// Take and clear the retained activity events.
    pub fn drain_events(&mut self) -> Vec<ActivityEvent> {
        self.events.drain(..).collect()
    }

    /// Move `key` to the front of the recency order, bounded.
    fn touch(&mut self, key: &str) {
        self.recent.retain(|existing| existing != key);
        self.recent.push_front(key.to_owned());
        while self.recent.len() > RECENT_CAPACITY {
            self.recent.pop_back();
        }
    }

    fn push_event(&mut self, event: ActivityEvent) {
        self.events.push_back(event);
        while self.events.len() > ACTIVITY_CAPACITY {
            self.events.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_and_exited_maintain_the_window_count() {
        let mut registry = LaunchRegistry::new();
        let event = registry.app_running("firefox.desktop", "Firefox", "firefox");
        assert_eq!(event.kind, ActivityKind::Running);
        assert_eq!(event.windows, 1);
        assert_eq!(registry.running_count(), 1);

        registry.app_running("firefox.desktop", "Firefox", "firefox");
        assert_eq!(registry.running().next().unwrap().windows, 2);

        let event = registry.app_exited("firefox.desktop").unwrap();
        assert_eq!(event.kind, ActivityKind::Exited);
        assert_eq!(event.windows, 1);
        assert!(registry.is_running("firefox.desktop"));

        let event = registry.app_exited("firefox.desktop").unwrap();
        assert_eq!(event.windows, 0);
        assert!(!registry.is_running("firefox.desktop"));
        assert_eq!(registry.running_count(), 0);
        // A close for a non-running app is a no-op.
        assert!(registry.app_exited("firefox.desktop").is_none());
    }

    #[test]
    fn recency_is_most_recent_first_and_a_running_app_jumps_to_the_front() {
        let mut registry = LaunchRegistry::new();
        registry.app_running("a.desktop", "A", "");
        registry.app_running("b.desktop", "B", "");
        registry.app_running("c.desktop", "C", "");
        assert_eq!(
            registry.recent(10),
            vec!["c.desktop", "b.desktop", "a.desktop"]
        );

        // Re-opening an already-running app moves it to the front.
        registry.app_running("a.desktop", "A", "");
        assert_eq!(
            registry.recent(10),
            vec!["a.desktop", "c.desktop", "b.desktop"]
        );

        // A focus change reorders without changing the running set.
        registry.note_activity("b.desktop");
        assert_eq!(
            registry.recent(10),
            vec!["b.desktop", "a.desktop", "c.desktop"]
        );
        assert_eq!(registry.running_count(), 3);

        // A resolved app is reported as resolved; a raw key is not.
        registry.app_running("org.example.Raw", "org.example.Raw", "");
        assert!(!registry.running().last().unwrap().is_resolved());
    }

    #[test]
    fn an_exited_app_stays_in_recency_but_leaves_running() {
        let mut registry = LaunchRegistry::new();
        registry.app_running("a.desktop", "A", "");
        registry.app_running("b.desktop", "B", "");
        registry.app_exited("b.desktop");

        assert!(!registry.is_running("b.desktop"));
        assert!(registry.was_recent("b.desktop"));
        assert_eq!(registry.recent(10), vec!["b.desktop", "a.desktop"]);
    }

    #[test]
    fn the_recency_ring_is_bounded() {
        let mut registry = LaunchRegistry::new();
        for index in 0..(RECENT_CAPACITY + 5) {
            let key = format!("app-{index}.desktop");
            registry.app_running(&key, &key, "");
        }
        assert_eq!(registry.recent(usize::MAX).len(), RECENT_CAPACITY);
        // The oldest keys fell off the back.
        assert!(!registry.was_recent("app-0.desktop"));
    }

    #[test]
    fn events_record_the_activity_sequence() {
        let mut registry = LaunchRegistry::new();
        registry.app_running("a.desktop", "A", "");
        registry.note_activity("a.desktop");
        registry.app_exited("a.desktop");
        let events = registry.drain_events();
        let kinds: Vec<&str> = events.iter().map(|event| event.kind.as_str()).collect();
        assert_eq!(kinds, vec!["app_running", "focused", "app_exited"]);
        assert!(registry.events().next().is_none());
    }
}
