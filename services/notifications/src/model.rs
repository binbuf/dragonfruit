// SPDX-License-Identifier: MIT
//! The notification model (T-11.1a).
//!
//! One in-memory owner of the notification queue and its history
//! ([01-architecture.md](../../docs/design/01-architecture.md)): a
//! [`Notification`] is created by an app's `Notify` call, lives in the active
//! `banners` queue until it is closed or expires, and is recorded in the
//! bounded `history` for the notification center.
//!
//! The model is deliberately a plain Rust value with no D-Bus or JSON in it,
//! so the round-trip and policy tests drive it directly. The D-Bus surface in
//! [`crate::dbus`] is a thin adapter over it.
//!
//! Focus/DND is *owned* here (the single source of truth for Control Center,
//! the menu bar, and Settings) but its policy state machine is T-11.2a; this
//! task ships the state bit and the "suppress the banner, keep the history"
//! rule so the later policy task has one place to grow.

use std::time::{SystemTime, UNIX_EPOCH};

/// The expiration timeout a `Notify` call may request (`expire_timeout`).
pub const DEFAULT_TIMEOUT_MS: i32 = 5000;

/// How many closed notifications the history keeps. The oldest fall off.
pub const HISTORY_CAPACITY: usize = 100;

/// Wall-clock milliseconds since the Unix epoch; the model's only clock.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Why a notification left the active queue, matching the freedesktop
/// `NotificationClosed` reason field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    /// `expire_timeout` elapsed (1).
    Expired,
    /// The user or the shell dismissed it (2).
    Dismissed,
    /// `CloseNotification` was called, or the owner replaced it (3).
    Closed,
    /// Undefined/other (4).
    Undefined,
}

impl CloseReason {
    /// The numeric reason on the wire.
    pub fn as_u32(self) -> u32 {
        match self {
            CloseReason::Expired => 1,
            CloseReason::Dismissed => 2,
            CloseReason::Closed => 3,
            CloseReason::Undefined => 4,
        }
    }

    /// The lowercase name the shell-facing JSON carries.
    pub fn name(self) -> &'static str {
        match self {
            CloseReason::Expired => "expired",
            CloseReason::Dismissed => "dismissed",
            CloseReason::Closed => "closed",
            CloseReason::Undefined => "undefined",
        }
    }
}

/// The freedesktop urgency hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    /// Parse the `urgency` hint byte (0 low, 1 normal, 2 critical).
    pub fn from_hint(value: u8) -> Self {
        match value {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }

    /// The lowercase name the shell-facing JSON carries.
    pub fn name(self) -> &'static str {
        match self {
            Urgency::Low => "low",
            Urgency::Normal => "normal",
            Urgency::Critical => "critical",
        }
    }
}

/// One inline action a notification offers. T-11.1a records them; the
/// round-trip to the app is T-11.1b.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// The opaque key the app expects back in `ActionInvoked`.
    pub key: String,
    /// The user-visible label.
    pub label: String,
}

impl Action {
    /// A new action.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Action {
            key: key.into(),
            label: label.into(),
        }
    }
}

/// The `Notify` arguments, in the order the freedesktop interface receives
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyRequest {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<Action>,
    pub urgency: Urgency,
    /// The requested timeout in milliseconds: `< 0` is the service default,
    /// `0` means never expire, `> 0` is the exact lifetime.
    pub expire_timeout_ms: i32,
}

impl Default for NotifyRequest {
    fn default() -> Self {
        NotifyRequest {
            app_name: String::new(),
            replaces_id: 0,
            app_icon: String::new(),
            summary: String::new(),
            body: String::new(),
            actions: Vec::new(),
            urgency: Urgency::Normal,
            expire_timeout_ms: -1,
        }
    }
}

/// An active notification (a banner).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<Action>,
    pub urgency: Urgency,
    pub expire_timeout_ms: i32,
    pub created_at_ms: u64,
}

impl Notification {
    /// The deadline at which this banner expires, or `None` when it never
    /// does. `default_timeout_ms` is the service default a `< 0` request
    /// resolves to; a non-positive default means "never".
    pub fn deadline_ms(&self, default_timeout_ms: i32) -> Option<u64> {
        let effective = if self.expire_timeout_ms < 0 {
            default_timeout_ms
        } else {
            self.expire_timeout_ms
        };
        if effective <= 0 {
            None
        } else {
            Some(self.created_at_ms.saturating_add(effective as u64))
        }
    }
}

/// One recorded notification (open or closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<Action>,
    pub urgency: Urgency,
    pub created_at_ms: u64,
    /// `None` while the notification is still open in the banner queue.
    pub closed_at_ms: Option<u64>,
    pub reason: Option<CloseReason>,
}

impl From<&Notification> for HistoryEntry {
    fn from(notification: &Notification) -> Self {
        HistoryEntry {
            id: notification.id,
            app_name: notification.app_name.clone(),
            app_icon: notification.app_icon.clone(),
            summary: notification.summary.clone(),
            body: notification.body.clone(),
            actions: notification.actions.clone(),
            urgency: notification.urgency,
            created_at_ms: notification.created_at_ms,
            closed_at_ms: None,
            reason: None,
        }
    }
}

/// The notification queue and its history.
#[derive(Debug, Clone)]
pub struct Queue {
    next_id: u32,
    active: Vec<Notification>,
    history: Vec<HistoryEntry>,
    capacity: usize,
    default_timeout_ms: i32,
    do_not_disturb: bool,
}

impl Default for Queue {
    fn default() -> Self {
        Queue::new()
    }
}

impl Queue {
    /// A new empty queue with the service default timeout.
    pub fn new() -> Self {
        Queue {
            next_id: 1,
            active: Vec::new(),
            history: Vec::new(),
            capacity: HISTORY_CAPACITY,
            default_timeout_ms: DEFAULT_TIMEOUT_MS,
            do_not_disturb: false,
        }
    }

    /// A new queue with an explicit history capacity (tests).
    pub fn with_capacity(capacity: usize) -> Self {
        Queue {
            capacity,
            ..Queue::new()
        }
    }

    /// Record a `Notify` and return its id. `replaces_id` reuses an existing
    /// id when the notification is still known; otherwise a fresh id is
    /// allocated. The notification is always recorded in the history; it
    /// becomes an active banner unless Do Not Disturb is on.
    pub fn notify(&mut self, request: NotifyRequest, now_ms: u64) -> u32 {
        let replacing = request.replaces_id != 0
            && (self.active.iter().any(|n| n.id == request.replaces_id)
                || self.history.iter().any(|h| h.id == request.replaces_id));
        let id = if replacing {
            request.replaces_id
        } else {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            id
        };

        // A replacement supersedes the old banner and its history entry.
        self.active.retain(|n| n.id != id);
        self.history.retain(|entry| entry.id != id);

        let notification = Notification {
            id,
            app_name: request.app_name,
            app_icon: request.app_icon,
            summary: request.summary,
            body: request.body,
            actions: request.actions,
            urgency: request.urgency,
            expire_timeout_ms: request.expire_timeout_ms,
            created_at_ms: now_ms,
        };
        self.push_history(HistoryEntry::from(&notification));
        if !self.do_not_disturb {
            self.active.push(notification);
        }
        id
    }

    /// Close an active notification with `reason`. Returns the removed
    /// notification when it was active. The history entry gains the close
    /// time and reason.
    pub fn close(&mut self, id: u32, reason: CloseReason, now_ms: u64) -> Option<Notification> {
        let index = self.active.iter().position(|n| n.id == id)?;
        let notification = self.active.remove(index);
        if let Some(entry) = self.history.iter_mut().find(|entry| entry.id == id) {
            entry.closed_at_ms = Some(now_ms);
            entry.reason = Some(reason);
        }
        Some(notification)
    }

    /// Expire every active banner whose deadline has passed. Returns the
    /// closed ids, oldest first.
    pub fn expire_due(&mut self, now_ms: u64) -> Vec<u32> {
        let default = self.default_timeout_ms;
        let due: Vec<Notification> = self
            .active
            .iter()
            .filter(|n| n.deadline_ms(default).is_some_and(|d| d <= now_ms))
            .cloned()
            .collect();
        due.into_iter()
            .map(|notification| {
                let id = notification.id;
                self.active.retain(|n| n.id != id);
                if let Some(entry) = self.history.iter_mut().find(|entry| entry.id == id) {
                    entry.closed_at_ms = Some(now_ms);
                    entry.reason = Some(CloseReason::Expired);
                }
                id
            })
            .collect()
    }

    /// The earliest deadline among active banners, if any.
    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.active
            .iter()
            .filter_map(|n| n.deadline_ms(self.default_timeout_ms))
            .min()
    }

    /// The active banners, oldest first.
    pub fn banners(&self) -> &[Notification] {
        &self.active
    }

    /// The recorded history, most recent first.
    pub fn history(&self) -> Vec<&HistoryEntry> {
        self.history.iter().rev().collect()
    }

    /// Whether Do Not Disturb suppresses banners (history still records).
    pub fn do_not_disturb(&self) -> bool {
        self.do_not_disturb
    }

    /// Set Do Not Disturb. Enabling it leaves already-bannered notifications
    /// alone; new ones are recorded without a banner (the T-11.2a policy
    /// builds on this rule).
    pub fn set_do_not_disturb(&mut self, enabled: bool) {
        self.do_not_disturb = enabled;
    }

    /// The service default timeout for a `< 0` request.
    pub fn default_timeout_ms(&self) -> i32 {
        self.default_timeout_ms
    }

    /// Replace the service default timeout.
    pub fn set_default_timeout_ms(&mut self, timeout_ms: i32) {
        self.default_timeout_ms = timeout_ms;
    }

    /// One active banner by id.
    pub fn banner(&self, id: u32) -> Option<&Notification> {
        self.active.iter().find(|n| n.id == id)
    }

    /// One history entry by id.
    pub fn entry(&self, id: u32) -> Option<&HistoryEntry> {
        self.history.iter().find(|entry| entry.id == id)
    }

    fn push_history(&mut self, entry: HistoryEntry) {
        self.history.push(entry);
        let overflow = self.history.len().saturating_sub(self.capacity);
        if overflow > 0 {
            self.history.drain(0..overflow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(summary: &str, timeout: i32) -> NotifyRequest {
        NotifyRequest {
            app_name: "Mail".to_owned(),
            summary: summary.to_owned(),
            expire_timeout_ms: timeout,
            ..NotifyRequest::default()
        }
    }

    #[test]
    fn a_notify_is_bannered_and_recorded() {
        let mut queue = Queue::new();
        let id = queue.notify(request("New message", -1), 1000);
        assert_eq!(id, 1);
        assert_eq!(queue.banners().len(), 1);
        assert_eq!(queue.banners()[0].summary, "New message");
        let history = queue.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, 1);
        assert_eq!(history[0].reason, None);
        assert_eq!(history[0].closed_at_ms, None);
    }

    #[test]
    fn ids_are_monotonic_and_replaces_id_reuses_one() {
        let mut queue = Queue::new();
        let first = queue.notify(request("one", 0), 1000);
        let second = queue.notify(request("two", 0), 1001);
        assert_eq!((first, second), (1, 2));

        let mut replace = request("two (updated)", 0);
        replace.replaces_id = second;
        let replaced = queue.notify(replace, 1002);
        assert_eq!(replaced, second);
        assert_eq!(queue.banners().len(), 2);
        assert_eq!(queue.banner(second).unwrap().summary, "two (updated)");
        // The superseded history entry is replaced, not duplicated.
        assert_eq!(queue.history().len(), 2);
    }

    #[test]
    fn close_records_the_reason_and_removes_the_banner() {
        let mut queue = Queue::new();
        let id = queue.notify(request("one", 0), 1000);
        let closed = queue.close(id, CloseReason::Dismissed, 1500).unwrap();
        assert_eq!(closed.id, id);
        assert!(queue.banners().is_empty());
        let entry = queue.entry(id).unwrap();
        assert_eq!(entry.reason, Some(CloseReason::Dismissed));
        assert_eq!(entry.closed_at_ms, Some(1500));
        assert_eq!(CloseReason::Dismissed.as_u32(), 2);
    }

    #[test]
    fn deadline_rules_cover_default_expire_and_never() {
        let mut queue = Queue::new();
        let defaulted = queue.notify(request("default", -1), 1000);
        let never = queue.notify(request("never", 0), 1000);
        let exact = queue.notify(request("exact", 250), 1000);
        let banners = queue.banners();
        assert_eq!(
            banners[0].deadline_ms(DEFAULT_TIMEOUT_MS),
            Some(1000 + DEFAULT_TIMEOUT_MS as u64)
        );
        assert_eq!(banners[1].deadline_ms(DEFAULT_TIMEOUT_MS), None);
        assert_eq!(banners[2].deadline_ms(DEFAULT_TIMEOUT_MS), Some(1250));
        assert_eq!(queue.next_deadline_ms(), Some(1250));

        let expired = queue.expire_due(1250);
        assert_eq!(expired, vec![exact]);
        assert_eq!(
            queue.entry(exact).unwrap().reason,
            Some(CloseReason::Expired)
        );
        assert!(queue.banner(never).is_some());
        assert!(queue.banner(defaulted).is_some());
    }

    #[test]
    fn do_not_disturb_suppresses_the_banner_but_keeps_the_history() {
        let mut queue = Queue::new();
        queue.set_do_not_disturb(true);
        let id = queue.notify(request("quiet", -1), 1000);
        assert!(queue.banners().is_empty());
        assert_eq!(queue.history().len(), 1);
        assert_eq!(queue.entry(id).unwrap().summary, "quiet");
        assert!(queue.close(id, CloseReason::Dismissed, 1100).is_none());
    }

    #[test]
    fn history_is_bounded_newest_first() {
        let mut queue = Queue::with_capacity(3);
        for index in 0..5 {
            queue.notify(request(&format!("n{index}"), 0), 1000 + index as u64);
        }
        let history = queue.history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].summary, "n4");
        assert_eq!(history[2].summary, "n2");
    }

    #[test]
    fn urgency_parses_the_hint_bytes() {
        assert_eq!(Urgency::from_hint(0), Urgency::Low);
        assert_eq!(Urgency::from_hint(1), Urgency::Normal);
        assert_eq!(Urgency::from_hint(2), Urgency::Critical);
        assert_eq!(Urgency::from_hint(200), Urgency::Normal);
    }
}
