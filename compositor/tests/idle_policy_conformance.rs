// SPDX-License-Identifier: GPL-3.0-only
//! T315 acceptance: the compositor's idle timer applies the `idle.*` policy
//! with a fake clock.
//!
//! The live timer is `DfState`'s one calloop source, but the stage rules are
//! the pure [`dragonfruit_compositor::idle`] engine. This test drives that
//! engine with an opaque monotonic clock so no real time passes, and pins the
//! contract the shell consumes:
//!
//! * the four stages fire at their configured delays, in chain order;
//! * a disabled stage (`0`) is skipped and does not constrain later stages;
//! * out-of-order delays are normalized to the chain;
//! * an inhibitor freezes the chain and a fresh inhibitor wakes it;
//! * a policy change applies live (shorter fires, longer waits);
//! * any activity wakes only when a stage was active;
//! * the suspended state freezes the chain;
//! * the four values are clamped to the schema's 0..86400 range;
//! * the stable `input_action` spellings are exactly ADR 0313's names.
//!
//! `services/session/src/idle.rs` remains the unit-test oracle (ADR 0070); the
//! compositor's copy is the production timer (ADR 0313), and this test is the
//! regression guard on the duplicated rules.

use std::time::Duration;

use dragonfruit_compositor::idle::{IdlePolicy, IdleStage, IdleTimer, MAX_IDLE_SECONDS};

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}

/// dim 5 s, blank 10 s, lock 15 s, suspend 20 s.
fn chain() -> IdlePolicy {
    IdlePolicy::from_seconds(5, 10, 15, 20)
}

#[test]
fn the_four_stages_fire_at_their_configured_delays() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert_eq!(timer.stage(), IdleStage::Active);
    assert_eq!(timer.next_deadline(), Some(seconds(5)));

    // The tick before a deadline changes nothing.
    assert_eq!(timer.poll(seconds(4)), None);
    assert_eq!(timer.stage(), IdleStage::Active);

    for (at, expected) in [
        (5, IdleStage::Dim),
        (10, IdleStage::Blank),
        (15, IdleStage::Lock),
        (20, IdleStage::Suspend),
    ] {
        assert_eq!(timer.poll(seconds(at)), Some(expected), "at {at}s");
        assert_eq!(timer.stage(), expected, "at {at}s");
    }

    // Past the last stage the chain stays put and has no further deadline.
    assert_eq!(timer.poll(seconds(3600)), None);
    assert_eq!(timer.stage(), IdleStage::Suspend);
    assert_eq!(timer.next_deadline(), None);
}

#[test]
fn a_late_tick_crosses_every_stage_in_order() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert_eq!(timer.poll(seconds(1000)), Some(IdleStage::Suspend));
    assert_eq!(
        IdleStage::crossed(IdleStage::Active, IdleStage::Suspend).collect::<Vec<_>>(),
        vec![
            IdleStage::Dim,
            IdleStage::Blank,
            IdleStage::Lock,
            IdleStage::Suspend
        ],
        "the shell still dims, blanks, and locks before it suspends"
    );
}

#[test]
fn disabled_stages_are_skipped_and_do_not_constrain_later_stages() {
    let mut timer = IdleTimer::new(IdlePolicy::from_seconds(5, 0, 10, 0), seconds(0));
    assert_eq!(
        timer.next_deadline(),
        Some(seconds(5)),
        "dim is the first enabled stage"
    );
    assert_eq!(timer.poll(seconds(5)), Some(IdleStage::Dim));
    assert_eq!(
        timer.next_deadline(),
        Some(seconds(10)),
        "blank is disabled, so lock is next"
    );
    assert_eq!(timer.poll(seconds(10)), Some(IdleStage::Lock));
    assert_eq!(timer.next_deadline(), None, "suspend is disabled");

    // A disabled stage does not clamp a later stage.
    let normalized = IdlePolicy::from_seconds(10, 0, 5, 0).normalized();
    assert_eq!(normalized.delay(IdleStage::Dim), Some(seconds(10)));
    assert_eq!(
        normalized.delay(IdleStage::Lock),
        Some(seconds(10)),
        "lock is clamped to the earlier enabled dim, not a disabled blank"
    );
}

#[test]
fn an_inhibitor_freezes_the_chain_until_released() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert_eq!(timer.set_inhibited(true, seconds(0)), None);
    assert!(timer.is_inhibited());
    assert_eq!(timer.next_deadline(), None, "inhibited: no deadline");

    // The whole chain would be due, but the inhibitor freezes it.
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
fn a_fresh_inhibitor_wakes_an_idle_chain() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert_eq!(timer.poll(seconds(7)), Some(IdleStage::Dim));
    assert_eq!(
        timer.set_inhibited(true, seconds(7)),
        Some(IdleStage::Dim),
        "the stage that was left is the wake action"
    );
    assert_eq!(timer.stage(), IdleStage::Active);
}

#[test]
fn a_policy_change_applies_live() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert_eq!(timer.poll(seconds(4)), None);

    // Raising the dim delay keeps the engine active and moves the deadline.
    let slower = IdlePolicy::from_seconds(30, 10, 15, 20);
    assert_eq!(timer.set_policy(slower, seconds(4)), None);
    assert_eq!(timer.next_deadline(), Some(seconds(30)));

    // Lowering a delay can fire immediately.
    let faster = IdlePolicy::from_seconds(1, 10, 15, 20);
    assert_eq!(timer.set_policy(faster, seconds(4)), Some(IdleStage::Dim));
    assert_eq!(timer.stage(), IdleStage::Dim);
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
        "waking from blank emits idle-awake"
    );
    assert_eq!(timer.stage(), IdleStage::Active);
    assert_eq!(timer.idle_for(seconds(12)), Duration::ZERO);

    // Already active: the clock restarts, but there is no wake action.
    assert_eq!(timer.activity(seconds(13)), None);
    assert_eq!(timer.poll(seconds(17)), None);
    assert_eq!(timer.poll(seconds(18)), Some(IdleStage::Dim));
}

#[test]
fn a_suspended_chain_does_not_fire() {
    let mut timer = IdleTimer::new(chain(), seconds(0));
    assert!(timer.set_suspended(true));
    assert!(!timer.set_suspended(true), "idempotent");
    assert_eq!(timer.next_deadline(), None);
    assert_eq!(timer.poll(seconds(1000)), None);

    assert!(timer.set_suspended(false));
    assert_eq!(timer.poll(seconds(1000)), Some(IdleStage::Suspend));
}

#[test]
fn values_are_clamped_to_the_schema_range() {
    let policy = IdlePolicy::from_seconds(MAX_IDLE_SECONDS + 100, 0, 0, 0);
    assert_eq!(
        policy.delay(IdleStage::Dim),
        Some(Duration::from_secs(u64::from(MAX_IDLE_SECONDS)))
    );
    assert!(!IdlePolicy::from_seconds(0, 0, 0, 0).is_enabled());
    assert!(IdlePolicy::new().is_enabled());
}

#[test]
fn stage_action_names_are_stable() {
    assert_eq!(IdleStage::Active.action_name(), "idle-awake");
    assert_eq!(IdleStage::Dim.action_name(), "idle-dim");
    assert_eq!(IdleStage::Blank.action_name(), "idle-blank");
    assert_eq!(IdleStage::Lock.action_name(), "idle-lock");
    assert_eq!(IdleStage::Suspend.action_name(), "idle-suspend");
}
