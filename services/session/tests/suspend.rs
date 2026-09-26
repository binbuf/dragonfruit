// SPDX-License-Identifier: MIT
//! T-12.5a acceptance: one suspend/resume cycle keeps the session alive.
//!
//! The session manager runs a real child process; the idle chain reaches
//! `Suspend`, the [`SuspendController`] requests a suspend through a mock
//! backend, logind's `PrepareForSleep` hook confirms sleep and then wake, and
//! the child is still running throughout. No real time passes: the idle chain
//! is clock-injected and the backend records calls.

use std::time::{Duration, Instant};

use dragonfruit_session::idle::{IdleController, IdleEvent, IdlePolicy, IdleStage};
use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{SessionState, Supervisor};
use dragonfruit_session::suspend::{
    MockSuspend, SuspendController, SuspendCycle, SuspendRequest, SuspendState,
};

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
fn one_suspend_resume_cycle_keeps_the_supervised_session_alive() {
    // A real child that outlives the test. The session manager owns it; the
    // suspend cycle must not restart it or end the session.
    let spec = ServiceSpec::new("sleeper", "sleep")
        .args(["30"])
        .policy(RestartPolicy::Never);
    let mut supervisor = Supervisor::new(SessionPlan::new(vec![spec]));
    supervisor.start().expect("spawn the sleeper");

    let deadline = Instant::now() + Duration::from_secs(5);
    while supervisor.state() != SessionState::Running {
        supervisor.tick();
        assert!(
            Instant::now() < deadline,
            "the session never reached Running: {:?}",
            supervisor.state()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid = supervisor
        .running_pid("sleeper")
        .expect("the sleeper is running");

    // The idle chain reaches `Suspend` and the controller requests a suspend.
    let mut idle = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));
    let mut suspend = SuspendController::new(MockSuspend::new());
    let event = idle
        .poll(seconds(20))
        .expect("the chain reaches Suspend at 20 s");
    assert_eq!(event, IdleEvent::Enter(IdleStage::Suspend));
    assert!(suspend.on_idle_event(event));
    assert_eq!(suspend.state(), SuspendState::Requested);
    assert_eq!(
        suspend.backend().suspend_requests(),
        1,
        "the platform is asked exactly once"
    );

    // logind confirms sleep; the session composition is untouched.
    assert!(suspend.prepare_for_sleep(true));
    assert!(suspend.is_asleep());
    supervisor.tick();
    assert_eq!(supervisor.state(), SessionState::Running);
    assert_eq!(supervisor.running_pid("sleeper"), Some(pid));

    // logind reports the wake; one cycle completed, the session still up.
    assert!(suspend.prepare_for_sleep(false));
    assert!(suspend.is_awake());
    assert_eq!(suspend.cycles(), 1);
    supervisor.tick();
    assert_eq!(supervisor.state(), SessionState::Running);
    assert_eq!(supervisor.running_pid("sleeper"), Some(pid));
    assert_eq!(supervisor.restarts("sleeper"), Some(0), "no restart");

    // User activity wakes the idle chain; a `Restore(Suspend)` is a no-op for
    // an already-awake cycle, and the session keeps running.
    let restored = idle
        .activity(seconds(25))
        .expect("activity restores the chain");
    assert_eq!(restored, IdleEvent::Restore(IdleStage::Suspend));
    assert!(!suspend.on_idle_event(restored));
    supervisor.tick();
    assert_eq!(supervisor.state(), SessionState::Running);
    assert_eq!(supervisor.running_pid("sleeper"), Some(pid));

    supervisor.shutdown();
    assert_eq!(supervisor.state(), SessionState::Ended);
}

#[test]
fn activity_before_sleep_lands_cancels_the_request() {
    // A user touch after the idle chain requested a suspend, but before the
    // platform confirmed it, aborts the request: the session never sleeps.
    let mut cycle = SuspendCycle::new();
    assert_eq!(cycle.request(), Some(SuspendRequest::Suspend));
    assert!(cycle.is_requested());
    assert_eq!(cycle.activity(), Some(SuspendRequest::Cancel));
    assert!(cycle.is_awake());
    assert_eq!(cycle.cycles(), 0);

    let mut controller = SuspendController::new(MockSuspend::new());
    controller.on_idle_event(IdleEvent::Enter(IdleStage::Suspend));
    assert!(controller.activity());
    assert!(controller.is_awake());
    assert_eq!(controller.backend().suspend_requests(), 1);
    assert_eq!(controller.backend().cancel_requests(), 1);
    assert_eq!(controller.cycles(), 0);
}

#[test]
fn the_full_idle_chain_requests_exactly_one_suspend() {
    let mut idle = IdleController::new(policy_from_keys(5, 10, 15, 20), seconds(0));
    let mut suspend = SuspendController::new(MockSuspend::new());

    // Every earlier stage passes through without touching the suspend seam.
    for (at, stage) in [
        (5, IdleStage::Dim),
        (10, IdleStage::Blank),
        (15, IdleStage::Lock),
    ] {
        let event = idle.poll(seconds(at)).expect("stage fires");
        assert_eq!(event, IdleEvent::Enter(stage));
        assert!(!suspend.on_idle_event(event), "{stage:?} is not suspend");
    }
    assert_eq!(suspend.backend().suspend_requests(), 0);

    let event = idle.poll(seconds(20)).expect("suspend fires");
    assert!(suspend.on_idle_event(event));
    // A later poll past the stage does not request again.
    assert_eq!(idle.poll(seconds(100)), None);
    assert_eq!(suspend.backend().suspend_requests(), 1);
}
