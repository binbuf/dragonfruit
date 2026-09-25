// SPDX-License-Identifier: MIT
//! T-12.1a acceptance: the supervisor starts services in order and restarts
//! a killed one per policy.
//!
//! These are headless: every "service" is a real child process (`sleep` or a
//! short `sh -c`), driven through the same [`Supervisor`] the session binary
//! uses. The kill test is a real `SIGKILL` to a managed child.

use std::process::Command;
use std::time::{Duration, Instant};

use dragonfruit_session::plan::{RestartPolicy, ServiceSpec, SessionPlan};
use dragonfruit_session::supervisor::{ServiceState, SessionState, Supervisor, SupervisorEvent};

/// Run ticks until `predicate` holds or `timeout` elapses.
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

fn sleeping(name: &str, stage: u32) -> ServiceSpec {
    ServiceSpec::new(name, "sleep").args(["30"]).stage(stage)
}

#[test]
fn starts_in_stage_order_and_waits_for_the_gate() {
    let plan = SessionPlan::new(vec![
        sleeping("compositor", 0)
            .policy(RestartPolicy::Never)
            .ends_session(true)
            .gate(true),
        sleeping("shell", 1).policy(RestartPolicy::Always),
        sleeping("settingsd", 1),
    ]);
    let mut supervisor = Supervisor::new(plan);

    supervisor.start().expect("stage 0 starts");
    assert_eq!(
        supervisor.service_state("compositor"),
        Some(ServiceState::Running)
    );
    assert_eq!(
        supervisor.service_state("shell"),
        Some(ServiceState::Pending),
        "the shell waits for the compositor's socket"
    );

    supervisor.tick();
    assert_eq!(
        supervisor.service_state("shell"),
        Some(ServiceState::Pending),
        "an unready gate holds the next stage"
    );
    assert_eq!(supervisor.state(), SessionState::Starting);

    assert!(supervisor.set_ready("compositor"), "the socket appeared");
    supervisor.tick();
    assert_eq!(
        supervisor.service_state("shell"),
        Some(ServiceState::Running)
    );
    assert_eq!(
        supervisor.service_state("settingsd"),
        Some(ServiceState::Running)
    );
    assert_eq!(supervisor.state(), SessionState::Running);
    assert!(supervisor.running_pid("shell").is_some());

    supervisor.shutdown();
}

#[test]
fn a_killed_service_is_restarted_per_policy() {
    let plan = SessionPlan::new(vec![sleeping("worker", 0)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("worker starts");

    let first = supervisor.running_pid("worker").expect("worker has a pid");
    assert!(supervisor.kill("worker"), "SIGKILL the managed worker");

    let restarted = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.restarts("worker") == Some(1) && s.running_pid("worker").is_some()
    });
    assert!(restarted, "on-failure restarts a signalled service");
    let second = supervisor.running_pid("worker").expect("a fresh pid");
    assert_ne!(first, second, "the restart is a new process");
    assert_eq!(supervisor.state(), SessionState::Running);

    supervisor.shutdown();
}

#[test]
fn on_failure_leaves_a_clean_exit_alone() {
    let plan = SessionPlan::new(vec![ServiceSpec::new("oneshot", "sh")
        .args(["-c", "exit 0"])
        .policy(RestartPolicy::OnFailure)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("oneshot starts");

    let exited = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("oneshot") == Some(ServiceState::Exited)
    });
    assert!(exited, "a clean exit is recorded");
    assert_eq!(supervisor.restarts("oneshot"), Some(0));
    assert_eq!(supervisor.running_pid("oneshot"), None);

    supervisor.shutdown();
}

#[test]
fn always_restarts_even_a_clean_exit() {
    let plan = SessionPlan::new(vec![ServiceSpec::new("oneshot", "sh")
        .args(["-c", "exit 0"])
        .policy(RestartPolicy::Always)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("oneshot starts");

    let restarted = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.restarts("oneshot").is_some_and(|count| count >= 2)
    });
    assert!(restarted, "always keeps restarting a clean exit");

    supervisor.shutdown();
}

#[test]
fn never_leaves_a_failure_exited() {
    let plan = SessionPlan::new(vec![ServiceSpec::new("oneshot", "sh")
        .args(["-c", "exit 3"])
        .policy(RestartPolicy::Never)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("oneshot starts");

    let exited = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("oneshot") == Some(ServiceState::Exited)
    });
    assert!(exited, "a failure is recorded");
    assert_eq!(supervisor.restarts("oneshot"), Some(0));

    supervisor.shutdown();
}

#[test]
fn the_anchor_exit_ends_the_session_and_stops_the_rest() {
    let plan = SessionPlan::new(vec![
        sleeping("compositor", 0)
            .policy(RestartPolicy::Never)
            .ends_session(true),
        sleeping("shell", 1).policy(RestartPolicy::Always),
    ]);
    let mut supervisor = Supervisor::new(plan);
    supervisor.start().expect("stage 0 starts");

    // The shell starts once the anchor is spawned.
    let shell_started = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.service_state("shell") == Some(ServiceState::Running)
    });
    assert!(shell_started, "stage 1 starts after the anchor is spawned");

    // A compositor crash ends the session: killing the anchor stops the shell.
    assert!(supervisor.kill("compositor"), "the compositor crashes");
    let ended = drive_until(&mut supervisor, Duration::from_secs(5), |s| {
        s.state() == SessionState::Ended
    });
    assert!(ended, "the compositor's exit ends the session");
    assert_eq!(
        supervisor.service_state("shell"),
        Some(ServiceState::Stopped),
        "the surviving services are torn down"
    );
    assert_eq!(
        supervisor.restarts("compositor"),
        Some(0),
        "the anchor is never restarted"
    );
}

#[test]
fn an_anchor_that_cannot_start_ends_the_session() {
    let plan = SessionPlan::new(vec![ServiceSpec::new(
        "compositor",
        "definitely-not-a-real-program-xyz",
    )
    .policy(RestartPolicy::Never)
    .ends_session(true)]);
    let mut supervisor = Supervisor::new(plan);
    supervisor
        .start()
        .expect("start does not block on a missing anchor");
    let events = supervisor.tick();

    assert_eq!(
        supervisor.service_state("compositor"),
        Some(ServiceState::Failed)
    );
    assert_eq!(supervisor.state(), SessionState::Ended);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, SupervisorEvent::SessionEnded { .. })),
        "the session reports why it ended: {events:?}"
    );
}

#[test]
fn print_plan_lists_the_compositor_first() {
    let output = Command::new(env!("CARGO_BIN_EXE_dragonfruit-session"))
        .arg("--print-plan")
        .output()
        .expect("the session binary runs");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).expect("plan output is UTF-8");

    let first = text.lines().next().expect("a plan has at least one line");
    assert!(first.contains("compositor"), "{text}");
    assert!(first.starts_with("0\t"), "stage 0 comes first: {first}");
    assert!(text.contains("shell"), "{text}");
    assert!(text.contains("on-failure"), "{text}");
    assert!(text.contains("portal"), "{text}");
}
