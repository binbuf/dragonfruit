// SPDX-License-Identifier: MIT
//! T-12.5b: the settingsd session-policy keys drive lock/idle/suspend.
//!
//! The idle/lock/suspend chain is pure and lives in `dragonfruit-session`
//! (`IdlePolicy`, `IdleController`, `SuspendController`). T-12.5b registers the
//! four `idle.*` keys in `dragonfruit-settingsd`; the engine's
//! `IdlePolicy::from_keys` is the seam that turns the settings snapshot into a
//! policy ([ADR 0070](../../docs/design/adr/0070-idle-timer-engine-and-policy.md)).
//! There is no production idle service yet, so this test is the headless proof
//! that the exact keys the daemon now serves move the chain — with a fake clock
//! and no real time passing.
//!
//! settingsd values arrive as typed D-Bus variants; the `idle.*` keys are
//! `x` (int64) whole seconds, so a consumer's snapshot bridge is
//! `value.to_string()`. These tests build that snapshot from the schema
//! defaults (150/300/600/0) and from explicit writes.

use std::time::Duration;

use dragonfruit_session::idle::{IdlePolicy, IdleStage, KEY_BLANK, KEY_DIM, KEY_LOCK, KEY_SUSPEND};
use dragonfruit_session::{IdleController, IdleEvent, MockSuspend, SuspendController};

fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}

/// The settingsd schema defaults for the four session keys (schema v5),
/// already converted the way a D-Bus client would (`Value::Integer` ->
/// `to_string`). These must match `IdlePolicy::new()`.
fn default_settings_pairs() -> Vec<(&'static str, String)> {
    vec![
        (KEY_DIM, 150.to_string()),
        (KEY_BLANK, 300.to_string()),
        (KEY_LOCK, 600.to_string()),
        (KEY_SUSPEND, 0.to_string()),
    ]
}

#[test]
fn the_settings_defaults_reproduce_the_shipped_idle_policy() {
    let policy = IdlePolicy::from_keys(default_settings_pairs());
    assert_eq!(policy, IdlePolicy::new());
    assert_eq!(policy.delay(IdleStage::Dim), Some(seconds(150)));
    assert_eq!(policy.delay(IdleStage::Blank), Some(seconds(300)));
    assert_eq!(policy.delay(IdleStage::Lock), Some(seconds(600)));
    assert_eq!(
        policy.delay(IdleStage::Suspend),
        None,
        "the default suspends never"
    );
}

#[test]
fn the_settings_keys_drive_every_stage_of_the_chain() {
    let policy = IdlePolicy::from_keys([
        (KEY_DIM, "5"),
        (KEY_BLANK, "10"),
        (KEY_LOCK, "15"),
        (KEY_SUSPEND, "20"),
    ]);
    let mut idle = IdleController::new(policy, Duration::ZERO);

    // Nothing fires before the first key's delay.
    assert_eq!(idle.poll(seconds(4)), None);
    assert_eq!(idle.stage(), IdleStage::Active);

    // Each key drives its stage, in order: dim, blank, lock, suspend.
    for (at, stage) in [
        (5, IdleStage::Dim),
        (10, IdleStage::Blank),
        (15, IdleStage::Lock),
        (20, IdleStage::Suspend),
    ] {
        assert_eq!(
            idle.poll(seconds(at)),
            Some(IdleEvent::Enter(stage)),
            "at {at}s"
        );
    }

    // The suspend key is the seam T-12.5a consumes: Enter(Suspend) requests a
    // platform suspend through the backend. A user wake names Suspend to
    // restore.
    let mut suspend = SuspendController::new(MockSuspend::new());
    assert!(suspend.on_idle_event(IdleEvent::Enter(IdleStage::Suspend)));
    assert_eq!(suspend.backend().suspend_requests(), 1);
    assert_eq!(
        idle.activity(seconds(21)),
        Some(IdleEvent::Restore(IdleStage::Suspend))
    );
    assert!(suspend.on_idle_event(IdleEvent::Restore(IdleStage::Suspend)));
    assert_eq!(suspend.backend().cancel_requests(), 1);
}

#[test]
fn a_zero_key_disables_only_its_stage() {
    // `idle.suspend = 0` is the shipped "never suspend"; lock still fires.
    let policy = IdlePolicy::from_keys([
        (KEY_DIM, "5"),
        (KEY_BLANK, "0"),
        (KEY_LOCK, "10"),
        (KEY_SUSPEND, "0"),
    ]);
    let mut idle = IdleController::new(policy, Duration::ZERO);
    assert_eq!(
        idle.poll(seconds(5)),
        Some(IdleEvent::Enter(IdleStage::Dim))
    );
    assert_eq!(
        idle.poll(seconds(10)),
        Some(IdleEvent::Enter(IdleStage::Lock)),
        "blank and suspend are disabled"
    );
    assert_eq!(idle.poll(seconds(100_000)), None);
}

#[test]
fn a_live_key_change_moves_the_chain() {
    let mut idle = IdleController::new(IdlePolicy::from_keys([(KEY_LOCK, "600")]), Duration::ZERO);
    // The user lowers the lock delay through the settings pane; the engine
    // applies the new snapshot live and locks at the new deadline.
    let changed = idle.set_policy(IdlePolicy::from_keys([(KEY_LOCK, "3")]), seconds(4));
    assert_eq!(changed, Some(IdleEvent::Enter(IdleStage::Lock)));
    assert_eq!(idle.stage(), IdleStage::Lock);
}

#[test]
fn an_idle_inhibitor_holds_the_keys_at_bay() {
    // A held inhibitor freezes the chain no matter how overdue the keys are.
    let policy = IdlePolicy::from_keys([
        (KEY_DIM, "1"),
        (KEY_BLANK, "2"),
        (KEY_LOCK, "3"),
        (KEY_SUSPEND, "4"),
    ]);
    let mut idle = IdleController::new(policy, Duration::ZERO);
    let (inhibitor, _) = idle.acquire_inhibitor(Duration::ZERO);
    assert_eq!(idle.poll(seconds(10_000)), None);
    assert_eq!(idle.stage(), IdleStage::Active);

    // Releasing catches the chain up to the furthest key already due.
    assert!(idle.release_inhibitor(inhibitor));
    assert_eq!(
        idle.poll(seconds(10_000)),
        Some(IdleEvent::Enter(IdleStage::Suspend))
    );
}
