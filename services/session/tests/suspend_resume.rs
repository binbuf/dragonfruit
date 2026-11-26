// SPDX-License-Identifier: GPL-3.0-or-later
//! T318: an external suspend (a lid close or power button) needs no request
//! from this session. logind emits `PrepareForSleep(true)` anyway, the cycle
//! confirms sleep, and the wake counts one cycle without unlocking anything.
//!
//! The shell half of T318 is `shell/tests/tst_locksuspend.cpp`; this file
//! proves the session cycle accepts the same hook without an idle-chain
//! `Enter(Suspend)` and never asks the platform to suspend again.

use dragonfruit_session::suspend::{MockSuspend, SuspendController, SuspendState};

#[test]
fn a_lid_close_suspends_without_a_request_and_wakes_awake() {
    let mut suspend = SuspendController::new(MockSuspend::new());
    assert_eq!(suspend.state(), SuspendState::Awake);

    // logind's hook arrives without `Enter(Suspend)` and without a backend
    // request: the machine is already going to sleep.
    assert!(suspend.prepare_for_sleep(true));
    assert!(suspend.is_asleep());
    assert_eq!(
        suspend.backend().suspend_requests(),
        0,
        "an external suspend must not ask logind to suspend again"
    );

    // A duplicate hook is a no-op.
    assert!(!suspend.prepare_for_sleep(true));
    assert!(suspend.is_asleep());
    assert_eq!(suspend.cycles(), 0);

    // The wake counts one cycle; it is not an unlock and never calls the
    // backend. The session stays awake until the next request.
    assert!(suspend.prepare_for_sleep(false));
    assert!(suspend.is_awake());
    assert_eq!(suspend.cycles(), 1);
    assert_eq!(suspend.backend().suspend_requests(), 0);
    assert_eq!(suspend.backend().cancel_requests(), 0);

    // A duplicate wake does not count twice.
    assert!(!suspend.prepare_for_sleep(false));
    assert_eq!(suspend.cycles(), 1);
}

#[test]
fn a_requested_suspend_still_uses_the_backend_once() {
    // The menu/idle path is unchanged: one request, one backend call, then
    // logind's confirmation and wake.
    let mut suspend = SuspendController::new(MockSuspend::new());
    assert!(
        suspend.on_idle_event(dragonfruit_session::idle::IdleEvent::Enter(
            dragonfruit_session::idle::IdleStage::Suspend
        ))
    );
    assert_eq!(suspend.state(), SuspendState::Requested);
    assert_eq!(suspend.backend().suspend_requests(), 1);

    assert!(suspend.prepare_for_sleep(true));
    assert!(suspend.prepare_for_sleep(false));
    assert!(suspend.is_awake());
    assert_eq!(suspend.cycles(), 1);
    assert_eq!(suspend.backend().suspend_requests(), 1);
}
