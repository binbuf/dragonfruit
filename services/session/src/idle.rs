// SPDX-License-Identifier: MIT
//! Idle timers: the dim → blank → lock → suspend chain (T-12.4a).
//!
//! The desktop goes idle in stages. Each stage has a delay from the last
//! user activity, and the chain only ever advances forward:
//!
//! ```text
//! Active ──dim──▶ Dim ──blank──▶ Blank ──lock──▶ Lock ──suspend──▶ Suspend
//!    ▲                                                              │
//!    └──────────────────────── activity ───────────────────────────┘
//! ```
//!
//! [`IdlePolicy`] holds one optional delay per stage; `None` disables that
//! stage. Delays are the *policy keys*: the settings daemon will own the
//! `idle.*` keys (T-12.5b), and [`IdlePolicy::from_keys`] is the seam that
//! turns them into a policy. [`IdleTimers`] is the runtime state machine. It
//! is pure and clock-injected, so a headless test drives it with a fake clock
//! and no real time passes.
//!
//! [`IdleController`] wraps the engine with the two things T-12.4a left out:
//! **idle inhibitors** and **wake restore**. An inhibitor is an opaque
//! [`InhibitorId`] in an [`IdleInhibitors`] registry; while any inhibitor is
//! held the controller's [`IdleController::poll`] is a no-op, so the chain
//! cannot advance. A wake — either a recorded [`IdleController::activity`] or
//! a fresh inhibitor (which forces the chain back to the top, per
//! [`IdleStage::Active`]) — returns an [`IdleEvent::Restore`] naming the stage
//! the caller must undo. The engine stays pure; inhibitors are a caller-side
//! decision ([ADR 0070](../../docs/design/adr/0070-idle-timer-engine-and-policy.md)).
//!
//! The decision to keep the engine in `dragonfruit-session` — rather than the
//! compositor — is frozen in
//! [ADR 0070](../../docs/design/adr/0070-idle-timer-engine-and-policy.md).

use std::time::Duration;

/// One step of the idle chain, from `Active` to `Suspend`.
///
/// The discriminants are ordered so `stage_a < stage_b` means `stage_a` is
/// earlier in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdleStage {
    /// The user is active (or an inhibitor forced the chain back to the top).
    Active,
    /// The screen dims but stays readable.
    Dim,
    /// The screen blanks without locking.
    Blank,
    /// The session locks (input is captured by the lock UI).
    Lock,
    /// The session suspends.
    Suspend,
}

impl IdleStage {
    /// Every stage, in chain order.
    pub const ALL: [IdleStage; 5] = [
        IdleStage::Active,
        IdleStage::Dim,
        IdleStage::Blank,
        IdleStage::Lock,
        IdleStage::Suspend,
    ];

    /// The stable name used in logs and policy keys.
    pub const fn as_str(self) -> &'static str {
        match self {
            IdleStage::Active => "active",
            IdleStage::Dim => "dim",
            IdleStage::Blank => "blank",
            IdleStage::Lock => "lock",
            IdleStage::Suspend => "suspend",
        }
    }

    /// Whether the stage is past `Active` (the desktop has started to idle).
    pub const fn is_idle(self) -> bool {
        !matches!(self, IdleStage::Active)
    }

    /// The next stage in the chain, or `None` at `Suspend`.
    pub const fn next(self) -> Option<IdleStage> {
        match self {
            IdleStage::Active => Some(IdleStage::Dim),
            IdleStage::Dim => Some(IdleStage::Blank),
            IdleStage::Blank => Some(IdleStage::Lock),
            IdleStage::Lock => Some(IdleStage::Suspend),
            IdleStage::Suspend => None,
        }
    }
}

impl std::fmt::Display for IdleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The settings key prefix the idle policy is read from.
pub const KEY_PREFIX: &str = "idle.";
/// The policy key for the dim delay, in whole seconds.
pub const KEY_DIM: &str = "idle.dim";
/// The policy key for the blank delay, in whole seconds.
pub const KEY_BLANK: &str = "idle.blank";
/// The policy key for the lock delay, in whole seconds.
pub const KEY_LOCK: &str = "idle.lock";
/// The policy key for the suspend delay, in whole seconds.
pub const KEY_SUSPEND: &str = "idle.suspend";

/// The policy key for one stage (empty for [`IdleStage::Active`]).
pub const fn key_for(stage: IdleStage) -> &'static str {
    match stage {
        IdleStage::Active => "",
        IdleStage::Dim => KEY_DIM,
        IdleStage::Blank => KEY_BLANK,
        IdleStage::Lock => KEY_LOCK,
        IdleStage::Suspend => KEY_SUSPEND,
    }
}

/// One idle delay, or `None` when the stage is disabled.
///
/// The chain is ordered, so an enabled stage is only reached after every
/// earlier enabled stage; see [`IdlePolicy::normalized`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdlePolicy {
    dim: Option<Duration>,
    blank: Option<Duration>,
    lock: Option<Duration>,
    suspend: Option<Duration>,
}

/// The shipped default policy, until settingsd owns the keys (T-12.5b).
///
/// The defaults are deliberately conservative on real hardware: dim after
/// 2.5 minutes, blank after 5, lock after 10, and never suspend
/// automatically (a user can still suspend from the system menu). A policy is
/// data, so T-12.5b changes only the numbers that reach [`IdlePolicy::from_keys`].
pub const DEFAULT_DIM: Duration = Duration::from_secs(150);
/// Default blank delay (see [`DEFAULT_DIM`]).
pub const DEFAULT_BLANK: Duration = Duration::from_secs(300);
/// Default lock delay (see [`DEFAULT_DIM`]).
pub const DEFAULT_LOCK: Duration = Duration::from_secs(600);

impl IdlePolicy {
    /// The shipped defaults: dim 150 s, blank 300 s, lock 600 s, no suspend.
    pub const fn new() -> Self {
        IdlePolicy {
            dim: Some(DEFAULT_DIM),
            blank: Some(DEFAULT_BLANK),
            lock: Some(DEFAULT_LOCK),
            suspend: None,
        }
    }

    /// Every stage disabled: the session never idles.
    pub const fn disabled() -> Self {
        IdlePolicy {
            dim: None,
            blank: None,
            lock: None,
            suspend: None,
        }
    }

    /// The delay configured for `stage`, if enabled.
    pub const fn delay(&self, stage: IdleStage) -> Option<Duration> {
        match stage {
            IdleStage::Active => Some(Duration::ZERO),
            IdleStage::Dim => self.dim,
            IdleStage::Blank => self.blank,
            IdleStage::Lock => self.lock,
            IdleStage::Suspend => self.suspend,
        }
    }

    /// Set the delay for `stage` (`None` disables it).
    pub fn set_delay(&mut self, stage: IdleStage, delay: Option<Duration>) {
        match stage {
            IdleStage::Active => {}
            IdleStage::Dim => self.dim = delay,
            IdleStage::Blank => self.blank = delay,
            IdleStage::Lock => self.lock = delay,
            IdleStage::Suspend => self.suspend = delay,
        }
    }

    /// Builder form of [`IdlePolicy::set_delay`].
    pub fn with_delay(mut self, stage: IdleStage, delay: Option<Duration>) -> Self {
        self.set_delay(stage, delay);
        self
    }

    /// Whether any stage is enabled.
    pub const fn is_enabled(&self) -> bool {
        self.dim.is_some() || self.blank.is_some() || self.lock.is_some() || self.suspend.is_some()
    }

    /// A copy with the enabled delays forced into chain order.
    ///
    /// A stage's effective delay is never earlier than an enabled stage
    /// before it, so a policy cannot lock the session before it dims. A
    /// disabled stage does not constrain anything.
    pub fn normalized(&self) -> IdlePolicy {
        let mut out = IdlePolicy::disabled();
        let mut previous = Duration::ZERO;
        for stage in [
            IdleStage::Dim,
            IdleStage::Blank,
            IdleStage::Lock,
            IdleStage::Suspend,
        ] {
            if let Some(delay) = self.delay(stage) {
                let effective = delay.max(previous);
                out.set_delay(stage, Some(effective));
                previous = effective;
            }
        }
        out
    }

    /// Build a policy from `(key, value)` settings pairs.
    ///
    /// Recognized keys are [`KEY_DIM`], [`KEY_BLANK`], [`KEY_LOCK`], and
    /// [`KEY_SUSPEND`]; the value is whole seconds. `0` (or a negative value)
    /// disables the stage, as does `never`. Unknown keys are ignored, so this
    /// can be handed a whole settings snapshot. This is the exact seam T-12.5b
    /// calls with settingsd values.
    pub fn from_keys<I, K, V>(keys: I) -> IdlePolicy
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut policy = IdlePolicy::disabled();
        for (key, value) in keys {
            let Some(stage) = stage_for_key(key.as_ref()) else {
                continue;
            };
            policy.set_delay(stage, parse_seconds(value.as_ref()));
        }
        policy
    }
}

impl Default for IdlePolicy {
    fn default() -> Self {
        IdlePolicy::new()
    }
}

/// The stage `key` configures, if any.
fn stage_for_key(key: &str) -> Option<IdleStage> {
    match key {
        KEY_DIM => Some(IdleStage::Dim),
        KEY_BLANK => Some(IdleStage::Blank),
        KEY_LOCK => Some(IdleStage::Lock),
        KEY_SUSPEND => Some(IdleStage::Suspend),
        _ => None,
    }
}

/// Parse a policy value: `0`, a negative number, or `never` disables the
/// stage; a positive integer is that many seconds. A non-numeric value is
/// treated as disabled rather than silently firing at an unexpected delay.
fn parse_seconds(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("never") {
        return None;
    }
    match value.parse::<i64>() {
        Ok(seconds) if seconds > 0 => Some(Duration::from_secs(seconds as u64)),
        Ok(_) => None,
        Err(_) => None,
    }
}

/// The idle-chain state machine.
///
/// Time is an opaque monotonic [`Duration`] supplied by the caller, so tests
/// use a fake clock and a real session uses an `Instant`-derived one. All
/// delays are measured from the last [`IdleTimers::activity`] (or
/// construction).
#[derive(Debug, Clone)]
pub struct IdleTimers {
    policy: IdlePolicy,
    last_activity: Duration,
    stage: IdleStage,
}

impl IdleTimers {
    /// Start the chain active at `now`, using `policy` (normalized).
    pub fn new(policy: IdlePolicy, now: Duration) -> Self {
        IdleTimers {
            policy: policy.normalized(),
            last_activity: now,
            stage: IdleStage::Active,
        }
    }

    /// The engine's normalized policy.
    pub fn policy(&self) -> IdlePolicy {
        self.policy
    }

    /// The current stage.
    pub fn stage(&self) -> IdleStage {
        self.stage
    }

    /// When activity was last recorded.
    pub fn last_activity(&self) -> Duration {
        self.last_activity
    }

    /// How long ago activity was last recorded at `now` (saturating).
    pub fn idle_for(&self, now: Duration) -> Duration {
        now.saturating_sub(self.last_activity)
    }

    /// Record user activity at `now`.
    ///
    /// Resets the chain to [`IdleStage::Active`]; returns `Some(Active)` when
    /// the engine had left the active state (so a caller can restore the
    /// screen) and `None` when it was already active.
    pub fn activity(&mut self, now: Duration) -> Option<IdleStage> {
        self.last_activity = now;
        if self.stage == IdleStage::Active {
            None
        } else {
            self.stage = IdleStage::Active;
            Some(IdleStage::Active)
        }
    }

    /// The stage the policy calls for at `now` without changing state.
    pub fn stage_at(&self, now: Duration) -> IdleStage {
        let elapsed = self.idle_for(now);
        let mut reached = IdleStage::Active;
        for stage in IdleStage::ALL {
            if let Some(delay) = self.policy.delay(stage) {
                if elapsed >= delay {
                    reached = stage;
                }
            }
        }
        reached
    }

    /// Advance to the stage due at `now`.
    ///
    /// Returns `Some(stage)` when the stage changed, `None` when it did not.
    /// A single call jumps straight to the furthest due stage, so a caller
    /// that missed a tick does not replay every intermediate stage.
    pub fn poll(&mut self, now: Duration) -> Option<IdleStage> {
        let due = self.stage_at(now);
        if due == self.stage {
            return None;
        }
        self.stage = due;
        Some(due)
    }

    /// The instant the next enabled stage fires, if any.
    ///
    /// This is the deadline a driver sleeps toward; `None` means no further
    /// stage is enabled. The value may already be in the past when a tick is
    /// late, in which case [`IdleTimers::poll`] fires immediately.
    pub fn next_deadline(&self) -> Option<Duration> {
        let next = IdleStage::ALL
            .into_iter()
            .filter(|stage| *stage > self.stage)
            .find_map(|stage| self.policy.delay(stage).map(|delay| (stage, delay)))?;
        Some(self.last_activity.saturating_add(next.1))
    }

    /// Replace the policy live, re-evaluating the current stage at `now`.
    ///
    /// Returns `Some(stage)` if the change moved the engine, mirroring
    /// [`IdleTimers::poll`]. Raising a delay that has not yet elapsed keeps the
    /// current stage; lowering one can advance immediately.
    pub fn set_policy(&mut self, policy: IdlePolicy, now: Duration) -> Option<IdleStage> {
        self.policy = policy.normalized();
        self.poll(now)
    }

    /// Replace the policy without re-evaluating the current stage.
    ///
    /// [`IdleTimers::set_policy`] advances immediately when a shortened delay
    /// is already due; an inhibited [`IdleController`] must not, so it stores
    /// the new policy here and lets a later [`IdleController::poll`] catch up.
    pub fn replace_policy(&mut self, policy: IdlePolicy) {
        self.policy = policy.normalized();
    }
}

/// A handle for one held idle inhibitor.
///
/// Handles are minted by [`IdleInhibitors::acquire`] in increasing order, so a
/// released handle is never re-minted within the same registry and a double
/// release is a safe no-op. A handle is only meaningful to the registry that
/// minted it (see [ADR 0071](../../docs/design/adr/0071-idle-inhibitors-and-wake-restore.md));
/// the idle service recreates the controller across restarts rather than
/// carrying a handle over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InhibitorId(u64);

impl InhibitorId {
    /// The raw value, for logs and tests.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The set of currently held idle inhibitors.
///
/// The registry is deliberately surface-agnostic: the future idle service
/// translates the compositor's `idle-inhibit` surfaces into one handle each,
/// and the controller only cares whether the set is empty. A dead holder is a
/// release the service never sent, so callers must release on teardown.
#[derive(Debug, Clone, Default)]
pub struct IdleInhibitors {
    next: u64,
    held: std::collections::BTreeSet<InhibitorId>,
}

impl IdleInhibitors {
    /// An empty registry.
    pub fn new() -> Self {
        IdleInhibitors::default()
    }

    /// Mint a new inhibitor handle and add it to the set.
    pub fn acquire(&mut self) -> InhibitorId {
        let id = InhibitorId(self.next);
        self.next += 1;
        self.held.insert(id);
        id
    }

    /// Drop `id`; returns whether it was held.
    pub fn release(&mut self, id: InhibitorId) -> bool {
        self.held.remove(&id)
    }

    /// Whether at least one inhibitor is held.
    pub fn is_inhibited(&self) -> bool {
        !self.held.is_empty()
    }

    /// How many inhibitors are held.
    pub fn count(&self) -> usize {
        self.held.len()
    }

    /// Whether `id` is currently held.
    pub fn contains(&self, id: InhibitorId) -> bool {
        self.held.contains(&id)
    }

    /// Drop every inhibitor, returning how many were held.
    pub fn clear(&mut self) -> usize {
        let held = self.held.len();
        self.held.clear();
        held
    }

    /// The held handles, in mint order.
    pub fn iter(&self) -> impl Iterator<Item = InhibitorId> + '_ {
        self.held.iter().copied()
    }
}

/// An observable change to the idle chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleEvent {
    /// The chain advanced to `stage`; the caller applies the stage's screen
    /// state (dim, blank, lock, or suspend).
    Enter(IdleStage),
    /// Activity (or a new inhibitor) forced the chain back to active from
    /// `stage`; the caller restores the prior screen state.
    Restore(IdleStage),
}

impl IdleEvent {
    /// The stage the event names (the one entered or the one restored from).
    pub const fn stage(self) -> IdleStage {
        match self {
            IdleEvent::Enter(stage) | IdleEvent::Restore(stage) => stage,
        }
    }
}

/// The idle chain plus its inhibitors and wake restore (T-12.4b).
///
/// [`IdleController`] owns an [`IdleTimers`] and an [`IdleInhibitors`], and
/// turns both into [`IdleEvent`]s:
///
/// * While any inhibitor is held, [`poll`](IdleController::poll) does nothing,
///   so the chain is frozen and the deadline is not scheduled
///   ([`next_deadline`](IdleController::next_deadline) returns `None`).
/// * A wake — [`activity`](IdleController::activity) or a fresh inhibitor —
///   resets the chain to [`IdleStage::Active`] and reports a
///   [`IdleEvent::Restore`] with the stage that was left, so the caller knows
///   what display state to undo. Waking from [`IdleStage::Lock`] still reports
///   `Restore(Lock)`; whether that unlocks is a security decision the caller
///   makes (the lock UI, not the idle chain, owns unlocking).
/// * Releasing an inhibitor does not move the chain or reset the inactivity
///   clock; the next [`poll`](IdleController::poll) catches up to the stage
///   that is already due, which is what an inhibitor held over a deadline
///   should do.
#[derive(Debug, Clone)]
pub struct IdleController {
    timers: IdleTimers,
    inhibitors: IdleInhibitors,
}

impl IdleController {
    /// Start the chain active at `now`, with no inhibitors.
    pub fn new(policy: IdlePolicy, now: Duration) -> Self {
        IdleController {
            timers: IdleTimers::new(policy, now),
            inhibitors: IdleInhibitors::new(),
        }
    }

    /// The wrapped engine.
    pub fn timers(&self) -> &IdleTimers {
        &self.timers
    }

    /// The engine's normalized policy.
    pub fn policy(&self) -> IdlePolicy {
        self.timers.policy()
    }

    /// The current stage.
    pub fn stage(&self) -> IdleStage {
        self.timers.stage()
    }

    /// When activity was last recorded.
    pub fn last_activity(&self) -> Duration {
        self.timers.last_activity()
    }

    /// How long ago activity was last recorded at `now` (saturating).
    pub fn idle_for(&self, now: Duration) -> Duration {
        self.timers.idle_for(now)
    }

    /// The inhibitor registry.
    pub fn inhibitors(&self) -> &IdleInhibitors {
        &self.inhibitors
    }

    /// Whether at least one inhibitor is held (the chain is frozen).
    pub fn is_inhibited(&self) -> bool {
        self.inhibitors.is_inhibited()
    }

    /// How many inhibitors are held.
    pub fn inhibitor_count(&self) -> usize {
        self.inhibitors.count()
    }

    /// Hold a new inhibitor, forcing the chain back to [`IdleStage::Active`].
    ///
    /// Returns the handle and an [`IdleEvent::Restore`] when the chain had
    /// left active; the event is `None` when it was already active.
    pub fn acquire_inhibitor(&mut self, now: Duration) -> (InhibitorId, Option<IdleEvent>) {
        let id = self.inhibitors.acquire();
        (id, self.wake(now))
    }

    /// Release the inhibitor `id`; returns whether it was held.
    pub fn release_inhibitor(&mut self, id: InhibitorId) -> bool {
        self.inhibitors.release(id)
    }

    /// Drop every held inhibitor, returning how many were held.
    ///
    /// This is the teardown path: when the idle service restarts, no old
    /// handle can survive, so the new registry starts empty.
    pub fn clear_inhibitors(&mut self) -> usize {
        self.inhibitors.clear()
    }

    /// Record user activity, waking the chain.
    ///
    /// Returns [`IdleEvent::Restore`] if the chain had left active, else
    /// `None`.
    pub fn activity(&mut self, now: Duration) -> Option<IdleEvent> {
        self.wake(now)
    }

    /// Advance to the stage due at `now`, unless an inhibitor is held.
    pub fn poll(&mut self, now: Duration) -> Option<IdleEvent> {
        if self.is_inhibited() {
            return None;
        }
        self.timers.poll(now).map(IdleEvent::Enter)
    }

    /// The instant the next enabled stage fires, if any.
    ///
    /// `None` when the chain is inhibited, because no stage can fire while an
    /// inhibitor is held.
    pub fn next_deadline(&self) -> Option<Duration> {
        if self.is_inhibited() {
            None
        } else {
            self.timers.next_deadline()
        }
    }

    /// Replace the policy live. While inhibited the new policy is stored but
    /// the chain is not advanced; the next [`poll`](IdleController::poll)
    /// catches up.
    pub fn set_policy(&mut self, policy: IdlePolicy, now: Duration) -> Option<IdleEvent> {
        if self.is_inhibited() {
            self.timers.replace_policy(policy);
            None
        } else {
            self.timers.set_policy(policy, now).map(IdleEvent::Enter)
        }
    }

    /// Reset the chain to active, reporting the stage that was left.
    fn wake(&mut self, now: Duration) -> Option<IdleEvent> {
        let before = self.timers.stage();
        self.timers
            .activity(now)
            .map(|_| IdleEvent::Restore(before))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    fn chain_policy() -> IdlePolicy {
        IdlePolicy::disabled()
            .with_delay(IdleStage::Dim, Some(seconds(5)))
            .with_delay(IdleStage::Blank, Some(seconds(10)))
            .with_delay(IdleStage::Lock, Some(seconds(15)))
            .with_delay(IdleStage::Suspend, Some(seconds(20)))
    }

    #[test]
    fn each_stage_fires_at_its_configured_delay() {
        let mut timers = IdleTimers::new(chain_policy(), seconds(0));
        assert_eq!(timers.stage(), IdleStage::Active);

        // The tick before a deadline changes nothing.
        assert_eq!(timers.poll(seconds(4)), None);
        assert_eq!(timers.stage(), IdleStage::Active);

        for (at, expected) in [
            (5, IdleStage::Dim),
            (10, IdleStage::Blank),
            (15, IdleStage::Lock),
            (20, IdleStage::Suspend),
        ] {
            assert_eq!(timers.poll(seconds(at)), Some(expected), "at {at}s");
            assert_eq!(timers.stage(), expected);
        }

        // Past the last stage the chain stays put.
        assert_eq!(timers.poll(seconds(3600)), None);
        assert_eq!(timers.stage(), IdleStage::Suspend);
    }

    #[test]
    fn a_late_tick_jumps_to_the_furthest_due_stage() {
        let mut timers = IdleTimers::new(chain_policy(), seconds(0));
        assert_eq!(timers.poll(seconds(1000)), Some(IdleStage::Suspend));
        assert_eq!(timers.stage(), IdleStage::Suspend);
    }

    #[test]
    fn activity_resets_the_chain_to_active() {
        let mut timers = IdleTimers::new(chain_policy(), seconds(0));
        assert_eq!(timers.poll(seconds(12)), Some(IdleStage::Blank));
        assert_eq!(timers.activity(seconds(12)), Some(IdleStage::Active));
        assert_eq!(timers.stage(), IdleStage::Active);
        // Already active: no transition to report, but the timer still
        // restarts from the latest activity instant.
        assert_eq!(timers.activity(seconds(13)), None);
        assert_eq!(timers.poll(seconds(17)), None);
        assert_eq!(timers.poll(seconds(18)), Some(IdleStage::Dim));
    }

    #[test]
    fn disabled_stages_are_skipped() {
        let policy = IdlePolicy::disabled()
            .with_delay(IdleStage::Dim, Some(seconds(5)))
            .with_delay(IdleStage::Lock, Some(seconds(10)));
        let mut timers = IdleTimers::new(policy, seconds(0));
        assert_eq!(
            timers.next_deadline(),
            Some(seconds(5)),
            "the disabled blank stage is skipped"
        );
        assert_eq!(timers.poll(seconds(5)), Some(IdleStage::Dim));
        assert_eq!(
            timers.next_deadline(),
            Some(seconds(10)),
            "the next enabled stage is lock, not blank"
        );
        assert_eq!(timers.poll(seconds(10)), Some(IdleStage::Lock));
        assert_eq!(timers.next_deadline(), None, "suspend disabled");
    }

    #[test]
    fn a_disabled_policy_never_leaves_active() {
        let mut timers = IdleTimers::new(IdlePolicy::disabled(), seconds(0));
        assert_eq!(timers.poll(seconds(100_000)), None);
        assert_eq!(timers.stage(), IdleStage::Active);
        assert_eq!(timers.next_deadline(), None);
    }

    #[test]
    fn out_of_order_delays_are_clamped_to_the_chain() {
        // A lock delay earlier than dim is pushed to the dim deadline, so the
        // chain never locks before it dims.
        let policy = IdlePolicy::disabled()
            .with_delay(IdleStage::Dim, Some(seconds(10)))
            .with_delay(IdleStage::Lock, Some(seconds(5)));
        let normalized = policy.normalized();
        assert_eq!(normalized.delay(IdleStage::Dim), Some(seconds(10)));
        assert_eq!(normalized.delay(IdleStage::Lock), Some(seconds(10)));

        let mut timers = IdleTimers::new(policy, seconds(0));
        assert_eq!(timers.poll(seconds(9)), None);
        assert_eq!(timers.poll(seconds(10)), Some(IdleStage::Lock));
    }

    #[test]
    fn next_deadline_is_relative_to_the_last_activity() {
        let mut timers = IdleTimers::new(chain_policy(), seconds(100));
        assert_eq!(timers.next_deadline(), Some(seconds(105)));
        assert_eq!(timers.poll(seconds(105)), Some(IdleStage::Dim));
        assert_eq!(timers.next_deadline(), Some(seconds(110)));
        assert_eq!(timers.activity(seconds(106)), Some(IdleStage::Active));
        assert_eq!(timers.next_deadline(), Some(seconds(111)));
    }

    #[test]
    fn policy_changes_apply_live() {
        let mut timers = IdleTimers::new(chain_policy(), seconds(0));
        assert_eq!(timers.poll(seconds(4)), None);

        // Raising the dim delay keeps the engine active.
        let slower = chain_policy().with_delay(IdleStage::Dim, Some(seconds(30)));
        assert_eq!(timers.set_policy(slower, seconds(4)), None);
        assert_eq!(timers.next_deadline(), Some(seconds(30)));

        // Lowering a delay can fire immediately.
        let faster = chain_policy().with_delay(IdleStage::Dim, Some(seconds(1)));
        assert_eq!(timers.set_policy(faster, seconds(4)), Some(IdleStage::Dim));
    }

    #[test]
    fn from_keys_reads_seconds_and_disables_zero() {
        let policy = IdlePolicy::from_keys([
            ("idle.dim", "60"),
            ("idle.blank", "0"),
            ("idle.lock", "never"),
            ("idle.suspend", "1800"),
            ("unrelated", "7"),
        ]);
        assert_eq!(policy.delay(IdleStage::Dim), Some(seconds(60)));
        assert_eq!(policy.delay(IdleStage::Blank), None);
        assert_eq!(policy.delay(IdleStage::Lock), None);
        assert_eq!(policy.delay(IdleStage::Suspend), Some(seconds(1800)));
    }

    #[test]
    fn an_inhibitor_blocks_the_chain_until_released() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        let (id, event) = controller.acquire_inhibitor(seconds(0));
        assert_eq!(event, None, "already active");
        assert!(controller.is_inhibited());
        assert_eq!(controller.inhibitor_count(), 1);
        assert_eq!(controller.next_deadline(), None, "inhibited: no deadline");

        // The whole chain would be due, but the inhibitor freezes it.
        assert_eq!(controller.poll(seconds(1000)), None);
        assert_eq!(controller.stage(), IdleStage::Active);

        assert!(controller.release_inhibitor(id));
        assert!(!controller.is_inhibited());
        assert_eq!(controller.next_deadline(), Some(seconds(5)));
        assert_eq!(
            controller.poll(seconds(1000)),
            Some(IdleEvent::Enter(IdleStage::Suspend)),
            "the overdue chain catches up once released"
        );
    }

    #[test]
    fn activity_wakes_and_names_the_stage_to_restore() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        assert_eq!(
            controller.poll(seconds(12)),
            Some(IdleEvent::Enter(IdleStage::Blank))
        );
        assert_eq!(
            controller.activity(seconds(12)),
            Some(IdleEvent::Restore(IdleStage::Blank))
        );
        assert_eq!(controller.stage(), IdleStage::Active);
        assert_eq!(controller.idle_for(seconds(12)), Duration::ZERO);
        // Already active: activity restarts the clock but reports no restore.
        assert_eq!(controller.activity(seconds(13)), None);
    }

    #[test]
    fn a_new_inhibitor_wakes_an_idle_chain() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        assert_eq!(
            controller.poll(seconds(7)),
            Some(IdleEvent::Enter(IdleStage::Dim))
        );
        let (_id, event) = controller.acquire_inhibitor(seconds(7));
        assert_eq!(event, Some(IdleEvent::Restore(IdleStage::Dim)));
        assert_eq!(controller.stage(), IdleStage::Active);
    }

    #[test]
    fn releasing_one_of_many_inhibitors_keeps_the_chain_frozen() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        let (first, _) = controller.acquire_inhibitor(seconds(0));
        let (second, _) = controller.acquire_inhibitor(seconds(0));
        assert_eq!(controller.inhibitor_count(), 2);

        assert!(controller.release_inhibitor(first));
        assert!(
            controller.is_inhibited(),
            "the second inhibitor still holds"
        );
        assert_eq!(controller.poll(seconds(1000)), None);

        assert!(
            !controller.release_inhibitor(first),
            "double release is a no-op"
        );
        assert!(controller.release_inhibitor(second));
        assert!(!controller.is_inhibited());
    }

    #[test]
    fn a_stale_handle_never_releases_a_held_inhibitor() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        let (stale, _) = controller.acquire_inhibitor(seconds(0));
        assert!(controller.release_inhibitor(stale));
        let (held, _) = controller.acquire_inhibitor(seconds(0));
        assert!(!controller.release_inhibitor(stale), "already released");
        assert!(controller.is_inhibited());
        assert!(controller.inhibitors().contains(held));
    }

    #[test]
    fn a_policy_change_while_inhibited_does_not_advance() {
        let mut controller = IdleController::new(chain_policy(), seconds(0));
        let (_id, _) = controller.acquire_inhibitor(seconds(0));
        // A delay shorter than elapsed time would normally fire immediately.
        let faster = chain_policy_faster();
        assert_eq!(controller.set_policy(faster, seconds(100)), None);
        assert_eq!(controller.stage(), IdleStage::Active);
        assert_eq!(
            controller.policy().delay(IdleStage::Dim),
            Some(seconds(1)),
            "the new policy is stored for the next poll"
        );
    }

    fn chain_policy_faster() -> IdlePolicy {
        chain_policy().with_delay(IdleStage::Dim, Some(seconds(1)))
    }

    #[test]
    fn stage_names_round_trip_through_keys() {
        for stage in IdleStage::ALL {
            if stage == IdleStage::Active {
                continue;
            }
            assert_eq!(stage_for_key(key_for(stage)), Some(stage));
        }
        assert_eq!(stage_for_key("idle.nonsense"), None);
        assert_eq!(IdleStage::Active.next(), Some(IdleStage::Dim));
        assert_eq!(IdleStage::Suspend.next(), None);
    }
}
