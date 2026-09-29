// SPDX-License-Identifier: MIT
//! T-17.5a: an absent service degrades the session, it never blocks its start.
//!
//! The T-15.16 absence matrix proves every *pane* renders with its provider
//! gone; the T-16.8a kill matrix proves every *supervised service* recovers
//! when it dies. This suite is the premium gate's third absence question:
//! **a service that cannot even be spawned must not block the session.**
//!
//! A missing binary (or a daemon that fails to exec) is a spawn failure, not a
//! supervised death: the [`Supervisor`] records [`ServiceState::Failed`] and
//! moves on. Only the compositor — the one `ends_session` anchor and the one
//! `gate` — is load-bearing; its absence ends the session by design
//! ([ADR 0158](../../docs/design/adr/0158-compositor-death-ends-the-session.md)),
//! and every other absence is a degraded-but-running desktop.
//!
//! The stand-in plan below is derived from [`SessionPlan::default_session`], so
//! a new shipped service is exercised here without editing this file.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor, SupervisorEvent};

/// A program that is guaranteed not to exist, so `Command::spawn` fails with
/// `ENOENT` — the "absent daemon" a session can meet at startup.
const ABSENT: &str = "dragonfruit-definitely-absent-service-xyz";

/// The shipped composition with every program swapped for a stand-in: the
/// anchor becomes a long-lived `sleep` (so its gate can be opened), and every
/// other service becomes the absent binary. Names, stages, policies, and the
/// anchor/gate/trusted flags are exactly the shipped plan.
fn plan_with_every_optional_service_absent() -> SessionPlan {
    SessionPlan::new(
        SessionPlan::default_session()
            .services
            .into_iter()
            .map(|spec| {
                let mut spec = spec;
                if spec.ends_session {
                    spec.program = PathBuf::from("sleep");
                    spec.args = vec!["30".to_string()];
                } else {
                    spec.program = PathBuf::from(ABSENT);
                }
                spec
            })
            .collect(),
    )
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

/// The services the session can lose and keep running — every non-anchor.
fn optional_names() -> Vec<String> {
    SessionPlan::default_session()
        .services
        .into_iter()
        .filter(|spec| !spec.ends_session)
        .map(|spec| spec.name)
        .collect()
}

/// The compositor is the only service whose readiness gates the next stage, so
/// no optional absence can ever hold the launch order open. This is the
/// structural reason "no absence blocks start" holds for the shipped plan.
#[test]
fn the_only_gate_in_the_shipped_plan_is_the_compositor() {
    let gates: Vec<String> = SessionPlan::default_session()
        .services
        .into_iter()
        .filter(|spec| spec.gate)
        .map(|spec| spec.name)
        .collect();
    assert_eq!(
        gates,
        vec!["compositor".to_string()],
        "only the anchor may gate the launch order"
    );
    let shipped = SessionPlan::default_session();
    let anchor = shipped.anchor().expect("the compositor is the anchor");
    assert_eq!(anchor.name, "compositor");
    assert!(anchor.gate, "the anchor is the gate it is");
}

/// The acceptance case: every optional service absent at startup, and the
/// session still reaches [`SessionState::Running`]. Nothing blocks start.
#[test]
fn every_absent_optional_service_degrades_and_never_blocks_start() {
    let mut supervisor = Supervisor::new(plan_with_every_optional_service_absent());
    supervisor
        .start()
        .expect("start never fails on an absent service");
    assert!(
        supervisor.set_ready("compositor"),
        "the anchor's gate opens like any other"
    );
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.state() == SessionState::Running
        }),
        "the session reaches Running with every optional program absent"
    );

    for name in optional_names() {
        assert_eq!(
            supervisor.service_state(&name),
            Some(ServiceState::Failed),
            "{name} is absent, so it is Failed"
        );
        assert_eq!(
            supervisor.running_pid(&name),
            None,
            "{name} has no pid to leak"
        );
        assert_eq!(
            supervisor.restarts(&name),
            Some(0),
            "an absent {name} is not retried into a spawn loop"
        );
    }
    assert_eq!(
        supervisor.service_state("compositor"),
        Some(ServiceState::Running),
        "the anchor runs"
    );

    // The session is stable: more ticks never end it and never resurrect the
    // absent services.
    let before = supervisor.tick();
    assert!(
        !before
            .iter()
            .any(|event| matches!(event, SupervisorEvent::SessionEnded { .. })),
        "an optional absence is never a session-ending event: {before:?}"
    );
    assert_eq!(supervisor.state(), SessionState::Running);
    for name in optional_names() {
        assert_eq!(supervisor.restarts(&name), Some(0));
    }

    supervisor.shutdown();
}

/// The general rule behind the shipped plan: even a *gate* service that cannot
/// be spawned leaves its stage ready, so the next stage still starts. Absence
/// is never a deadlock.
#[test]
fn an_absent_gate_service_does_not_block_the_next_stage() {
    let plan = SessionPlan::new(vec![
        ServiceSpec::new("anchor", "sleep")
            .args(["30"])
            .policy(RestartPolicy::Never)
            .ends_session(true)
            .gate(true),
        ServiceSpec::new("absent-gate", ABSENT).stage(1).gate(true),
        ServiceSpec::new("worker", "sleep")
            .args(["30"])
            .stage(2)
            .policy(RestartPolicy::Always),
    ]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("stage 0 starts");
    assert!(supervisor.set_ready("anchor"), "the anchor's gate opens");

    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.service_state("worker") == Some(ServiceState::Running)
        }),
        "the stage behind the absent gate still starts"
    );
    assert_eq!(
        supervisor.service_state("absent-gate"),
        Some(ServiceState::Failed)
    );
    assert_eq!(supervisor.state(), SessionState::Running);
    supervisor.shutdown();
}

/// Only the anchor's absence ends the session; an optional absence never does.
/// The contrast here is the matrix's boundary: the compositor is load-bearing
/// because it is the Wayland anchor, and everything else is degradable.
#[test]
fn only_the_anchor_absence_ends_the_session() {
    let plan = SessionPlan::new(vec![
        ServiceSpec::new("compositor", ABSENT)
            .policy(RestartPolicy::Never)
            .ends_session(true)
            .gate(true),
        ServiceSpec::new("shell", "sleep")
            .args(["30"])
            .stage(1)
            .policy(RestartPolicy::Always),
    ]);
    let mut supervisor = Supervisor::new(plan);
    supervisor
        .start()
        .expect("start does not block on a missing anchor");
    assert!(
        drive_until(&mut supervisor, Duration::from_secs(5), |s| {
            s.state() == SessionState::Ended
        }),
        "an absent anchor ends the session by design"
    );
    assert_eq!(
        supervisor.service_state("compositor"),
        Some(ServiceState::Failed)
    );
    assert_eq!(
        supervisor.service_state("shell"),
        Some(ServiceState::Pending),
        "the stage after an absent gate never starts once the session is over"
    );
}
