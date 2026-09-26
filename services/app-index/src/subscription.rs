// SPDX-License-Identifier: MIT
//! Coalesced change subscriptions (T-14.1c).
//!
//! Consumers stop re-querying the index: they [`subscribe`](Subscriptions::subscribe)
//! to the categories they care about and receive one coalesced signal per burst
//! of changes. The bookkeeping is pure and std-only so it is unit-tested with an
//! injected clock; the D-Bus surface ([`crate::dbus`]) only forwards the
//! notices and starts a wakeable coalescer thread.
//!
//! * **Interests** — a subscriber is interested in [`ChangeKind::Identity`]
//!   (a `.desktop` entry appeared, changed, or vanished), [`ChangeKind::Recency`]
//!   (an app started, stopped, or became focused), and/or [`ChangeKind::Icons`]
//!   (the set of themed icons that resolve changed). A signal carries the union
//!   of the kinds that changed since the last delivery, so a subscriber that
//!   wanted any of them gets exactly one wake-up.
//! * **Coalescing** — the first change opens a [`COALESCE_WINDOW_MS`] window;
//!   every further change inside it folds into the same pending set. A busy
//!   install storm or a window opening and focusing in the same tick is one
//!   signal, not ten.
//! * **No polling** — [`Subscriptions::next_deadline_ms`] tells the coalescer
//!   exactly when to wake next, or `None` when there is nothing pending, so an
//!   idle service sleeps rather than polls.

use std::collections::BTreeMap;

/// How long (milliseconds) a burst of changes is coalesced before one signal is
/// sent. The window opens at the first change and is not extended by later
/// ones, so a continuous stream still delivers at a bounded rate.
pub const COALESCE_WINDOW_MS: u64 = 100;

/// One category of change a consumer can subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChangeKind {
    /// A `.desktop` entry was installed, updated, or uninstalled.
    Identity,
    /// An app started, stopped, or became the focused window.
    Recency,
    /// The themed icon resolution set changed.
    Icons,
}

impl ChangeKind {
    /// Every kind, in the canonical order.
    pub const ALL: [ChangeKind; 3] = [ChangeKind::Identity, ChangeKind::Recency, ChangeKind::Icons];

    /// The wire spelling, as accepted by [`Interests::parse`].
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeKind::Identity => "identity",
            ChangeKind::Recency => "recency",
            ChangeKind::Icons => "icons",
        }
    }

    /// Parse one wire token.
    pub fn parse(value: &str) -> Option<ChangeKind> {
        match value.trim().to_ascii_lowercase().as_str() {
            "identity" | "index" => Some(ChangeKind::Identity),
            "recency" | "activity" => Some(ChangeKind::Recency),
            "icons" | "icon" => Some(ChangeKind::Icons),
            _ => None,
        }
    }
}

/// A set of [`ChangeKind`]s, packed as a bitmask (the D-Bus surface is a
/// string, but the model stays a set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Interests(u8);

impl Interests {
    /// No interests.
    pub const NONE: Interests = Interests(0);
    /// Install/update/uninstall.
    pub const IDENTITY: Interests = Interests(0b001);
    /// App running/exited/focused.
    pub const RECENCY: Interests = Interests(0b010);
    /// Themed icon resolution.
    pub const ICONS: Interests = Interests(0b100);
    /// Every kind.
    pub const ALL: Interests = Interests(0b111);

    /// The singleton interest for one kind.
    pub fn of(kind: ChangeKind) -> Interests {
        match kind {
            ChangeKind::Identity => Interests::IDENTITY,
            ChangeKind::Recency => Interests::RECENCY,
            ChangeKind::Icons => Interests::ICONS,
        }
    }

    /// Parse a consumer spec: a comma/space-separated list of `identity`,
    /// `recency`, and `icons`, or `all`. An empty spec means "all"; unknown
    /// tokens are ignored. A non-empty spec that names no known token is
    /// treated as "all" so an older consumer asking for an unknown category
    /// still hears about everything rather than nothing.
    pub fn parse(spec: &str) -> Interests {
        let trimmed = spec.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
            return Interests::ALL;
        }
        let mut interests = Interests::NONE;
        let mut recognized = false;
        for token in trimmed.split(|c: char| c == ',' || c.is_whitespace()) {
            if token.is_empty() {
                continue;
            }
            if let Some(kind) = ChangeKind::parse(token) {
                recognized = true;
                interests = interests.union(Interests::of(kind));
            }
        }
        if recognized {
            interests
        } else {
            Interests::ALL
        }
    }

    /// Whether this set contains `kind`.
    pub fn includes(self, kind: ChangeKind) -> bool {
        self.contains(Interests::of(kind))
    }

    /// Whether this set contains every bit of `other`.
    pub fn contains(self, other: Interests) -> bool {
        self.0 & other.0 == other.0
    }

    /// The union of two sets.
    pub fn union(self, other: Interests) -> Interests {
        Interests(self.0 | other.0)
    }

    /// The intersection of two sets.
    pub fn intersection(self, other: Interests) -> Interests {
        Interests(self.0 & other.0)
    }

    /// True when no kind is selected.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The raw bitmask (for logs).
    pub fn bits(self) -> u8 {
        self.0
    }

    /// The wire spelling: the selected kinds in canonical order, comma
    /// separated. Empty means none.
    pub fn as_str(self) -> String {
        ChangeKind::ALL
            .iter()
            .filter(|kind| self.includes(**kind))
            .map(|kind| kind.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The selected kinds in canonical order.
    pub fn kinds(self) -> Vec<ChangeKind> {
        ChangeKind::ALL
            .into_iter()
            .filter(|kind| self.includes(*kind))
            .collect()
    }
}

impl From<ChangeKind> for Interests {
    fn from(kind: ChangeKind) -> Self {
        Interests::of(kind)
    }
}

/// One pending delivery for one subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeNotice {
    /// The subscriber's unique bus name (the signal destination).
    pub destination: String,
    /// The union of kinds that changed since the last delivery.
    pub interests: Interests,
}

impl ChangeNotice {
    /// The wire spelling of the changed kinds.
    pub fn interests_str(&self) -> String {
        self.interests.as_str()
    }
}

/// The subscriber table and the pending coalesced deliveries.
#[derive(Debug, Default)]
pub struct Subscriptions {
    subscribers: BTreeMap<String, Interests>,
    pending: BTreeMap<String, Interests>,
    /// When the current coalescing window closes, if anything is pending.
    deadline_ms: Option<u64>,
}

impl Subscriptions {
    /// An empty subscription table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or update) the subscriber at `destination` for `interests`.
    /// Returns `true` when the registration changed: a new subscriber, or an
    /// existing one whose interests differ. A redundant call is a no-op, so a
    /// consumer re-subscribing after a reconnect is harmless.
    ///
    /// An existing subscriber's pending kinds are trimmed to the new interests,
    /// so narrowing mid-window cannot deliver a kind that is no longer wanted.
    pub fn subscribe(&mut self, destination: &str, interests: Interests) -> bool {
        if destination.is_empty() {
            return false;
        }
        if self.subscribers.get(destination) == Some(&interests) {
            return false;
        }
        self.subscribers.insert(destination.to_owned(), interests);
        if let Some(pending) = self.pending.get_mut(destination) {
            *pending = pending.intersection(interests);
            if pending.is_empty() {
                self.pending.remove(destination);
            }
        }
        self.clear_deadline_if_idle();
        true
    }

    /// Remove a subscriber and any pending delivery for it. Returns `true`
    /// when a subscriber was removed.
    pub fn unsubscribe(&mut self, destination: &str) -> bool {
        let removed = self.subscribers.remove(destination).is_some();
        self.pending.remove(destination);
        self.clear_deadline_if_idle();
        removed
    }

    /// Whether `destination` is currently subscribed.
    pub fn is_subscribed(&self, destination: &str) -> bool {
        self.subscribers.contains_key(destination)
    }

    /// The interests a subscriber registered, if any.
    pub fn interests(&self, destination: &str) -> Option<Interests> {
        self.subscribers.get(destination).copied()
    }

    /// The pending (not yet delivered) kinds for a destination.
    pub fn pending(&self, destination: &str) -> Interests {
        self.pending.get(destination).copied().unwrap_or_default()
    }

    /// Number of subscribers.
    pub fn subscribers(&self) -> usize {
        self.subscribers.len()
    }

    /// Whether any delivery is pending.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Record a change of `kind` at `now_ms`. Every subscriber interested in
    /// the kind accumulates it; the first change in a window starts the
    /// coalescing deadline.
    pub fn note(&mut self, kind: ChangeKind, now_ms: u64) {
        let mut any = false;
        for (destination, interests) in &self.subscribers {
            if interests.includes(kind) {
                let entry = self.pending.entry(destination.clone()).or_default();
                *entry = entry.union(Interests::of(kind));
                any = true;
            }
        }
        if any && self.deadline_ms.is_none() {
            self.deadline_ms = Some(now_ms.saturating_add(COALESCE_WINDOW_MS));
        }
    }

    /// When the coalescer should wake next, or `None` when idle.
    pub fn next_deadline_ms(&self) -> Option<u64> {
        self.deadline_ms
    }

    /// Deliver every pending notice once `now_ms` has reached the deadline,
    /// otherwise nothing. Clears the pending set and the deadline.
    pub fn due(&mut self, now_ms: u64) -> Vec<ChangeNotice> {
        match self.deadline_ms {
            Some(deadline) if now_ms >= deadline => self.flush(),
            _ => Vec::new(),
        }
    }

    /// Deliver every pending notice immediately, regardless of the deadline
    /// (the D-Bus `Flush` method and deterministic tests).
    pub fn flush(&mut self) -> Vec<ChangeNotice> {
        self.deadline_ms = None;
        std::mem::take(&mut self.pending)
            .into_iter()
            .map(|(destination, interests)| ChangeNotice {
                destination,
                interests,
            })
            .collect()
    }

    fn clear_deadline_if_idle(&mut self) {
        if self.pending.is_empty() {
            self.deadline_ms = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_or_all_spec_selects_every_kind() {
        assert_eq!(Interests::parse(""), Interests::ALL);
        assert_eq!(Interests::parse("all"), Interests::ALL);
        assert_eq!(Interests::parse("  ALL  "), Interests::ALL);
        assert_eq!(Interests::parse("nonsense"), Interests::ALL);
    }

    #[test]
    fn a_spec_parses_a_subset_and_round_trips() {
        let interests = Interests::parse("identity, icons");
        assert!(interests.includes(ChangeKind::Identity));
        assert!(interests.includes(ChangeKind::Icons));
        assert!(!interests.includes(ChangeKind::Recency));
        assert_eq!(interests.as_str(), "identity,icons");
        assert_eq!(Interests::parse(&interests.as_str()), interests);
        assert_eq!(Interests::parse("activity"), Interests::RECENCY);
    }

    #[test]
    fn subscribing_is_idempotent_and_unsubscribe_clears() {
        let mut subs = Subscriptions::new();
        assert!(subs.subscribe(":1.1", Interests::ALL));
        assert!(!subs.subscribe(":1.1", Interests::ALL));
        assert_eq!(subs.subscribers(), 1);
        assert!(subs.subscribe(":1.1", Interests::IDENTITY));
        assert_eq!(subs.interests(":1.1"), Some(Interests::IDENTITY));
        assert!(subs.unsubscribe(":1.1"));
        assert!(!subs.unsubscribe(":1.1"));
        assert_eq!(subs.subscribers(), 0);
    }

    #[test]
    fn a_burst_coalesces_into_one_notice_per_interested_subscriber() {
        let mut subs = Subscriptions::new();
        subs.subscribe(":1.1", Interests::ALL);
        subs.subscribe(":1.2", Interests::IDENTITY);

        // An identity change, then two recency changes, all inside the window.
        subs.note(ChangeKind::Identity, 1_000);
        subs.note(ChangeKind::Recency, 1_010);
        subs.note(ChangeKind::Recency, 1_020);

        // Before the deadline: nothing delivered.
        assert!(subs.due(1_050).is_empty());

        let notices = subs.due(1_000 + COALESCE_WINDOW_MS);
        assert_eq!(notices.len(), 2, "one notice per interested subscriber");
        let all = notices.iter().find(|n| n.destination == ":1.1").unwrap();
        assert_eq!(all.interests.as_str(), "identity,recency");
        let identity = notices.iter().find(|n| n.destination == ":1.2").unwrap();
        assert_eq!(identity.interests.as_str(), "identity");
        assert!(!subs.has_pending());
        // Delivered once; another flush is empty.
        assert!(subs.flush().is_empty());
    }

    #[test]
    fn a_subscriber_only_hears_its_own_kinds() {
        let mut subs = Subscriptions::new();
        subs.subscribe(":1.1", Interests::RECENCY);
        subs.note(ChangeKind::Identity, 0);
        assert!(subs.flush().is_empty());
        subs.note(ChangeKind::Recency, 0);
        assert_eq!(subs.flush().len(), 1);
    }

    #[test]
    fn narrowing_interests_mid_window_drops_unwanted_pending() {
        let mut subs = Subscriptions::new();
        subs.subscribe(":1.1", Interests::ALL);
        subs.note(ChangeKind::Identity, 0);
        subs.note(ChangeKind::Recency, 1);
        // Narrow to recency only before delivery.
        subs.subscribe(":1.1", Interests::RECENCY);
        let notices = subs.flush();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].interests.as_str(), "recency");
    }

    #[test]
    fn no_subscribers_means_no_deadline_and_no_work() {
        let mut subs = Subscriptions::new();
        subs.note(ChangeKind::Identity, 0);
        assert!(subs.next_deadline_ms().is_none());
        assert!(!subs.has_pending());
        assert!(subs.flush().is_empty());
    }

    #[test]
    fn the_deadline_opens_once_and_is_not_extended() {
        let mut subs = Subscriptions::new();
        subs.subscribe(":1.1", Interests::ALL);
        subs.note(ChangeKind::Identity, 100);
        assert_eq!(subs.next_deadline_ms(), Some(100 + COALESCE_WINDOW_MS));
        // A later note in the same window does not push the deadline out.
        subs.note(ChangeKind::Recency, 150);
        assert_eq!(subs.next_deadline_ms(), Some(100 + COALESCE_WINDOW_MS));
    }

    #[test]
    fn unsubscribing_clears_a_pending_notice() {
        let mut subs = Subscriptions::new();
        subs.subscribe(":1.1", Interests::ALL);
        subs.note(ChangeKind::Identity, 0);
        subs.unsubscribe(":1.1");
        assert!(!subs.has_pending());
        assert!(subs.flush().is_empty());
    }
}
