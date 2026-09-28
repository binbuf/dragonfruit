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

use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor};

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
