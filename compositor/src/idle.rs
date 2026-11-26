// SPDX-License-Identifier: GPL-3.0-only
#![allow(dead_code)] // The pure stage API is exercised by the lib conformance test.

//! The compositor's live inactivity timer (T315).
//!
//! The compositor is the only process that sees every input event, so it owns
//! the production idle chain: `Active → Dim → Blank → Lock → Suspend`
//! ([ADR 0313](../../docs/design/adr/0313-idle-dpms-ownership.md)). This module
//! is the pure, clock-injected part: [`IdleTimer`] takes an opaque monotonic
//! [`Duration`] and returns the stage transitions the caller turns into
//! `idle-dim`/`idle-blank`/`idle-lock`/`idle-suspend`/`idle-awake` actions.
//!
//! [`IdlePolicy`] and the stage rules deliberately duplicate
//! `services/session/src/idle.rs`, which remains the unit-test oracle
//! ([ADR 0070](../../docs/design/adr/0070-idle-timer-engine-and-policy.md)).
//! The two crates have no dependency edge and the compositor must not grow one
//! just for the timer, so a shared crate can be extracted later if the rules
//! grow ([ADR 0313](../../docs/design/adr/0313-idle-dpms-ownership.md)). The
//! conformance test `idle_policy_conformance` pins the duplicated rules against
//! the oracle's contract: ordered stages, skipped disabled stages, live policy
//! changes, inhibitor suppression, suspension, and wake restore.
//!
//! The timer is pure so a fake clock drives every stage without sleeping. The
//! live state syncs its `idle-inhibit` surfaces and the suspend model into
//! [`IdleTimer::set_inhibited`] / [`IdleTimer::set_suspended`], and arms one
//! calloop timer at [`IdleTimer::next_deadline`]; when no stage is enabled or
//! the chain is frozen there is no deadline and therefore no timer.

use std::time::Duration;

/// The schema's maximum idle delay, in whole seconds (`idle.*` max).
pub const MAX_IDLE_SECONDS: u32 = 86_400;

/// One step of the idle chain, from `Active` to `Suspend`.
///
/// The discriminants are ordered so `stage_a < stage_b` means `stage_a` is
/// earlier in the chain, matching the session engine's vocabulary.
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

    /// The stable `input_action` spelling for this stage's transition.
    ///
    /// `Active` is the wake action; the others are the stage actions the shell
    /// applies ([ADR 0313](../../docs/design/adr/0313-idle-dpms-ownership.md)).
    pub const fn action_name(self) -> &'static str {
        match self {
            IdleStage::Active => "idle-awake",
            IdleStage::Dim => "idle-dim",
            IdleStage::Blank => "idle-blank",
            IdleStage::Lock => "idle-lock",
            IdleStage::Suspend => "idle-suspend",
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

    /// Every stage crossed when the chain moves from `from` to `to`.
    ///
    /// A late tick can jump straight from `Active` to `Suspend`; the caller
    /// emits each crossed stage in order so the shell still dims, blanks, and
    /// locks before it suspends. A backward move (a live policy change that
    /// removes a stage) yields nothing.
    pub fn crossed(from: IdleStage, to: IdleStage) -> impl Iterator<Item = IdleStage> {
        IdleStage::ALL
            .into_iter()
            .filter(move |stage| *stage > from && *stage <= to)
    }
}

impl std::fmt::Display for IdleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

impl IdlePolicy {
    /// The shipped defaults, matching the session engine: dim 150 s, blank
    /// 300 s, lock 600 s, no automatic suspend.
    pub const fn new() -> Self {
        IdlePolicy {
            dim: Some(Duration::from_secs(150)),
            blank: Some(Duration::from_secs(300)),
            lock: Some(Duration::from_secs(600)),
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

    /// Build a policy from the four `idle.*` values in whole seconds.
    ///
    /// `0` disables a stage. Values above [`MAX_IDLE_SECONDS`] are clamped to
    /// the schema range, so a malformed or hostile request cannot arm a timer
    /// far past the settings contract. The policy is not normalized here;
    /// [`IdleTimer`] normalizes it when it is applied.
    pub fn from_seconds(dim: u32, blank: u32, lock: u32, suspend: u32) -> Self {
        let stage = |seconds: u32| {
            let seconds = seconds.min(MAX_IDLE_SECONDS);
            (seconds > 0).then(|| Duration::from_secs(u64::from(seconds)))
        };
        IdlePolicy {
            dim: stage(dim),
            blank: stage(blank),
            lock: stage(lock),
            suspend: stage(suspend),
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

    /// Whether any stage is enabled.
    pub const fn is_enabled(&self) -> bool {
        self.dim.is_some() || self.blank.is_some() || self.lock.is_some() || self.suspend.is_some()
    }

    /// A copy with the enabled delays forced into chain order.
    ///
    /// A stage's effective delay is never earlier than an enabled stage
    /// before it, so a policy cannot lock before it dims. A disabled stage
    /// does not constrain later stages, exactly as ADR 0070 specifies.
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
}

impl Default for IdlePolicy {
    fn default() -> Self {
        IdlePolicy::new()
    }
}

/// The idle-chain state machine for the compositor.
///
/// Time is an opaque monotonic [`Duration`] supplied by the caller. All delays
/// are measured from the last [`IdleTimer::activity`] (or construction). The
/// timer is frozen while inhibited or suspended: [`IdleTimer::poll`] does
/// nothing and [`IdleTimer::next_deadline`] is `None`.
#[derive(Debug, Clone)]
pub struct IdleTimer {
    policy: IdlePolicy,
    last_activity: Duration,
    stage: IdleStage,
    inhibited: bool,
    suspended: bool,
}

impl IdleTimer {
    /// Start the chain active at `now`, using `policy` (normalized).
    pub fn new(policy: IdlePolicy, now: Duration) -> Self {
        IdleTimer {
            policy: policy.normalized(),
            last_activity: now,
            stage: IdleStage::Active,
            inhibited: false,
            suspended: false,
        }
    }

    /// The timer's normalized policy.
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

    /// Whether the chain is frozen by an inhibitor.
    pub const fn is_inhibited(&self) -> bool {
        self.inhibited
    }

    /// Whether the chain is frozen by the suspended state.
    pub const fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// Record user activity at `now`.
    ///
    /// Resets the chain to [`IdleStage::Active`] and returns the stage that
    /// was left when the chain had started to idle (the caller emits
    /// `idle-awake`); returns `None` when it was already active. The clock is
    /// reset either way.
    pub fn activity(&mut self, now: Duration) -> Option<IdleStage> {
        self.last_activity = now;
        if self.stage == IdleStage::Active {
            None
        } else {
            let left = self.stage;
            self.stage = IdleStage::Active;
            Some(left)
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
    /// A single call jumps to the furthest due stage, matching the session
    /// oracle; the caller uses [`IdleStage::crossed`] to emit every stage it
    /// passed. A frozen chain (inhibited or suspended) never advances.
    pub fn poll(&mut self, now: Duration) -> Option<IdleStage> {
        if self.inhibited || self.suspended {
            return None;
        }
        let due = self.stage_at(now);
        if due == self.stage {
            return None;
        }
        self.stage = due;
        Some(due)
    }

    /// The instant the next enabled stage fires, if any.
    ///
    /// `None` when no stage is enabled, when every later stage is disabled,
    /// or while the chain is inhibited or suspended. This is the deadline a
    /// driver sleeps toward; the value may already be in the past when a tick
    /// is late, in which case [`IdleTimer::poll`] fires immediately.
    pub fn next_deadline(&self) -> Option<Duration> {
        if self.inhibited || self.suspended {
            return None;
        }
        let next = IdleStage::ALL
            .into_iter()
            .filter(|stage| *stage > self.stage)
            .find_map(|stage| self.policy.delay(stage).map(|delay| (stage, delay)))?;
        Some(self.last_activity.saturating_add(next.1))
    }

    /// Replace the policy live, re-evaluating the current stage at `now`.
    ///
    /// While the chain is frozen the new policy is stored but the chain is not
    /// advanced; the next [`IdleTimer::poll`] catches up, exactly like the
    /// session controller. Returns the stage that became due, mirroring
    /// [`IdleTimer::poll`].
    pub fn set_policy(&mut self, policy: IdlePolicy, now: Duration) -> Option<IdleStage> {
        self.policy = policy.normalized();
        self.poll(now)
    }

    /// Freeze or release the chain for an idle inhibitor.
    ///
    /// Acquiring the first inhibitor wakes the chain: if a stage was active it
    /// returns that stage so the caller emits `idle-awake`, and the inactivity
    /// clock restarts. Releasing the last inhibitor does not reset the clock;
    /// the next [`IdleTimer::poll`] catches up to the stage already due, which
    /// is what an inhibitor held over a deadline should do (ADR 0071).
    pub fn set_inhibited(&mut self, inhibited: bool, now: Duration) -> Option<IdleStage> {
        if self.inhibited == inhibited {
            return None;
        }
        self.inhibited = inhibited;
        if inhibited {
            self.activity(now)
        } else {
            None
        }
    }

    /// Freeze or release the chain for the suspend/resume cycle.
    ///
    /// Entering suspension leaves the stage where it is (usually `Suspend`)
    /// and produces no action. Resuming does not wake the chain by itself;
    /// the resume path records activity, which emits `idle-awake` when a stage
    /// was active.
    pub fn set_suspended(&mut self, suspended: bool) -> bool {
        if self.suspended == suspended {
            return false;
        }
        self.suspended = suspended;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    fn chain() -> IdlePolicy {
        IdlePolicy::from_seconds(5, 10, 15, 20)
    }

    #[test]
    fn each_stage_fires_at_its_configured_delay() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.stage(), IdleStage::Active);
        assert_eq!(timer.next_deadline(), Some(seconds(5)));

        assert_eq!(timer.poll(seconds(4)), None);
        for (at, expected) in [
            (5, IdleStage::Dim),
            (10, IdleStage::Blank),
            (15, IdleStage::Lock),
            (20, IdleStage::Suspend),
        ] {
            assert_eq!(timer.poll(seconds(at)), Some(expected), "at {at}s");
        }
        assert_eq!(timer.poll(seconds(3600)), None);
        assert_eq!(timer.next_deadline(), None);
    }

    #[test]
    fn disabled_stages_are_skipped_and_do_not_constrain_later_ones() {
        let mut timer = IdleTimer::new(IdlePolicy::from_seconds(5, 0, 10, 0), seconds(0));
        assert_eq!(timer.next_deadline(), Some(seconds(5)));
        assert_eq!(timer.poll(seconds(5)), Some(IdleStage::Dim));
        assert_eq!(timer.next_deadline(), Some(seconds(10)));
        assert_eq!(timer.poll(seconds(10)), Some(IdleStage::Lock));
        assert_eq!(timer.next_deadline(), None);

        // Out-of-order delays are clamped to the chain.
        let normalized = IdlePolicy::from_seconds(10, 0, 5, 0).normalized();
        assert_eq!(normalized.delay(IdleStage::Lock), Some(seconds(10)));
    }

    #[test]
    fn an_inhibitor_freezes_the_chain_until_released() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.set_inhibited(true, seconds(0)), None);
        assert!(timer.is_inhibited());
        assert_eq!(timer.next_deadline(), None);
        assert_eq!(timer.poll(seconds(1000)), None);
        assert_eq!(timer.stage(), IdleStage::Active);

        assert_eq!(timer.set_inhibited(false, seconds(1000)), None);
        assert!(!timer.is_inhibited());
        assert_eq!(
            timer.poll(seconds(1000)),
            Some(IdleStage::Suspend),
            "the overdue chain catches up once released"
        );
    }

    #[test]
    fn a_policy_change_applies_live() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.poll(seconds(4)), None);

        let slower = IdlePolicy::from_seconds(30, 10, 15, 20);
        assert_eq!(timer.set_policy(slower, seconds(4)), None);
        assert_eq!(timer.next_deadline(), Some(seconds(30)));

        let faster = IdlePolicy::from_seconds(1, 10, 15, 20);
        assert_eq!(timer.set_policy(faster, seconds(4)), Some(IdleStage::Dim));
    }

    #[test]
    fn a_policy_change_while_inhibited_does_not_advance() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.set_inhibited(true, seconds(0)), None);
        let faster = IdlePolicy::from_seconds(1, 10, 15, 20);
        assert_eq!(timer.set_policy(faster, seconds(100)), None);
        assert_eq!(timer.stage(), IdleStage::Active);
        assert_eq!(
            timer.policy().delay(IdleStage::Dim),
            Some(seconds(1)),
            "the new policy is stored for the next poll"
        );
    }

    #[test]
    fn activity_wakes_only_when_a_stage_was_active() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.poll(seconds(12)), Some(IdleStage::Blank));
        assert_eq!(
            timer.activity(seconds(12)),
            Some(IdleStage::Blank),
            "the stage that was left is the wake action"
        );
        assert_eq!(timer.stage(), IdleStage::Active);
        assert_eq!(timer.idle_for(seconds(12)), Duration::ZERO);
        assert_eq!(timer.activity(seconds(13)), None);
        assert_eq!(timer.poll(seconds(17)), None);
        assert_eq!(timer.poll(seconds(18)), Some(IdleStage::Dim));
    }

    #[test]
    fn a_new_inhibitor_wakes_an_idle_chain() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert_eq!(timer.poll(seconds(7)), Some(IdleStage::Dim));
        assert_eq!(timer.set_inhibited(true, seconds(7)), Some(IdleStage::Dim));
        assert_eq!(timer.stage(), IdleStage::Active);
    }

    #[test]
    fn a_suspended_chain_does_not_fire() {
        let mut timer = IdleTimer::new(chain(), seconds(0));
        assert!(timer.set_suspended(true));
        assert!(!timer.set_suspended(true), "idempotent");
        assert!(timer.is_suspended());
        assert_eq!(timer.next_deadline(), None);
        assert_eq!(timer.poll(seconds(1000)), None);

        assert!(timer.set_suspended(false));
        assert_eq!(timer.poll(seconds(1000)), Some(IdleStage::Suspend));
    }

    #[test]
    fn seconds_are_clamped_to_the_schema_range() {
        let policy = IdlePolicy::from_seconds(MAX_IDLE_SECONDS + 1, 0, 0, 0);
        assert_eq!(
            policy.delay(IdleStage::Dim),
            Some(Duration::from_secs(u64::from(MAX_IDLE_SECONDS)))
        );
        assert!(!IdlePolicy::from_seconds(0, 0, 0, 0).is_enabled());
        assert!(IdlePolicy::new().is_enabled());
    }

    #[test]
    fn crossed_stages_cover_a_late_tick() {
        assert_eq!(
            IdleStage::crossed(IdleStage::Active, IdleStage::Suspend).collect::<Vec<_>>(),
            vec![
                IdleStage::Dim,
                IdleStage::Blank,
                IdleStage::Lock,
                IdleStage::Suspend
            ]
        );
        assert_eq!(
            IdleStage::crossed(IdleStage::Blank, IdleStage::Lock).collect::<Vec<_>>(),
            vec![IdleStage::Lock]
        );
        assert_eq!(
            IdleStage::crossed(IdleStage::Lock, IdleStage::Dim).collect::<Vec<_>>(),
            Vec::<IdleStage>::new(),
            "a backward move emits nothing"
        );
    }

    #[test]
    fn stage_names_are_the_stable_action_strings() {
        assert_eq!(IdleStage::Active.action_name(), "idle-awake");
        assert_eq!(IdleStage::Dim.action_name(), "idle-dim");
        assert_eq!(IdleStage::Blank.action_name(), "idle-blank");
        assert_eq!(IdleStage::Lock.action_name(), "idle-lock");
        assert_eq!(IdleStage::Suspend.action_name(), "idle-suspend");
    }
}
