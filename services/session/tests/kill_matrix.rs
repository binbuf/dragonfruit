// SPDX-License-Identifier: MIT
//! T-12.5b: the session kill matrix.
//!
//! The lock screen's fail-secure invariant is "every plausible crash during
//! lock leaves the session locked or ends it — never unlocked"
//! ([legacy/26](../../docs/tasks/legacy/26-lock-screen-idle.md)). The
//! compositor owns the lock itself: `compositor/tests/session_lock_conformance.rs`
//! proves input cannot reach clients while locked and that closing the lock
//! UI's socket leaves `locked=1`. This file covers the *supervision* half:
//! what the session manager does when each named service dies, using the same
//! [`Supervisor`] and the same restart vocabulary the shipped plan uses
//! ([ADR 0064](../../docs/design/adr/0064-session-manager-plan-and-restart-policy.md)).
//!
//! The documented outcomes asserted here:
//!
//! | Killed | Outcome |
//! |---|---|
//! | shell / lock UI | restarts (`always`); the session keeps running |
//! | settingsd | restarts (`on-failure`); the session keeps running |
//! | notification service | restarts (`on-failure`); the session keeps running |
//! | compositor | the session ends; every other service is stopped; never restarted |
//!
//! The shell is the lock UI (`shell/lock/LockScreen.qml`): killing it while
//! locked is a lock kill. The compositor's fail-secure closure of the lock is
//! asserted in the compositor conformance suite; here we assert the session
//! manager restarts it in place, so a locked session returns to a lock UI.

use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

/// Ticks the supervisor until `predicate` holds or `timeout` elapses.
fn drive_until(
    supervisor: &mut Supervisor,
    timeout: Duration,
    mut predicate: impl FnMut(&Supervisor) -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        supervisor.tick();
        if predicate(supervisor) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A long-lived stand-in for one service; `sleep`'s exit is not what we test.
fn sleeping(name: &str, stage: u32, policy: RestartPolicy) -> ServiceSpec {
    ServiceSpec::new(name, "sleep")
        .args(["30"])
        .stage(stage)
        .policy(policy)
}

/// The default session's composition with cheap stand-ins: the compositor
/// anchors stage 0, the shell/lock UI, settingsd, and the notification service
/// share stage 1 exactly as [`SessionPlan::default_session`] configures them.
fn matrix_plan() -> SessionPlan {
    SessionPlan::new(vec![
        ServiceSpec::new("compositor", "sleep")
            .args(["30"])
            .stage(0)
            .policy(RestartPolicy::Never)
            .ends_session(true)
            .gate(true),
        sleeping("shell", 1, RestartPolicy::Always),
        sleeping("settingsd", 1, RestartPolicy::OnFailure),
        sleeping("notifications", 1, RestartPolicy::OnFailure),
    ])
}

/// Start the plan and open the compositor gate so every stage is running.
fn running_session() -> Supervisor {
    let mut supervisor = Supervisor::new(matrix_plan());
    supervisor.start().expect("stage 0 starts");
    assert!(supervisor.set_ready("compositor"), "the socket appeared");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.state() == SessionState::Running
        }),
        "the session reaches Running"
    );
    supervisor
}

#[test]
fn the_kill_matrix_plan_matches_the_shipped_restart_policies() {
    let plan = matrix_plan();
    plan.validate().expect("the matrix plan is valid");
    let policy = |name: &str| {
        plan.services
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.policy)
            .unwrap()
    };
    assert_eq!(policy("shell"), RestartPolicy::Always, "the lock UI");
    assert_eq!(policy("settingsd"), RestartPolicy::OnFailure);
    assert_eq!(policy("notifications"), RestartPolicy::OnFailure);
    assert_eq!(policy("compositor"), RestartPolicy::Never, "the anchor");
}

#[test]
fn killing_the_shell_lock_ui_restarts_it_and_keeps_the_session() {
    let mut supervisor = running_session();
    let first = supervisor.running_pid("shell").expect("shell is running");

    assert!(supervisor.kill("shell"), "SIGKILL the shell/lock UI");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.restarts("shell") == Some(1) && s.running_pid("shell").is_some()
        }),
        "the shell restarts in place"
    );
    let second = supervisor.running_pid("shell").expect("a fresh shell pid");
    assert_ne!(first, second, "the restart is a new process");
    assert_eq!(
        supervisor.state(),
        SessionState::Running,
        "the session does not end"
    );

    supervisor.shutdown();
}

#[test]
fn killing_settingsd_restarts_it_and_keeps_the_session() {
    let mut supervisor = running_session();
    assert!(supervisor.kill("settingsd"), "SIGKILL settingsd");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.restarts("settingsd") == Some(1) && s.running_pid("settingsd").is_some()
        }),
        "settingsd restarts on failure"
    );
    assert_eq!(supervisor.state(), SessionState::Running);
    supervisor.shutdown();
}

#[test]
fn killing_the_notification_service_restarts_it_and_keeps_the_session() {
    let mut supervisor = running_session();
    assert!(
        supervisor.kill("notifications"),
        "SIGKILL the notification service"
    );
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.restarts("notifications") == Some(1) && s.running_pid("notifications").is_some()
        }),
        "the notification service restarts on failure"
    );
    assert_eq!(supervisor.state(), SessionState::Running);
    supervisor.shutdown();
}

#[test]
fn killing_the_compositor_ends_the_session_and_stops_everything_else() {
    let mut supervisor = running_session();
    let shell = supervisor.running_pid("shell").expect("shell is running");

    assert!(supervisor.kill("compositor"), "the compositor dies");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.state() == SessionState::Ended
        }),
        "a compositor exit ends the session"
    );
    assert_eq!(
        supervisor.restarts("compositor"),
        Some(0),
        "the anchor is never restarted"
    );
    for name in ["shell", "settingsd", "notifications"] {
        assert_eq!(
            supervisor.service_state(name),
            Some(ServiceState::Stopped),
            "{name} is torn down"
        );
        assert_eq!(supervisor.running_pid(name), None, "{name} has no pid");
    }
    assert!(
        supervisor.service_state("shell") == Some(ServiceState::Stopped),
        "the old shell pid {shell} is gone"
    );
}
