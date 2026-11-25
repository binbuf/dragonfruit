// SPDX-License-Identifier: MIT
//! T-16.8b: the restart-policy matrix.
//!
//! [ADR 0064](../../docs/design/adr/0064-session-manager-plan-and-restart-policy.md)
//! freezes three [`RestartPolicy`] values (`Always`, `OnFailure`, `Never`) and
//! one rule for the session anchor: the compositor is never restarted and its
//! exit ends the session. This suite crosses every policy with every way a
//! child can leave (`exit 0`, a non-zero exit, and a fatal signal) and asserts
//! the documented restart decision, then crosses the anchor death with the same
//! three exits.
//!
//! The outcomes asserted here are the *restart policy* half of the T-16.8b
//! documentation; the session-ending observability and the live/VM drill are
//! recorded in [ADR 0158](../../docs/design/adr/0158-compositor-death-ends-the-session.md)
//! and `docs/captures/t16-kill-matrix.md`.
//!
//! | Policy \ Exit | `exit 0` | non-zero | signal |
//! |---|---|---|---|
//! | `always` | restart | restart | restart |
//! | `on-failure` | stay exited | restart | restart |
//! | `never` | stay exited | stay exited | stay exited |
//!
//! Every "service" is a real child process driven through the same
//! [`Supervisor`] the session binary uses, so a policy regression fails here
//! without a host session or a VM.
//!
//! T345 adds the backoff half of the matrix: a crash loop waits between
//! attempts, stops after the configured maximum, and resets its attempt count
//! after a stable run. The PGID-reuse teardown ordering is a `supervisor` unit
//! test, because it substitutes the kernel's process-id allocator.

use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{
    RestartBackoff, ServiceState, SessionState, Supervisor, SupervisorEvent,
};

/// How a matrix child leaves the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitKind {
    /// `exit 0` — a clean exit.
    Clean,
    /// A non-zero status — a failure.
    Failure,
    /// Killed by a signal (here `SIGKILL` to itself).
    Signal,
}

impl ExitKind {
    const ALL: [ExitKind; 3] = [ExitKind::Clean, ExitKind::Failure, ExitKind::Signal];

    /// The program and arguments that produce this exit.
    fn program(self) -> (&'static str, Vec<&'static str>) {
        match self {
            ExitKind::Clean => ("sh", vec!["-c", "exit 0"]),
            ExitKind::Failure => ("sh", vec!["-c", "exit 7"]),
            ExitKind::Signal => ("sh", vec!["-c", "kill -9 $$"]),
        }
    }

    /// Whether this exit is a failure for [`RestartPolicy::OnFailure`].
    fn is_failure(self) -> bool {
        !matches!(self, ExitKind::Clean)
    }
}

/// The documented restart decision for one policy/exit cell.
fn expected_restart(policy: RestartPolicy, exit: ExitKind) -> bool {
    match policy {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => exit.is_failure(),
        RestartPolicy::Never => false,
    }
}

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

/// One non-anchor service that leaves in exactly `exit`'s way.
fn one_shot(policy: RestartPolicy, exit: ExitKind) -> ServiceSpec {
    let (program, args) = exit.program();
    ServiceSpec::new("worker", program)
        .args(args)
        .policy(policy)
}

/// The compositor-death matrix: every exit kind ends the session and stops the
/// survivors, and the anchor is never restarted.
#[test]
fn the_compositor_exit_ends_the_session_for_every_exit_kind() {
    for exit in ExitKind::ALL {
        let (program, args) = exit.program();
        let plan = SessionPlan::new(vec![
            ServiceSpec::new("compositor", program)
                .args(args)
                .policy(RestartPolicy::Never)
                .ends_session(true),
            ServiceSpec::new("shell", "sleep")
                .args(["30"])
                .stage(1)
                .policy(RestartPolicy::Always),
        ]);
        assert!(
            plan.validate().is_ok(),
            "{exit:?}: the anchor is a valid Never anchor"
        );

        let mut supervisor = Supervisor::new(plan);
        supervisor.start().expect("stage 0 starts");
        assert_eq!(
            supervisor.service_state("shell"),
            Some(ServiceState::Running),
            "{exit:?}: the shell starts behind the anchor"
        );

        assert!(
            drive_until(&mut supervisor, Duration::from_secs(5), |s| {
                s.state() == SessionState::Ended
            }),
            "{exit:?}: compositor death ends the session"
        );
        assert_eq!(
            supervisor.restarts("compositor"),
            Some(0),
            "{exit:?}: the anchor is never restarted"
        );
        assert_eq!(
            supervisor.service_state("shell"),
            Some(ServiceState::Stopped),
            "{exit:?}: the surviving services are torn down"
        );
        assert_eq!(
            supervisor.running_pid("shell"),
            None,
            "{exit:?}: no survivor keeps a pid"
        );
    }
}

/// The policy matrix: every `RestartPolicy` against every exit kind.
#[test]
fn restart_policy_matrix_matches_the_documented_decisions() {
    for policy in [
        RestartPolicy::Always,
        RestartPolicy::OnFailure,
        RestartPolicy::Never,
    ] {
        for exit in ExitKind::ALL {
            let mut supervisor = Supervisor::new(SessionPlan::new(vec![one_shot(policy, exit)]));
            supervisor.start().expect("the worker starts");

            if expected_restart(policy, exit) {
                assert!(
                    drive_until(&mut supervisor, Duration::from_secs(5), |s| {
                        s.restarts("worker").unwrap_or(0) >= 1 && s.running_pid("worker").is_some()
                    }),
                    "{policy:?}/{exit:?}: the service restarts per policy"
                );
                assert_eq!(
                    supervisor.state(),
                    SessionState::Running,
                    "{policy:?}/{exit:?}: the session keeps running"
                );
            } else {
                assert!(
                    drive_until(&mut supervisor, Duration::from_secs(5), |s| {
                        s.service_state("worker") == Some(ServiceState::Exited)
                    }),
                    "{policy:?}/{exit:?}: the service exits and stays exited"
                );
                assert_eq!(
                    supervisor.restarts("worker"),
                    Some(0),
                    "{policy:?}/{exit:?}: nothing restarts it"
                );
                assert_eq!(
                    supervisor.running_pid("worker"),
                    None,
                    "{policy:?}/{exit:?}: no pid survives"
                );
            }

            supervisor.shutdown();
        }
    }
}

/// The shipped anchor is `Never` + `ends_session`; a restartable anchor is a
/// plan error, so the matrix above can never be satisfied by restarting it.
#[test]
fn the_shipped_anchor_is_the_compositor_and_is_never_restartable() {
    let plan = SessionPlan::default_session();
    let anchor = plan.anchor().expect("the compositor is the anchor");
    assert_eq!(anchor.name, "compositor");
    assert!(anchor.ends_session, "the anchor ends the session");
    assert_eq!(
        anchor.policy,
        RestartPolicy::Never,
        "a restartable anchor could not end the session"
    );

    let restartable_anchor = SessionPlan::new(vec![ServiceSpec::new("compositor", "x")
        .policy(RestartPolicy::Always)
        .ends_session(true)]);
    assert!(
        restartable_anchor.validate().is_err(),
        "a restartable anchor is rejected outright"
    );
}

/// T345: a crash loop waits between restarts instead of respawning on every
/// 50 ms tick. The delay doubles from the base and is clamped at the cap.
#[test]
fn a_crash_loop_backs_off_between_restarts() {
    let backoff = RestartBackoff {
        base: Duration::from_millis(120),
        max_delay: Duration::from_millis(480),
        max_attempts: 4,
        stable_run: Duration::from_secs(60),
    };
    let mut supervisor = Supervisor::with_backoff(
        SessionPlan::new(vec![one_shot(RestartPolicy::Always, ExitKind::Failure)]),
        backoff,
    );
    supervisor.start().expect("the worker starts");

    let mut delays: Vec<Duration> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while delays.len() < 3 {
        supervisor.tick();
        let attempts = supervisor.restart_attempts("worker").unwrap_or(0);
        if attempts as usize > delays.len() {
            let at = supervisor
                .next_restart_at("worker")
                .unwrap_or_else(|| panic!("attempt {attempts} schedules a restart"));
            delays.push(at.saturating_duration_since(Instant::now()));
        }
        assert!(Instant::now() < deadline, "three restarts are scheduled");
        std::thread::sleep(Duration::from_millis(5));
    }

    // 120 ms, 240 ms, 480 ms; the lower bounds leave room for scheduler lag
    // but a constant or missing backoff still fails.
    assert!(delays[0] >= Duration::from_millis(60), "{delays:?}");
    assert!(delays[1] >= Duration::from_millis(120), "{delays:?}");
    assert!(delays[2] >= Duration::from_millis(240), "{delays:?}");
    assert!(
        delays[0] < delays[1] && delays[1] < delays[2],
        "each restart waits longer than the last: {delays:?}"
    );

    supervisor.shutdown();
}

/// T345: a crash loop stops after [`RestartBackoff::max_attempts`] and stays
/// exited instead of respawning forever.
#[test]
fn a_crash_loop_stops_after_the_max_attempts() {
    let backoff = RestartBackoff {
        base: Duration::from_millis(10),
        max_delay: Duration::from_millis(20),
        max_attempts: 3,
        stable_run: Duration::from_secs(60),
    };
    let mut supervisor = Supervisor::with_backoff(
        SessionPlan::new(vec![one_shot(RestartPolicy::Always, ExitKind::Failure)]),
        backoff,
    );
    supervisor.start().expect("the worker starts");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut events = Vec::new();
    loop {
        events.extend(supervisor.tick());
        let abandoned = supervisor.restart_attempts("worker") == Some(3)
            && supervisor.next_restart_at("worker").is_none()
            && supervisor.service_state("worker") == Some(ServiceState::Exited);
        if abandoned {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the crash loop reaches its attempt limit"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(
        supervisor.restarts("worker"),
        Some(3),
        "exactly the max attempts ran"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            SupervisorEvent::RestartAbandoned { name, attempts: 3 } if name == "worker"
        )),
        "the supervisor reports that it gave up: {events:?}"
    );

    // More ticks never resurrect the service.
    for _ in 0..20 {
        supervisor.tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(supervisor.restarts("worker"), Some(3));
    assert_eq!(supervisor.next_restart_at("worker"), None);
    assert_eq!(
        supervisor.service_state("worker"),
        Some(ServiceState::Exited),
        "the crash loop stays stopped"
    );

    supervisor.shutdown();
}

/// T345: a run at least `stable_run` long is not a crash, so the attempt
/// counter resets and a service that fails later still restarts.
#[test]
fn a_stable_run_resets_the_restart_attempts() {
    let backoff = RestartBackoff {
        base: Duration::from_millis(5),
        max_delay: Duration::from_millis(10),
        max_attempts: 2,
        stable_run: Duration::from_millis(60),
    };
    let plan = SessionPlan::new(vec![ServiceSpec::new("worker", "sh")
        .args(["-c", "sleep 0.1; exit 7"])
        .policy(RestartPolicy::Always)]);
    let mut supervisor = Supervisor::with_backoff(plan, backoff);
    supervisor.start().expect("the worker starts");

    assert!(
        drive_until(&mut supervisor, Duration::from_secs(10), |s| {
            s.restarts("worker").is_some_and(|count| count >= 4)
        }),
        "a stable run lets the service restart again past max_attempts"
    );
    assert!(
        supervisor
            .restart_attempts("worker")
            .is_some_and(|a| a <= 1),
        "the counter resets after each stable run"
    );

    supervisor.shutdown();
}
