// SPDX-License-Identifier: MIT
//! T-12.5b / T-16.8a: the session kill matrix.
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
//! T-16.8a grows the T-12.5b matrix to the *whole* shipped composition, not a
//! hand-copied subset: the stand-in plan in [`shipped_plan_with_standins`] is
//! derived from [`SessionPlan::default_session`], so a new service or a
//! changed policy is exercised here without editing this file.
//!
//! The documented outcomes asserted here:
//!
//! | Killed | Outcome |
//! |---|---|
//! | shell / lock UI | restarts (`always`); the session keeps running |
//! | settingsd, menu-broker, app-index, notifications, wallpaperd | restart (`on-failure`); the session keeps running |
//! | portal backend | restarts (`on-failure`) and fails soft; the session keeps running |
//! | app (a user-launched client) | a plain client exit: not restarted, never a session event; the session keeps running |
//! | compositor | the session ends; every other service is stopped; never restarted |
//!
//! The shell is the lock UI (`shell/lock/LockScreen.qml`): killing it while
//! locked is a lock kill. The compositor's fail-secure closure of the lock is
//! asserted in the compositor conformance suite; here we assert the session
//! manager restarts it in place, so a locked session returns to a lock UI.
//! The compositor-death behavior itself is documented by T-16.8b.

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

/// The shipped composition ([`SessionPlan::default_session`]) with every real
/// program swapped for a long-lived `sleep`. Names, stages, policies, and the
/// anchor/gate/trusted flags are exactly the shipped plan, so this matrix
/// cannot drift from `services/session/src/plan.rs`.
fn shipped_plan_with_standins() -> SessionPlan {
    SessionPlan::new(
        SessionPlan::default_session()
            .services
            .into_iter()
            .map(|spec| {
                ServiceSpec::new(spec.name, "sleep")
                    .args(["30"])
                    .stage(spec.stage)
                    .policy(spec.policy)
                    .ends_session(spec.ends_session)
                    .gate(spec.gate)
                    .trusted(spec.trusted)
            })
            .collect(),
    )
}

/// The non-anchor services of the shipped plan — every one is restartable.
fn restartable_names() -> Vec<String> {
    SessionPlan::default_session()
        .services
        .into_iter()
        .filter(|spec| !spec.ends_session)
        .map(|spec| spec.name)
        .collect()
}

/// The shipped plan plus one app: a user-launched client the session does
/// **not** supervise. `Never` is the faithful policy — nothing restarts an
/// app, so its death is a plain client exit.
fn session_with_an_app() -> SessionPlan {
    let mut plan = shipped_plan_with_standins();
    plan.services.push(
        ServiceSpec::new("app", "sleep")
            .args(["30"])
            .stage(1)
            .policy(RestartPolicy::Never),
    );
    plan
}

/// Start a plan and open the compositor gate so every stage is running.
fn running_session_with(plan: SessionPlan) -> Supervisor {
    let mut supervisor = Supervisor::new(plan);
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

/// Start the shipped plan (stand-ins) with every stage up.
fn running_session() -> Supervisor {
    running_session_with(shipped_plan_with_standins())
}

#[test]
fn the_kill_matrix_plan_matches_the_shipped_restart_policies() {
    let plan = SessionPlan::default_session();
    plan.validate().expect("the shipped plan is valid");
    let policy = |name: &str| {
        plan.services
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.policy)
            .unwrap_or_else(|| panic!("{name} missing from the shipped plan"))
    };
    assert_eq!(
        policy("shell"),
        RestartPolicy::Always,
        "the shell / lock UI"
    );
    for name in [
        "settingsd",
        "menu-broker",
        "app-index",
        "notifications",
        "wallpaperd",
    ] {
        assert_eq!(policy(name), RestartPolicy::OnFailure, "{name}");
    }
    assert_eq!(
        policy("portal"),
        RestartPolicy::OnFailure,
        "the portal backend fails soft and restarts"
    );
    assert_eq!(policy("compositor"), RestartPolicy::Never, "the anchor");

    let portal = plan
        .services
        .iter()
        .find(|spec| spec.name == "portal")
        .expect("portal present");
    assert_eq!(
        portal.stage, 2,
        "the portal registers after the user services"
    );

    // Apps are clients the shell launches, not session services.
    assert!(
        !plan.services.iter().any(|spec| spec.name == "app"),
        "an app must never be a supervised session service"
    );
}

#[test]
fn every_restartable_service_restarts_in_place_and_keeps_the_session() {
    for name in restartable_names() {
        let mut supervisor = running_session();
        let first = supervisor
            .running_pid(&name)
            .unwrap_or_else(|| panic!("{name} is running"));
        assert!(supervisor.kill(&name), "SIGKILL {name}");
        assert!(
            drive_until(&mut supervisor, Duration::from_secs(5), |s| {
                s.restarts(&name) == Some(1) && s.running_pid(&name).is_some()
            }),
            "{name} restarts in place per its policy"
        );
        let second = supervisor.running_pid(&name).expect("a fresh pid");
        assert_ne!(first, second, "{name}'s restart is a new process");
        assert_eq!(
            supervisor.state(),
            SessionState::Running,
            "killing {name} must not end the session"
        );
        supervisor.shutdown();
    }
}

/// The matrix is complete: every restartable name is covered by the sweep
/// above. This guards against a new shipped service being silently skipped.
#[test]
fn the_restartable_set_is_exactly_the_shipped_services() {
    let mut names = restartable_names();
    names.sort();
    assert_eq!(
        names,
        vec![
            "app-index",
            "menu-broker",
            "notifications",
            "portal",
            "settingsd",
            "shell",
            "wallpaperd",
        ],
        "the kill matrix must cover every restartable shipped service"
    );
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
fn killing_the_portal_backend_restarts_it_and_keeps_the_session() {
    let mut supervisor = running_session();
    assert!(supervisor.kill("portal"), "SIGKILL the portal backend");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.restarts("portal") == Some(1) && s.running_pid("portal").is_some()
        }),
        "the portal backend restarts and fails soft"
    );
    assert_eq!(
        supervisor.state(),
        SessionState::Running,
        "a portal crash must never end the session"
    );
    // The user services are untouched by the portal death.
    for name in ["shell", "settingsd", "notifications"] {
        assert!(
            supervisor.running_pid(name).is_some(),
            "{name} is unaffected by the portal crash"
        );
    }
    supervisor.shutdown();
}

#[test]
fn killing_an_app_is_a_client_exit_not_a_session_event() {
    let mut supervisor = running_session_with(session_with_an_app());
    let app = supervisor.running_pid("app").expect("the app is running");

    assert!(supervisor.kill("app"), "SIGKILL the app client");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.service_state("app") == Some(ServiceState::Exited)
        }),
        "the app is recorded as a plain client exit"
    );
    assert_eq!(
        supervisor.restarts("app"),
        Some(0),
        "nothing restarts an app"
    );
    assert_eq!(
        supervisor.running_pid("app"),
        None,
        "the app pid {app} is gone"
    );
    assert_eq!(
        supervisor.state(),
        SessionState::Running,
        "an app crash is never a session event"
    );
    // The session's services keep running.
    for name in ["shell", "settingsd", "notifications", "portal"] {
        assert!(
            supervisor.running_pid(name).is_some(),
            "{name} keeps running after the app died"
        );
    }
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
    for name in restartable_names() {
        assert_eq!(
            supervisor.service_state(&name),
            Some(ServiceState::Stopped),
            "{name} is torn down"
        );
        assert_eq!(supervisor.running_pid(&name), None, "{name} has no pid");
    }
    assert!(
        supervisor.service_state("shell") == Some(ServiceState::Stopped),
        "the old shell pid {shell} is gone"
    );
}
