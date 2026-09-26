// SPDX-License-Identifier: MIT
//! T-12.4a acceptance: a fake clock drives each idle stage at its configured
//! delay, and activity restarts the chain.
//!
//! The engine is clock-injected, so this test never sleeps: every instant is a
//! literal `Duration` and the whole chain runs in microseconds.

use std::time::Duration;

use dragonfruit_session::idle::{IdlePolicy, IdleStage, IdleTimers};

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}

/// Build a policy the way T-12.5b will: from `idle.*` policy keys.
fn policy_from_keys(dim: u64, blank: u64, lock: u64, suspend: u64) -> IdlePolicy {
    IdlePolicy::from_keys([
        ("idle.dim", dim.to_string()),
        ("idle.blank", blank.to_string()),
        ("idle.lock", lock.to_string()),
        ("idle.suspend", suspend.to_string()),
    ])
}

#[test]
fn each_idle_stage_fires_at_its_configured_delay() {
    let policy = policy_from_keys(5, 10, 15, 20);
    let mut timers = IdleTimers::new(policy, seconds(0));

    assert_eq!(timers.stage(), IdleStage::Active);
    assert_eq!(
        timers.next_deadline(),
        Some(seconds(5)),
        "the first deadline is the dim delay"
    );

    // One second before each deadline, nothing has happened yet.
    for (at, expected) in [
        (4, IdleStage::Active),
        (9, IdleStage::Dim),
        (14, IdleStage::Blank),
        (19, IdleStage::Lock),
    ] {
        assert_eq!(timers.stage_at(seconds(at)), expected, "at {at}s");
    }

    // At each deadline the matching stage fires exactly once.
    for (at, expected) in [
        (5, IdleStage::Dim),
        (10, IdleStage::Blank),
        (15, IdleStage::Lock),
        (20, IdleStage::Suspend),
    ] {
        assert_eq!(timers.poll(seconds(at)), Some(expected), "at {at}s");
        assert_eq!(timers.poll(seconds(at)), None, "fired once at {at}s");
    }
}

#[test]
fn activity_restarts_the_chain_and_wake_returns_to_active() {
    let policy = policy_from_keys(5, 10, 15, 20);
    let mut timers = IdleTimers::new(policy, seconds(0));

    assert_eq!(timers.poll(seconds(11)), Some(IdleStage::Blank));
    assert_eq!(timers.activity(seconds(11)), Some(IdleStage::Active));
    assert_eq!(timers.stage(), IdleStage::Active);
    assert_eq!(timers.idle_for(seconds(11)), Duration::ZERO);

    // The dim delay runs again from the activity instant.
    assert_eq!(timers.poll(seconds(15)), None);
    assert_eq!(timers.poll(seconds(16)), Some(IdleStage::Dim));
}

#[test]
fn a_zero_key_disables_that_stage() {
    let policy = policy_from_keys(5, 0, 10, 0);
    let mut timers = IdleTimers::new(policy, seconds(0));

    assert_eq!(timers.poll(seconds(5)), Some(IdleStage::Dim));
    assert_eq!(
        timers.poll(seconds(10)),
        Some(IdleStage::Lock),
        "blank is skipped"
    );
    assert_eq!(timers.next_deadline(), None, "nothing after lock");
}
