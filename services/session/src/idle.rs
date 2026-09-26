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
//! Inhibitors (a client holding the screen awake) and wake restore are
//! **not** here: they are T-12.4b, which wraps this engine. The engine itself
//! only spends time; a caller with an inhibitor simply declines to call
//! [`IdleTimers::poll`].
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
