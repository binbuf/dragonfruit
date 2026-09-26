// SPDX-License-Identifier: MIT
//! T-12.4b acceptance: an inhibitor blocks the idle stage, and a wake restores
//! the prior state.
//!
//! The controller is clock-injected, so this test never sleeps: every instant
//! is a literal `Duration` and the whole chain runs in microseconds.

use std::time::Duration;

use dragonfruit_session::idle::{IdleController, IdleEvent, IdlePolicy, IdleStage};

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}

fn policy_from_keys(dim: u64, blank: u64, lock: u64, suspend: u64) -> IdlePolicy {
    IdlePolicy::from_keys([
        ("idle.dim", dim.to_string()),
        ("idle.blank", blank.to_string()),
        ("idle.lock", lock.to_string()),
        ("idle.suspend", suspend.to_string()),
    ])
}

#[test]
fn an_inhibitor_blocks_every_stage_until_it_is_released() {
    let mut controller = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));
    let (id, event) = controller.acquire_inhibitor(seconds(0));
    assert_eq!(event, None);
    assert_eq!(controller.next_deadline(), None);

    // Every deadline passes, but the inhibitor freezes the chain active.
    for at in [5, 10, 15, 20, 3600] {
        assert_eq!(controller.poll(seconds(at)), None, "inhibited at {at}s");
        assert_eq!(controller.stage(), IdleStage::Active);
    }

    assert!(controller.release_inhibitor(id));
    assert_eq!(
        controller.poll(seconds(3600)),
        Some(IdleEvent::Enter(IdleStage::Suspend)),
        "the overdue chain catches up after release"
    );
}

#[test]
fn wake_restores_the_stage_the_chain_left() {
    let mut controller = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));

    assert_eq!(
        controller.poll(seconds(11)),
        Some(IdleEvent::Enter(IdleStage::Blank))
    );
    assert_eq!(
        controller.activity(seconds(11)),
        Some(IdleEvent::Restore(IdleStage::Blank)),
        "wake names the stage to restore"
    );
    assert_eq!(controller.stage(), IdleStage::Active);

    // A fresh inhibitor is a wake too: it forces the chain back to the top.
    assert_eq!(
        controller.poll(seconds(26)),
        Some(IdleEvent::Enter(IdleStage::Lock))
    );
    let (id, event) = controller.acquire_inhibitor(seconds(26));
    assert_eq!(event, Some(IdleEvent::Restore(IdleStage::Lock)));
    assert_eq!(controller.stage(), IdleStage::Active);
    assert!(controller.inhibitors().contains(id));
}

#[test]
fn the_wake_from_lock_is_reported_but_not_an_unlock() {
    // Waking from `Lock` is a *screen* restore; the controller never clears
    // the lock itself, it only names the stage. The lock UI owns unlocking.
    let mut controller = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));
    assert_eq!(
        controller.poll(seconds(15)),
        Some(IdleEvent::Enter(IdleStage::Lock))
    );
    assert_eq!(
        controller.activity(seconds(16)),
        Some(IdleEvent::Restore(IdleStage::Lock))
    );
    assert_eq!(controller.stage(), IdleStage::Active);
}

#[test]
fn multiple_inhibitors_are_reference_counted() {
    let mut controller = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));
    let (first, _) = controller.acquire_inhibitor(seconds(0));
    let (second, _) = controller.acquire_inhibitor(seconds(0));
    assert_eq!(controller.inhibitor_count(), 2);

    assert!(controller.release_inhibitor(first));
    assert_eq!(
        controller.poll(seconds(100)),
        None,
        "one inhibitor still held"
    );
    assert!(controller.release_inhibitor(second));
    assert_eq!(
        controller.poll(seconds(100)),
        Some(IdleEvent::Enter(IdleStage::Suspend))
    );

    controller.acquire_inhibitor(seconds(200));
    controller.acquire_inhibitor(seconds(200));
    assert_eq!(controller.clear_inhibitors(), 2);
    assert!(!controller.is_inhibited());

    // A released handle stays released; a double release is a no-op.
    let (stale, _) = controller.acquire_inhibitor(seconds(200));
    assert!(controller.release_inhibitor(stale));
    assert!(!controller.release_inhibitor(stale));
}
